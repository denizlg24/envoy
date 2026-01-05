use std::path::Path;

use crate::{
    commands::crypto::decrypt_bytes,
    utils::{
        config::load_token,
        manifest::{load_manifest, read_applied, write_applied},
        project_config::{get_remote_url, load_project_config},
        storage::{download_blob, download_manifest},
        ui::{create_progress_bar, create_spinner, print_header, print_success},
    },
};

pub async fn pull(passphrase: &str, remote: Option<&str>) -> anyhow::Result<()> {
    let token = load_token()?;
    let project = load_project_config()?;
    let server = get_remote_url(&project, remote)?;

    let client = reqwest::Client::new();

    let manifest_hash = tokio::fs::read_to_string(".envoy/latest")
        .await?
        .trim()
        .to_string();

    if let Some(applied) = read_applied()
        && applied == manifest_hash
    {
        print_success("Already up to date");
        return Ok(());
    }

    let manifest_blob_path = Path::new(".envoy/cache").join(format!("{}.blob", manifest_hash));

    if !manifest_blob_path.exists() {
        let spinner = create_spinner("Downloading manifest...");
        download_manifest(
            &client,
            &server,
            &token,
            &project.project_id,
            &manifest_hash,
        )
        .await?;
        spinner.finish_and_clear();
    }

    let manifest = load_manifest(passphrase)?;

    print_header(&format!("Pulling {} files", manifest.files.len()));

    let pb = create_progress_bar(manifest.files.len() as u64);

    let mut downloaded = 0;

    for hash in manifest.files.values() {
        let path = Path::new(".envoy/cache").join(format!("{}.blob", hash));

        if path.exists() {
            pb.inc(1);
            continue;
        }

        pb.set_message(format!("Downloading {}...", &hash[..8]));
        download_blob(&client, &server, &token, &project.project_id, hash).await?;

        downloaded += 1;
        pb.inc(1);
    }

    pb.finish_and_clear();

    if downloaded > 0 {
        print_success(&format!("Downloaded {} blobs", downloaded));
    }

    let mut restored = 0;

    let pb = create_progress_bar(manifest.files.len() as u64);
    pb.set_message("Restoring files...");

    for (file_path, hash) in &manifest.files {
        let blob_path = Path::new(".envoy/cache").join(format!("{}.blob", hash));
        let encrypted = tokio::fs::read(&blob_path).await?;

        let plaintext = decrypt_bytes(&encrypted, passphrase)?;

        if let Some(parent) = Path::new(file_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(file_path, plaintext).await?;
        restored += 1;
        pb.inc(1);
    }

    pb.finish_and_clear();
    print_success(&format!("Restored {} files", restored));
    write_applied(&manifest_hash)?;
    print_success(&format!("Updated to {}", &manifest_hash[..8]));
    Ok(())
}
