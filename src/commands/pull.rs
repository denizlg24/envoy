use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

use crate::{
    commands::crypto::decrypt_bytes,
    utils::{
        config::load_token,
        manifest::{load_manifest, read_applied, write_applied},
        project_config::{get_remote_url, load_project_config},
        storage::{download_blob, download_manifest},
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
        println!(
            "{} {}",
            style("✓").green().bold(),
            style("Already up to date").green()
        );
        return Ok(());
    }

    let manifest_blob_path = Path::new(".envoy/cache").join(format!("{}.blob", manifest_hash));

    if !manifest_blob_path.exists() {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(80));
        spinner.set_message("Downloading manifest...");
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

    println!(
        "\n{} Pulling {} files...",
        style("→").cyan().bold(),
        manifest.files.len()
    );

    let pb = ProgressBar::new(manifest.files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓▒░"),
    );

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
        println!(
            "{} {}",
            style("✓").green().bold(),
            style(format!("Downloaded {} blobs", downloaded)).green()
        );
    }

    let mut restored = 0;

    let pb = ProgressBar::new(manifest.files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓▒░"),
    );
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
    println!(
        "{} {}",
        style("✓").green().bold(),
        style(format!("Restored {} files", restored)).green()
    );
    write_applied(&manifest_hash)?;
    println!(
        "{} Updated to {}",
        style("✓").green().bold(),
        style(&manifest_hash[..8]).yellow().bold()
    );
    Ok(())
}
