use std::path::Path;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{ utils::{
    config::load_token, manifest::{load_manifest, save_manifest}, project_config::{get_remote_url, load_project_config}, storage::{upload_blob, upload_manifest}
}};

pub async fn push(passphrase: &str,remote: Option<&str>) -> anyhow::Result<()> {
    let token = load_token()?;
    let project = load_project_config()?;
    let server = get_remote_url(&project, remote)?;

    let manifest = load_manifest(passphrase)?;

    let client = reqwest::Client::new();

    let total = manifest.files.len();
    println!("\n{} Pushing {} files...", style("→").cyan().bold(), total);
    
    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓▒░"),
    );

    let mut uploaded = 0;

    for hash in manifest.files.values() {
        let blob_path = Path::new(".envoy/cache").join(format!("{}.blob", hash));

        if !blob_path.exists() {
            anyhow::bail!("Missing blob {}", hash);
        }

        pb.set_message(format!("Uploading {}...", &hash[..8]));
        upload_blob(
            &client,
            &server,
            &token,
            &project.project_id,
            hash,
            &blob_path,
        )
        .await?;

        uploaded += 1;
        pb.inc(1);
    }
    
    pb.finish_and_clear();

  
    let manifest_hash = save_manifest(&manifest, passphrase)?;


    let manifest_blob_path =
        Path::new(".envoy/cache").join(format!("{}.blob", manifest_hash));

    upload_manifest(
        &client,
        &server,
        &token,
        &project.project_id,
        &manifest_hash,
        &manifest_blob_path,
    )
    .await?;

    println!("{} {} {}", 
        style("✓").green().bold(), 
        style("Uploaded").green(),
        style(format!("{} blobs", uploaded)).cyan()
    );
    println!("{} {}", 
        style("📦").cyan(),
        style(format!("Manifest: {}", &manifest_hash[..12])).dim()
    );

    Ok(())
}