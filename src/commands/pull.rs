use std::path::Path;

use crate::{
    commands::crypto::decrypt_bytes,
    utils::{
        commit::{
            commit_exists, load_commit, read_head, read_remote_head, write_head, write_remote_head,
        },
        config::load_token,
        manifest::{load_manifest, read_applied, set_manifest, write_applied},
        paths::{ensure_parent_exists, normalize_path, to_native_path},
        project_config::{get_remote_url, load_project_config},
        storage::{download_blob, download_commit, download_manifest, fetch_remote_head},
        ui::{
            PassphraseResult, create_progress_bar, create_spinner, print_header, print_info,
            print_kv, print_success, print_warn, prompt_file_passphrase,
        },
    },
};

pub async fn pull(remote: Option<&str>) -> anyhow::Result<()> {
    let token = load_token()?;
    let project = load_project_config()?;
    let server = get_remote_url(&project, remote)?;

    let client = reqwest::Client::new();

    let remote_head_result =
        fetch_remote_head(&client, &server, &token, &project.project_id).await?;

    if let Some(ref remote_head) = remote_head_result {
        return pull_with_commits(&client, &server, &token, &project.project_id, remote_head).await;
    }

    // Fall back to legacy manifest-based pull
    legacy_pull(&client, &server, &token, &project.project_id).await
}

async fn pull_with_commits(
    client: &reqwest::Client,
    server: &str,
    token: &str,
    project_id: &str,
    remote_head: &str,
) -> anyhow::Result<()> {
    let local_remote_head = read_remote_head();

    if local_remote_head.as_ref() == Some(&remote_head.to_string()) {
        let local_head = read_head();
        if local_head.as_ref() == Some(&remote_head.to_string()) {
            print_success("Already up to date.");
            return Ok(());
        }
    }

    print_header("Fetching commits");

    let mut commits_to_download = Vec::new();
    let mut current_hash = Some(remote_head.to_string());

    while let Some(hash) = current_hash {
        if commit_exists(&hash) {
            break; // We have this commit and all ancestors
        }
        commits_to_download.push(hash.clone());

        let spinner = create_spinner(&format!("Fetching commit {}...", &hash[..8]));
        download_commit(client, server, token, project_id, &hash).await?;
        spinner.finish_and_clear();

        let commit = load_commit(&hash)?;
        current_hash = commit.parent;
    }

    if commits_to_download.is_empty() {
        print_success("Already up to date.");
        return Ok(());
    }

    print_success(&format!("Fetched {} commit(s).", commits_to_download.len()));

    let latest_commit = load_commit(remote_head)?;
    let manifest_hash = &latest_commit.manifest_hash;

    let manifest_blob_path = Path::new(".envoy/cache").join(format!("{}.blob", manifest_hash));

    if !manifest_blob_path.exists() {
        let spinner = create_spinner("Downloading manifest...");
        download_manifest(client, server, token, project_id, manifest_hash).await?;
        spinner.finish_and_clear();
    }

    set_manifest(manifest_hash)?;

    let manifest = load_manifest()?;
    if !manifest.files.is_empty() {
        print_header(&format!("Pulling {} file(s)", manifest.files.len()));

        let pb = create_progress_bar(manifest.files.len() as u64);
        let mut downloaded = 0;

        for hash in manifest.files.values() {
            let path = Path::new(".envoy/cache").join(format!("{}.blob", hash));

            if path.exists() {
                pb.inc(1);
                continue;
            }

            pb.set_message(format!("Downloading {}...", &hash[..8]));
            download_blob(client, server, token, project_id, hash).await?;

            downloaded += 1;
            pb.inc(1);
        }

        pb.finish_and_clear();

        if downloaded > 0 {
            print_success(&format!("Downloaded {} file(s).", downloaded));
        }

        let pb = create_progress_bar(manifest.files.len() as u64);
        pb.set_message("Restoring files...");
        let mut restored = 0;
        let mut skipped = 0;

        for (file_path, hash) in &manifest.files {
            let blob_path = Path::new(".envoy/cache").join(format!("{}.blob", hash));
            let encrypted = tokio::fs::read(&blob_path)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to read blob for '{}': {}", file_path, e))?;

            pb.suspend(|| {
                println!();
            });

            let passphrase = match prompt_file_passphrase(file_path) {
                Ok(PassphraseResult::Passphrase(pass)) => pass,
                Ok(PassphraseResult::Skip) => {
                    pb.suspend(|| {
                        print_info(&format!("Skipping '{}'", file_path));
                    });
                    skipped += 1;
                    pb.inc(1);
                    continue;
                }
                Err(e) => {
                    pb.suspend(|| {
                        print_warn(&format!(
                            "Failed to read passphrase for '{}': {}",
                            file_path, e
                        ));
                    });
                    skipped += 1;
                    pb.inc(1);
                    continue;
                }
            };

            match decrypt_bytes(&encrypted, &passphrase) {
                Ok(plaintext) => {
                    let normalized = normalize_path(file_path);
                    let target_path = to_native_path(&normalized);

                    if let Err(e) = ensure_parent_exists(&target_path) {
                        pb.suspend(|| {
                            print_warn(&format!(
                                "Failed to create directory for '{}': {}",
                                file_path, e
                            ));
                        });
                        skipped += 1;
                        pb.inc(1);
                        continue;
                    }

                    if let Err(e) = tokio::fs::write(&target_path, plaintext).await {
                        pb.suspend(|| {
                            print_warn(&format!(
                                "Failed to write '{}': {}",
                                target_path.display(),
                                e
                            ));
                        });
                        skipped += 1;
                        pb.inc(1);
                        continue;
                    }

                    restored += 1;
                }
                Err(_) => {
                    pb.suspend(|| {
                        print_warn(&format!("Wrong passphrase for '{}', skipping", file_path));
                    });
                    skipped += 1;
                }
            }

            pb.inc(1);
        }

        pb.finish_and_clear();

        if restored > 0 {
            print_success(&format!("Restored {} file(s).", restored));
        }
        if skipped > 0 {
            print_info(&format!("Skipped {} file(s).", skipped));
        }
    }

    write_head(remote_head)?;
    write_remote_head(remote_head)?;
    write_applied(manifest_hash)?;

    println!();
    print_kv("HEAD", &remote_head[..12]);
    print_success(&format!("Updated to commit {}.", &remote_head[..8]));

    Ok(())
}

