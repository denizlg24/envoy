use anyhow::bail;
use argon2::password_hash::rand_core::RngCore;
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use once_cell::sync::OnceCell;

static SESSION_KEY: OnceCell<[u8; 32]> = OnceCell::new();

fn session_key_path() -> PathBuf {
    let mut path = dirs::home_dir().expect("home dir");
    path.push(".envoy");
    path.push(".session_key");
    path
}

fn get_or_init_session_key() -> &'static [u8; 32] {
    SESSION_KEY.get_or_init(|| {
        let path = session_key_path();

        if path.exists()
            && let Ok(data) = fs::read(&path)
            && data.len() == 32
        {
            let mut key = [0u8; 32];
            key.copy_from_slice(&data);
            return key;
        }

        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);

        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let _ = fs::write(&path, key);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
        }

        key
    })
}

const SESSION_TTL_SECS: u64 = 15 * 60;

#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
    pub project_id: String,
    pub encrypted_manifest_key: Vec<u8>,
    pub expires_at: u64,
}

#[derive(Serialize, Deserialize, Default)]
struct SessionStore {
    sessions: HashMap<String, Session>,
}

fn session_path() -> PathBuf {
    let mut path = dirs::home_dir().expect("home dir");
    path.push(".envoy");
    path.push("sessions.json");
    path
}

fn load_store() -> anyhow::Result<SessionStore> {
    let path = session_path();
    if !path.exists() {
        return Ok(SessionStore::default());
    }

    let data = fs::read(&path)?;
    let store: SessionStore = serde_json::from_slice(&data)?;
    Ok(store)
}

fn save_store(store: &SessionStore) -> anyhow::Result<()> {
    let path = session_path();
    fs::create_dir_all(path.parent().unwrap())?;

    fs::write(&path, serde_json::to_vec_pretty(&store)?)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

pub fn clear_session(project_id: &str) -> anyhow::Result<()> {
    let mut store = load_store()?;
    store.sessions.remove(project_id);
    save_store(&store)?;
    Ok(())
}

pub fn load_session(project_id: &str) -> anyhow::Result<Option<Session>> {
    let store = load_store()?;

    if let Some(session) = store.sessions.get(project_id) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        if session.expires_at < now {
            let _ = clear_session(project_id);
            return Ok(None);
        }

        if session.project_id != project_id || session.expires_at < now {
            clear_session(&session.project_id)?;
            return Ok(None);
        }

        let key = decrypt_manifest_key(
            &session.encrypted_manifest_key,
            &session.project_id,
            session.expires_at,
        )?;

        let final_session = Session {
            project_id: project_id.to_string(),
            encrypted_manifest_key: key,
            expires_at: session.expires_at,
        };

        Ok(Some(final_session.clone()))
    } else {
        Ok(None)
    }
}

pub fn save_session(project_id: &str, encrypted_manifest_key: &[u8]) -> anyhow::Result<()> {
    let mut store = load_store()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let expires_at = now + SESSION_TTL_SECS;

    let encrypted_key = encrypt_manifest_key(encrypted_manifest_key, project_id, expires_at)?;

    let session = Session {
        project_id: project_id.to_string(),
        encrypted_manifest_key: encrypted_key,
        expires_at,
    };

    store.sessions.insert(project_id.to_string(), session);
    save_store(&store)?;

    Ok(())
}

use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, Payload},
};

const SESSION_BLOB_VERSION: u8 = 1;
const SESSION_VERSION_LEN: usize = 1;
const SESSION_NONCE_LEN: usize = 24;
const SESSION_HEADER_LEN: usize = SESSION_VERSION_LEN + SESSION_NONCE_LEN;

fn encrypt_manifest_key(
    manifest_key: &[u8],
    project_id: &str,
    expires_at: u64,
) -> anyhow::Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(get_or_init_session_key().into());

    let mut nonce = [0u8; SESSION_NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);

    let aad = format!("{}:{}", project_id, expires_at);

    let ciphertext = match cipher.encrypt(
        XNonce::from_slice(&nonce),
        Payload {
            msg: manifest_key,
            aad: aad.as_bytes(),
        },
    ) {
        Ok(cipher) => cipher,
        Err(error) => bail!(error),
    };

    let mut out = Vec::with_capacity(SESSION_HEADER_LEN + ciphertext.len());
    out.push(SESSION_BLOB_VERSION);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);

    Ok(out)
}

fn decrypt_manifest_key(
    encrypted: &[u8],
    project_id: &str,
    expires_at: u64,
) -> anyhow::Result<Vec<u8>> {
    if encrypted.len() < SESSION_HEADER_LEN {
        anyhow::bail!("Invalid session blob: too short");
    }

    let version = encrypted[0];
    if version != SESSION_BLOB_VERSION {
        anyhow::bail!("Unsupported session blob version: {}", version);
    }

    let nonce_start = SESSION_VERSION_LEN;
    let ciphertext_start = nonce_start + SESSION_NONCE_LEN;

    let nonce = &encrypted[nonce_start..ciphertext_start];
    let ciphertext = &encrypted[ciphertext_start..];

    let cipher = XChaCha20Poly1305::new(get_or_init_session_key().into());

    let aad = format!("{}:{}", project_id, expires_at);

    let plaintext = match cipher.decrypt(
        XNonce::from_slice(nonce),
        Payload {
            msg: ciphertext,
            aad: aad.as_bytes(),
        },
    ) {
        Ok(plain) => plain,
        Err(error) => bail!(error),
    };

    Ok(plaintext)
}

pub fn derive_manifest_key_from_passphrase(
    passphrase: &str,
    project_id: &str,
) -> anyhow::Result<Vec<u8>> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(project_id.as_bytes());
    let hash = hasher.finalize();
    let salt: [u8; 16] = hash[..16].try_into().unwrap();

    let argon2 = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(19456, 2, 1, Some(32))
            .map_err(|e| anyhow::anyhow!("Failed to create Argon2 params: {}", e))?,
    );
    let pass = passphrase.as_bytes();
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(pass, &salt, &mut key)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;
    Ok(key.to_vec())
}
