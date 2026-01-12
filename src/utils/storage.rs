use std::path::Path;

#[derive(serde::Deserialize)]
struct SignedUrlResponse {
    method: String,
    url: String,
}

#[derive(serde::Deserialize)]
struct HeadResponse {
    head: Option<String>,
}

#[derive(serde::Serialize)]
struct UpdateHeadRequest {
    new_head: String,
    expected_head: Option<String>,
}

pub async fn fetch_remote_head(
    client: &reqwest::Client,
    server: &str,
    token: &str,
    project_id: &str,
) -> anyhow::Result<Option<String>> {
    let res: HeadResponse = client
        .get(format!("{}/projects/{}/head", server, project_id))
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(res.head)
}

pub async fn update_remote_head(
    client: &reqwest::Client,
    server: &str,
    token: &str,
    project_id: &str,
    new_head: &str,
    expected_head: Option<&str>,
) -> anyhow::Result<()> {
    let body = UpdateHeadRequest {
        new_head: new_head.to_string(),
        expected_head: expected_head.map(|s| s.to_string()),
    };

    let response = client
        .put(format!("{}/projects/{}/head", server, project_id))
        .bearer_auth(token)
        .json(&body)
        .send()
        .await?;

    if response.status() == 400 {
        anyhow::bail!("Remote HEAD has changed. Pull first, then push again.");
    }

    response.error_for_status()?;
    Ok(())
}

pub async fn upload_commit(
    client: &reqwest::Client,
    server: &str,
    token: &str,
    project_id: &str,
    commit_hash: &str,
    commit_path: &Path,
) -> anyhow::Result<()> {
    let res: SignedUrlResponse = client
        .post(format!(
            "{}/projects/{}/blobs/{}/upload?type=commit",
            server, project_id, commit_hash
        ))
        .bearer_auth(token)
        .send()
        .await?
        .json::<SignedUrlResponse>()
        .await?;

    if res.method.to_uppercase() != "PUT" {
        anyhow::bail!("Expected PUT method, got {}", res.method);
    }

    let data = tokio::fs::read(commit_path).await?;

    client
        .put(&res.url)
        .body(data)
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

pub async fn download_commit(
    client: &reqwest::Client,
    server: &str,
    token: &str,
    project_id: &str,
    commit_hash: &str,
) -> anyhow::Result<()> {
    let res: SignedUrlResponse = client
        .get(format!(
            "{}/projects/{}/blobs/{}/download?type=commit",
            server, project_id, commit_hash
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

    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let computed = format!("{:x}", hasher.finalize());
    if computed != *commit_hash {
        anyhow::bail!("Hash mismatch for commit {}", commit_hash);
    }

    let path = std::path::Path::new(".envoy/cache/commits").join(format!("{}.blob", commit_hash));
    tokio::fs::create_dir_all(".envoy/cache/commits").await?;
    tokio::fs::write(path, &bytes).await?;

    Ok(())
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

    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let computed = format!("{:x}", hasher.finalize());
    if computed != *hash {
        anyhow::bail!("Hash mismatch for blob {}", hash);
    }

    let path = std::path::Path::new(".envoy/cache").join(format!("{}.blob", hash));

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

    let path = std::path::Path::new(".envoy/cache").join(format!("{}.blob", manifest_hash));

    tokio::fs::create_dir_all(".envoy/cache").await?;
    tokio::fs::write(path, &bytes).await?;

    Ok(())
}
