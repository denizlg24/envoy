use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u8,
    pub files: HashMap<String, String>,
}

impl Default for Manifest {
    fn default() -> Self {
        Self::new()
    }
}

impl Manifest {
    pub fn new() -> Self {
        Self {
            version: 1,
            files: HashMap::new(),
        }
    }
}
use crate::{
    commands::crypto::{decrypt_bytes_with_key, encrypt_bytes_with_key},
    utils::{
        project_config::load_project_config,
        session::{clear_session, derive_manifest_key_from_passphrase, load_session, save_session},
        ui::{print_error, prompt_passphrase},
    },
};
use anyhow::{Result, bail};
use hex;
use sha2::{Digest, Sha256};
use std::fs;

pub fn save_manifest(manifest: &Manifest) -> Result<String> {
    let plaintext = serde_json::to_vec(manifest)
        .map_err(|e| anyhow::anyhow!("Failed to serialize manifest: {}", e))?;

    let manifest_key = get_project_key()?;

    let encrypted = encrypt_bytes_with_key(&plaintext, &manifest_key)?;

    let mut hasher = Sha256::new();
    hasher.update(&encrypted);
    let hash_hex = hex::encode(hasher.finalize());

    let path = format!(".envoy/cache/{}.blob", hash_hex);
    fs::write(&path, encrypted)
        .map_err(|e| anyhow::anyhow!("Failed to write manifest blob: {}", e))?;

    fs::write(".envoy/latest", &hash_hex)
        .map_err(|e| anyhow::anyhow!("Failed to update latest manifest reference: {}", e))?;

    Ok(hash_hex)
}

pub fn load_manifest() -> Result<Manifest> {
    let project = load_project_config()?;
    let manifest_key = get_project_key()?;
    if !std::path::Path::new(".envoy/latest").exists() {
        return Ok(Manifest::new());
    }

    let hash = fs::read_to_string(".envoy/latest")
        .map_err(|e| anyhow::anyhow!("Failed to read manifest reference: {}", e))?;
    let path = format!(".envoy/cache/{}.blob", hash.trim());

    if !std::path::Path::new(&path).exists() {
        return Ok(Manifest::new());
    }

    let encrypted =
        fs::read(&path).map_err(|e| anyhow::anyhow!("Failed to read manifest blob: {}", e))?;

    let plaintext = match decrypt_bytes_with_key(&encrypted, &manifest_key) {
        Ok(plain) => plain,
        Err(_) => {
            clear_session(&project.project_id)?;
            bail!("Failed to decrypt manifest. The passphrase may be incorrect.");
        }
    };

    let manifest: Manifest = serde_json::from_slice(&plaintext)
        .map_err(|e| anyhow::anyhow!("Failed to parse manifest: {}", e))?;

    if manifest.version != 1 {
        bail!(
            "Unsupported manifest version {}. Please update envy.",
            manifest.version
        );
    }

    Ok(manifest)
}

pub fn load_manifest_by_hash(hash: &str) -> Result<Manifest> {
    let project = load_project_config()?;
    let manifest_key = get_project_key()?;

    let path = format!(".envoy/cache/{}.blob", hash.trim());

    if !std::path::Path::new(&path).exists() {
        bail!(
            "Manifest blob {} not found in cache. Run `envy pull` to fetch it.",
            &hash[..12]
        );
    }

    let encrypted = fs::read(&path)
        .map_err(|e| anyhow::anyhow!("Failed to read manifest blob {}: {}", &hash[..12], e))?;

    let plaintext = match decrypt_bytes_with_key(&encrypted, &manifest_key) {
        Ok(plain) => plain,
        Err(_) => {
            clear_session(&project.project_id)?;
            bail!(
                "Failed to decrypt manifest {}. The passphrase may be incorrect.",
                &hash[..12]
            );
        }
    };

    let manifest: Manifest = serde_json::from_slice(&plaintext)
        .map_err(|e| anyhow::anyhow!("Failed to parse manifest {}: {}", &hash[..12], e))?;

    if manifest.version != 1 {
        bail!(
            "Unsupported manifest version {}. Please update envy.",
            manifest.version
        );
    }

    Ok(manifest)
}

const APPLIED_PATH: &str = ".envoy/cache/applied";

pub fn read_applied() -> Option<String> {
    fs::read_to_string(APPLIED_PATH)
        .ok()
        .map(|s| s.trim().to_string())
}

pub fn write_applied(hash: &str) -> anyhow::Result<()> {
    if let Some(parent) = Path::new(APPLIED_PATH).parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(APPLIED_PATH, hash)?;
    Ok(())
}

pub fn set_manifest(manifest_hash: &str) -> anyhow::Result<()> {
    fs::write(".envoy/latest", manifest_hash)?;
    Ok(())
}

pub fn get_current_manifest_hash() -> Option<String> {
    fs::read_to_string(".envoy/latest")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn compute_manifest_content_hash(manifest: &Manifest) -> String {
    let plaintext = serde_json::to_vec(manifest).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&plaintext);
    hex::encode(hasher.finalize())
}

pub fn get_project_key() -> Result<Vec<u8>> {
    use crate::utils::session::take_passphrase_override;

    let project = load_project_config()?;
    let session = load_session(&project.project_id)?;
    let manifest_key = match session {
        Some(ses) => ses.manifest_key,
        None => {
            let passphrase = if let Some(override_pass) = take_passphrase_override() {
                override_pass
            } else {
                match prompt_passphrase("Project passphrase", 6) {
                    Ok(pass) => pass,
                    Err(e) => {
                        print_error(&format!("Failed to read passphrase: {}", e));
                        std::process::exit(1);
                    }
                }
            };
            println!();
            derive_manifest_key_from_passphrase(&passphrase, &project.project_id)?
        }
    };
    save_session(&project.project_id, &manifest_key)?;
    Ok(manifest_key)
}
