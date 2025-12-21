use anyhow::Result;
use std::fs;

pub fn save_token(token: &str) -> Result<()> {
    let mut dir =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

    dir.push(".envoy");

    fs::create_dir_all(&dir)?;

    let mut file = dir.clone();
    file.push("config.toml");

    let contents = format!("api_token = \"{}\"\n", token);

    fs::write(&file, contents)?;

    // Silently save token - success message shown by caller

    Ok(())
}

use anyhow::bail;

pub fn load_token() -> Result<String> {
    let mut path =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

    path.push(".envoy");
    path.push("config.toml");

    let contents = fs::read_to_string(&path).map_err(|_| anyhow::anyhow!("Not logged in"))?;

    for line in contents.lines() {
        if let Some(value) = line.strip_prefix("api_token = ") {
            return Ok(value.trim().trim_matches('"').to_string());
        }
    }

    bail!("api_token not found in config")
}

pub fn logout() -> Result<()> {
    let mut path =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

    path.push(".envoy");
    path.push("config.toml");

    if path.exists() {
        fs::remove_file(&path)?;
        println!(
            "{} {}",
            console::style("✓").green().bold(),
            console::style("Logged out of Envoy.").green()
        );
    } else {
        println!(
            "{} {}",
            console::style("ℹ").cyan(),
            console::style("Already logged out.").cyan()
        );
    }

    Ok(())
}

pub fn auth_server_url() -> String {
    "https://envoy-server.fly.dev".to_string()
}
