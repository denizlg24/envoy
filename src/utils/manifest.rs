use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u8,
    pub files: HashMap<String, String>,
}

impl Manifest {
    pub fn new() -> Self {
        Self {
            version: 1,
            files: HashMap::new(),
        }
    }
}
use crate::commands;
use anyhow::{Result, bail};
use commands::crypto::encrypt_bytes;
use hex;
use sha2::{Digest, Sha256};
use std::fs;

pub fn save_manifest(manifest: &Manifest, passphrase: &str) -> Result<String> {
    let plaintext = serde_json::to_vec(manifest)?;

    let encrypted = encrypt_bytes(&plaintext, passphrase)?;

    let mut hasher = Sha256::new();
    hasher.update(&encrypted);
    let hash_hex = hex::encode(hasher.finalize());

    let path = format!(".envoy/cache/{}.blob", hash_hex);
    fs::write(&path, encrypted)?;

    fs::write(".envoy/latest", &hash_hex)?;

    Ok(hash_hex)
}

use commands::crypto::decrypt_bytes;

pub fn load_manifest(passphrase: &str) -> Result<Manifest> {
    if !std::path::Path::new(".envoy/latest").exists() {
        return Ok(Manifest::new());
    }

    let hash = fs::read_to_string(".envoy/latest")?;
    let path = format!(".envoy/cache/{}.blob", hash.trim());

    let encrypted = fs::read(&path)?;

    let plaintext = decrypt_bytes(&encrypted, passphrase)?;

    let manifest: Manifest = serde_json::from_slice(&plaintext)?;

    if manifest.version != 1 {
        bail!("Unsupported manifest version {}", manifest.version);
    }

    Ok(manifest)
}

