use std::path::Path;

use crate::{
    commands::crypto::decrypt_bytes,
    utils::{
        commit::{
            commit_exists, load_commit, read_head, read_remote_head, write_head, write_remote_head,
        },
        config::load_token,
        manifest::{load_manifest, read_applied, set_manifest, write_applied},
        project_config::{get_remote_url, load_project_config},
        storage::{download_blob, download_commit, download_manifest, fetch_remote_head},
        ui::{
            create_progress_bar, create_spinner, print_error, print_header, print_kv,
            print_success, prompt_input,
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

        for (file_path, hash) in &manifest.files {
            let blob_path = Path::new(".envoy/cache").join(format!("{}.blob", hash));
            let encrypted = tokio::fs::read(&blob_path).await?;
            let file_passphrase =
                match prompt_input(&format!("Enter passphrase to decrypt {}", file_path)) {
                    Ok(pass) => pass,
                    Err(e) => {
                        print_error(&format!("Failed to read passphrase: {}", e));
                        std::process::exit(1);
                    }
                };
            let plaintext = decrypt_bytes(&encrypted, &file_passphrase)?;

            if let Some(parent) = Path::new(file_path).parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            tokio::fs::write(file_path, plaintext).await?;
            restored += 1;
            pb.inc(1);
        }

        pb.finish_and_clear();
        print_success(&format!("Restored {} file(s).", restored));
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

        let pb = create_progress_bar(manifest.files.len() as u64);
        pb.set_message("Restoring files...");

        for (file_path, hash) in &manifest.files {
            let blob_path = Path::new(".envoy/cache").join(format!("{}.blob", hash));
            let encrypted = tokio::fs::read(&blob_path).await?;
            let file_passphrase =
                match prompt_input(&format!("Enter passphrase to decrypt {}", file_path)) {
                    Ok(pass) => pass,
                    Err(e) => {
                        print_error(&format!("Failed to read passphrase: {}", e));
                        std::process::exit(1);
                    }
                };
            let plaintext = decrypt_bytes(&encrypted, &file_passphrase)?;

            if let Some(parent) = Path::new(file_path).parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            tokio::fs::write(file_path, plaintext).await?;
            restored += 1;
            pb.inc(1);
        }

        pb.finish_and_clear();
        print_success(&format!("Restored {} file(s).", restored));
    }

    write_applied(&manifest_hash)?;

    println!();
    print_kv("Manifest", &manifest_hash[..12]);
    print_success(&format!("Updated to manifest {}.", &manifest_hash[..8]));

    Ok(())
}
