use anyhow::{Result, bail};
use std::fs;

use crate::utils::project_config::load_project_config;

pub fn add_remote(name: &str, url: &str) -> Result<()> {
    let _project = load_project_config()?;

    let path = ".envoy/config.toml";
    let contents = fs::read_to_string(path)?;
    let mut value: toml::Value = toml::from_str(&contents)?;

    let remotes = value
        .get_mut("remotes")
        .and_then(|v| v.as_table_mut())
        .ok_or_else(|| anyhow::anyhow!("Missing [remotes] section"))?;

    if remotes.contains_key(name) {
        bail!("Remote '{}' already exists", name);
    }

    remotes.insert(name.to_string(), toml::Value::String(url.to_string()));

    fs::write(path, toml::to_string_pretty(&value)?)?;

    println!(
        "{} {} {} {}",
        console::style("✓").green().bold(),
        console::style("Added remote").green(),
        console::style(format!("'{}'", name)).cyan().bold(),
        console::style(format!("> {}", url)).dim()
    );
    Ok(())
}
