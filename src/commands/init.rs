use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::fs;
use std::path::Path;

use crate::{
    commands::auth::login,
    utils::config::{auth_server_url, load_token},
};

#[derive(serde::Deserialize)]
struct CreateProjectResponse {
    #[serde(rename = "projectId")]
    project_id: String,
}

pub async fn ensure_logged_in() -> anyhow::Result<String> {
    match load_token() {
        Ok(token) => Ok(token),
        Err(_) => {
            login().await?;
            load_token()
        }
    }
}

use std::fs::OpenOptions;
use std::io::Write;

pub fn ensure_gitignore() -> anyhow::Result<()> {
    let path = Path::new(".gitignore");

    let envoy_block = "# Envoy\n.envoy/cache/\n.envoy/*.blob\n";

    let existing = if path.exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };

    if existing.contains(".envoy/cache/") {
        return Ok(());
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;

    if !existing.ends_with('\n') && !existing.is_empty() {
        writeln!(file)?;
    }

    writeln!(file, "{}", envoy_block)?;

    Ok(())
}

pub async fn init_project(name: Option<String>) -> anyhow::Result<()> {
    let api_token = ensure_logged_in().await?;

    let root = Path::new(".envoy");

    if root.exists() {
        println!(
            "{} {}",
            style("ℹ").cyan(),
            style("Envoy project already initialized.").cyan()
        );
        return Ok(());
    }

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Creating project...");

    let client = Client::new();
    let res: CreateProjectResponse = client
        .post(format!("{}/projects", auth_server_url()))
        .bearer_auth(&api_token)
        .send()
        .await?
        .json::<CreateProjectResponse>()
        .await?;

    let project_id = res.project_id;

    fs::create_dir(root)?;
    fs::create_dir(root.join("cache"))?;

    let project_name = name.unwrap_or_else(|| "My Envoy Project".to_string());
    let server_url = auth_server_url().to_owned();
    let config = format!(
        r#"
version = 1
project_id = "{}"
name = "{}"

default_remote = "origin"

[remotes]
origin = "{}"


"#,
        project_id, project_name, server_url
    );

    fs::write(root.join("config.toml"), config)?;
    ensure_gitignore()?;
    println!("Envoy project initialized");
    println!("Project ID: {}", project_id);

    Ok(())
}
