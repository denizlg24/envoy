use std::path::{Path, PathBuf};

pub fn normalize_path(path: &str) -> String {
    let path = path.trim();

    let normalized = path.replace('\\', "/");

    let normalized = normalized
        .strip_prefix("./")
        .or_else(|| normalized.strip_prefix(".\\"))
        .unwrap_or(&normalized)
        .to_string();

    let normalized = normalized.trim_start_matches('/');

    let parts: Vec<&str> = normalized.split('/').filter(|p| !p.is_empty()).collect();

    parts.join("/")
}

pub fn to_native_path(normalized: &str) -> PathBuf {
    Path::new(normalized).to_path_buf()
}

pub fn ensure_parent_exists(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path(".env"), ".env");
        assert_eq!(normalize_path("./.env"), ".env");
        assert_eq!(normalize_path(".\\.env"), ".env");
        assert_eq!(normalize_path("./config/.env"), "config/.env");
        assert_eq!(normalize_path(".\\config\\.env"), "config/.env");
        assert_eq!(normalize_path("config/.env"), "config/.env");
        assert_eq!(normalize_path("config\\.env"), "config/.env");
        assert_eq!(normalize_path("/config/.env"), "config/.env");
        assert_eq!(normalize_path("\\config\\.env"), "config/.env");
        assert_eq!(normalize_path("  .env  "), ".env");
        assert_eq!(normalize_path("./foo//bar/.env"), "foo/bar/.env");
    }
}
