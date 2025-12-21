use std::path::Path;

#[derive(serde::Deserialize)]
struct SignedUrlResponse {
    method: String, 
    url: String,
}

pub async fn upload_blob(
    client: &reqwest::Client,
    server: &str,
    token: &str,
    project_id: &str,
    hash: &str,
    blob_path: &std::path::Path,
) -> anyhow::Result<()> {
    let res: SignedUrlResponse = client
        .post(format!(
            "{}/projects/{}/blobs/{}/upload",
            server, project_id, hash
        ))
        .bearer_auth(token)
        .send()
        .await?
        .json::<SignedUrlResponse>()
        .await?;

    if res.method.to_uppercase() != "PUT" {
        anyhow::bail!("Expected PUT method, got {}", res.method);
    }

    let data = tokio::fs::read(blob_path).await?;

    client
        .put(&res.url)
        .body(data)
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

pub async fn download_blob(
    client: &reqwest::Client,
    server: &str,
    token: &str,
    project_id: &str,
    hash: &str,
) -> anyhow::Result<()> {
    let res: SignedUrlResponse = client
        .get(format!(
            "{}/projects/{}/blobs/{}/download",
            server, project_id, hash
        ))
        .bearer_auth(token)
        .send()
        .await?
        .json::<SignedUrlResponse>()
        .await?;

    let bytes = client
        .get(&res.url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let computed = format!("{:x}", hasher.finalize());
    if computed != *hash {
        anyhow::bail!("Hash mismatch for blob {}", hash);
    }

    let path = std::path::Path::new(".envoy/cache")
        .join(format!("{}.blob", hash));

    tokio::fs::write(path, &bytes).await?;

    Ok(())
}

pub async fn upload_manifest(
    client: &reqwest::Client,
    server: &str,
    token: &str,
    project_id: &str,
    manifest_hash: &str,
    manifest_path: &Path,
) -> anyhow::Result<()> {
    let res: SignedUrlResponse = client
        .post(format!(
            "{}/projects/{}/blobs/{}/upload?type=manifest",
            server, project_id, manifest_hash
        ))
        .bearer_auth(token)
        .send()
        .await?
        .json::<SignedUrlResponse>()
        .await?;

    let bytes = tokio::fs::read(manifest_path).await?;

    client
        .put(&res.url)
        .body(bytes)
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

pub async fn download_manifest(
    client: &reqwest::Client,
    server: &str,
    token: &str,
    project_id: &str,
    manifest_hash: &str,
) -> anyhow::Result<()> {
    let res: SignedUrlResponse = client
        .get(format!(
            "{}/projects/{}/blobs/{}/download?type=manifest",
            server, project_id, manifest_hash
        ))
        .bearer_auth(token)
        .send()
        .await?
        .json::<SignedUrlResponse>()
        .await?;

    let bytes = client
        .get(&res.url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    let path = std::path::Path::new(".envoy/cache")
        .join(format!("{}.blob", manifest_hash));

    tokio::fs::create_dir_all(".envoy/cache").await?;
    tokio::fs::write(path, &bytes).await?;

    Ok(())
}