/// Legacy pull for backwards compatibility
async fn legacy_pull(
    client: &reqwest::Client,
    server: &str,
    token: &str,
    project_id: &str,
) -> anyhow::Result<()> {
    let manifest_hash = tokio::fs::read_to_string(".envoy/latest")
        .await?
        .trim()
        .to_string();

    if let Some(applied) = read_applied()
        && applied == manifest_hash
    {
        print_success("Already up to date.");
        return Ok(());
    }

    let manifest_blob_path = Path::new(".envoy/cache").join(format!("{}.blob", manifest_hash));

    if !manifest_blob_path.exists() {
        let spinner = create_spinner("Downloading manifest...");
        download_manifest(client, server, token, project_id, &manifest_hash).await?;
        spinner.finish_and_clear();
    }

    let manifest = load_manifest()?;
    if !manifest.files.is_empty() {
        print_header(&format!("Pulling {} file(s)", manifest.files.len()));

        let pb = create_progress_bar(manifest.files.len() as u64);

        let mut downloaded = 0;

        for hash in manifest.files.values() {
            let path = Path::new(".envoy/cache").join(format!("{}.blob", hash));

            if path.exists() {
                pb.inc(1);
                continue;
            }

            pb.set_message(format!("Downloading {}...", &hash[..8]));
            download_blob(client, server, token, project_id, hash).await?;

            downloaded += 1;
            pb.inc(1);
        }

        pb.finish_and_clear();

        if downloaded > 0 {
            print_success(&format!("Downloaded {} file(s).", downloaded));
        }

        let mut restored = 0;
        let mut skipped = 0;

        let pb = create_progress_bar(manifest.files.len() as u64);
        pb.set_message("Restoring files...");

        for (file_path, hash) in &manifest.files {
            let blob_path = Path::new(".envoy/cache").join(format!("{}.blob", hash));
            let encrypted = tokio::fs::read(&blob_path)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to read blob for '{}': {}", file_path, e))?;

            pb.suspend(|| {
                println!();
            });

            let passphrase = match prompt_file_passphrase(file_path) {
                Ok(PassphraseResult::Passphrase(pass)) => pass,
                Ok(PassphraseResult::Skip) => {
                    pb.suspend(|| {
                        print_info(&format!("Skipping '{}'", file_path));
                    });
                    skipped += 1;
                    pb.inc(1);
                    continue;
                }
                Err(e) => {
                    pb.suspend(|| {
                        print_warn(&format!(
                            "Failed to read passphrase for '{}': {}",
                            file_path, e
                        ));
                    });
                    skipped += 1;
                    pb.inc(1);
                    continue;
                }
            };

            match decrypt_bytes(&encrypted, &passphrase) {
                Ok(plaintext) => {
                    let normalized = normalize_path(file_path);
                    let target_path = to_native_path(&normalized);

                    if let Err(e) = ensure_parent_exists(&target_path) {
                        pb.suspend(|| {
                            print_warn(&format!(
                                "Failed to create directory for '{}': {}",
                                file_path, e
                            ));
                        });
                        skipped += 1;
                        pb.inc(1);
                        continue;
                    }

                    if let Err(e) = tokio::fs::write(&target_path, plaintext).await {
                        pb.suspend(|| {
                            print_warn(&format!(
                                "Failed to write '{}': {}",
                                target_path.display(),
                                e
                            ));
                        });
                        skipped += 1;
                        pb.inc(1);
                        continue;
                    }

                    restored += 1;
                }
                Err(_) => {
                    pb.suspend(|| {
                        print_warn(&format!("Wrong passphrase for '{}', skipping", file_path));
                    });
                    skipped += 1;
                }
            }

            pb.inc(1);
        }

        pb.finish_and_clear();

        if restored > 0 {
            print_success(&format!("Restored {} file(s).", restored));
        }
        if skipped > 0 {
            print_info(&format!("Skipped {} file(s).", skipped));
        }
    }

    write_applied(&manifest_hash)?;

    println!();
    print_kv("Manifest", &manifest_hash[..12]);
    print_success(&format!("Updated to manifest {}.", &manifest_hash[..8]));

    Ok(())
}
