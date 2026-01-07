use std::path::Path;

use crate::utils::{
    config::load_token,
    manifest::{load_manifest, save_manifest, write_applied},
    project_config::{get_remote_url, load_project_config},
    storage::{upload_blob, upload_manifest},
    ui::{create_progress_bar, print_header, print_kv, print_success},
};

pub async fn push(remote: Option<&str>) -> anyhow::Result<()> {
    let token = load_token()?;
    let project = load_project_config()?;
    let server = get_remote_url(&project, remote)?;

    let manifest = load_manifest()?;

    let client = reqwest::Client::new();

    let total = manifest.files.len();
    print_header(&format!("Pushing {} files", total));

    let pb = create_progress_bar(total as u64);

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

    let manifest_hash = save_manifest(&manifest)?;

    let manifest_blob_path = Path::new(".envoy/cache").join(format!("{}.blob", manifest_hash));

    upload_manifest(
        &client,
        &server,
        &token,
        &project.project_id,
        &manifest_hash,
        &manifest_blob_path,
    )
    .await?;

    write_applied(&manifest_hash)?;

    print_success(&format!("Uploaded {} blobs", uploaded));
    print_kv("Manifest:", &manifest_hash[..12]);

    Ok(())
}
