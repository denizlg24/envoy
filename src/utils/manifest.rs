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
    let plaintext = serde_json::to_vec(manifest)?;

    let manifest_key = get_project_key()?;

    let encrypted = encrypt_bytes_with_key(&plaintext, &manifest_key)?;

    let mut hasher = Sha256::new();
    hasher.update(&encrypted);
    let hash_hex = hex::encode(hasher.finalize());

    let path = format!(".envoy/cache/{}.blob", hash_hex);
    fs::write(&path, encrypted)?;

    fs::write(".envoy/latest", &hash_hex)?;

    Ok(hash_hex)
}

pub fn load_manifest() -> Result<Manifest> {
    let project = load_project_config()?;
    let manifest_key = get_project_key()?;
    if !std::path::Path::new(".envoy/latest").exists() {
        return Ok(Manifest::new());
    }

    let hash = fs::read_to_string(".envoy/latest")?;
    let path = format!(".envoy/cache/{}.blob", hash.trim());

    let encrypted = fs::read(&path)?;

    let plaintext = match decrypt_bytes_with_key(&encrypted, &manifest_key) {
        Ok(plain) => plain,
        Err(err) => {
            clear_session(&project.project_id)?;
            bail!(err);
        }
    };

    let manifest: Manifest = serde_json::from_slice(&plaintext)?;

    if manifest.version != 1 {
        bail!("Unsupported manifest version {}", manifest.version);
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

pub fn get_project_key() -> Result<Vec<u8>> {
    use crate::utils::session::take_passphrase_override;

    let project = load_project_config()?;
    let session = load_session(&project.project_id)?;
    let manifest_key = match session {
        Some(ses) => ses.encrypted_manifest_key,
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
