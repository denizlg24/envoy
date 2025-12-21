const BLOB_VERSION: u8 = 1;

const VERSION_LEN: usize = 1;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;

const HEADER_LEN: usize = VERSION_LEN + SALT_LEN + NONCE_LEN;

use anyhow::{Ok, Result, bail};
use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::rand_core::{OsRng, RngCore},
};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce, aead::Aead};
use sha2::{Digest, Sha256};
use std::fs;
use zeroize::Zeroize;

use crate::utils::manifest::{load_manifest, save_manifest};

pub fn encrypt_bytes(plaintext: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    let mut pass = passphrase.as_bytes().to_vec();

    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);

    let argon2 = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(19456, 2, 1, Some(32))
            .map_err(|e| anyhow::anyhow!("Failed to create Argon2 params: {}", e))?,
    );

    let mut key = [0u8; 32];
    argon2
        .hash_password_into(&pass, &salt, &mut key)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;

    let cipher = XChaCha20Poly1305::new(&key.into());

    let mut nonce_bytes = [0u8; 24];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| anyhow::anyhow!("Encryption failed {}", e))?;

    let mut output = Vec::with_capacity(HEADER_LEN + ciphertext.len());

    output.push(BLOB_VERSION); // [1] version
    output.extend_from_slice(&salt); // [16] salt
    output.extend_from_slice(&nonce_bytes); // [24] nonce
    output.extend_from_slice(&ciphertext); // [n] ciphertext

    pass.zeroize();
    key.zeroize();

    Ok(output)
}

pub fn decrypt_bytes(encrypted_data: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    let mut pass = passphrase.as_bytes().to_vec();

    if encrypted_data.len() < HEADER_LEN {
        bail!("Invalid encrypted file: too short");
    }

    let version = encrypted_data[0];

    if version != BLOB_VERSION {
        bail!("Unsupported encrypted blob version: {}", version);
    }

    let salt_start = VERSION_LEN;
    let nonce_start = salt_start + SALT_LEN;
    let ciphertext_start = nonce_start + NONCE_LEN;

    let salt = &encrypted_data[salt_start..nonce_start];
    let nonce_bytes = &encrypted_data[nonce_start..ciphertext_start];
    let ciphertext = &encrypted_data[ciphertext_start..];

    let argon2 = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(19456, 2, 1, Some(32))
            .map_err(|e| anyhow::anyhow!("Failed to create Argon2 params: {}", e))?,
    );

    let mut key = [0u8; 32];
    argon2
        .hash_password_into(&pass, salt, &mut key)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;

    let cipher = XChaCha20Poly1305::new(&key.into());
    let nonce = XNonce::from_slice(nonce_bytes);

    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|e| {
        anyhow::anyhow!(
            "Decryption failed (wrong password or corrupted file): {}",
            e
        )
    })?;

    pass.zeroize();
    key.zeroize();

    Ok(plaintext)
}

pub fn encrypt_file(path: &str, passphrase: &str) -> Result<()> {
    let mut manifest = load_manifest(passphrase)?;

    let plaintext = fs::read(path)?;

    let output = encrypt_bytes(&plaintext, passphrase)?;

    let mut hasher = Sha256::new();
    hasher.update(&output);
    let hash = hasher.finalize();

    let hash_hex = hex::encode(hash);

    let filename = format!(".envoy/cache/{}.blob", hash_hex);
    fs::write(&filename, &output)?;
    manifest.files.insert(path.to_string(), hash_hex);
    save_manifest(&manifest, passphrase)?;

    Ok(())
}

pub fn decrypt_files(passphrase: &str) -> Result<()> {
    let manifest = load_manifest(passphrase)?;
    for (filename, blob_hash) in manifest.files {
        let path = format!(".envoy/cache/{}.blob", blob_hash);
        let encrypted = fs::read(&path)?;

        let expected_hash = blob_hash;

        let mut hasher = Sha256::new();
        hasher.update(&encrypted);

        let computed_hash = hex::encode(hasher.finalize());

        if computed_hash != expected_hash {
            bail!("Encrypted blob integrity check failed (hash mismatch)");
        }

        let plaintext = decrypt_bytes(&encrypted, passphrase)?;

        fs::write(&filename, plaintext)?;
    }
    Ok(())
}
