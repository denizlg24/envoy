use anyhow::{Ok, Result, bail};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::Deserialize;
use tokio::time::{Duration, Instant, sleep};

use crate::utils::config::{auth_server_url, load_token, logout, save_token};

#[derive(Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    interval: u64,
    expires_in: u64,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum TokenResponse {
    Success {
        #[serde(rename = "apiToken")]
        api_token: String,
    },
    Pending {
        error: String,
    },
}

pub async fn login() -> Result<()> {
    if load_token().is_ok() {
        println!(
            "{} {}",
            style("[i]").cyan().bold(),
            style("Already logged in").cyan()
        );
        return Ok(());
    }

    let client = Client::new();

    let device: DeviceCodeResponse = client
        .post(format!("{}/auth/github/device", auth_server_url()))
        .send()
        .await?
        .json::<DeviceCodeResponse>()
        .await?;

    println!(
        "\n{} {}",
        style(">").cyan().bold(),
        style("GitHub Authentication").bold()
    );
    println!("  {} {}", style(">").cyan(), device.verification_uri);
    println!(
        "  {} {}",
        style("*").cyan(),
        style(&device.user_code).yellow().bold()
    );
    println!();

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    spinner.set_message("Waiting for authorization...");

    let deadline = Instant::now() + Duration::from_secs(device.expires_in);

    loop {
        if Instant::now() >= deadline {
            spinner.finish_and_clear();
            bail!("Authorization timed out");
        }

        sleep(Duration::from_secs(device.interval)).await;

        let response_text = client
            .post(format!("{}/auth/github/token", auth_server_url()))
            .json(&serde_json::json!({
                "device_code": device.device_code
            }))
            .send()
            .await?
            .text()
            .await?;

        let res: TokenResponse = serde_json::from_str(&response_text)?;
        match res {
            TokenResponse::Pending { error } => {
                if error == "slow_down" {
                    sleep(Duration::from_secs(device.interval + 5)).await;
                }
                continue;
            }

            TokenResponse::Success { api_token } => {
                spinner.finish_and_clear();
                println!(
                    "{} {}",
                    style("✓").green().bold(),
                    style("Authentication successful!").green()
                );
                save_token(&api_token)?;
                return Ok(());
            }
        }
    }
}

pub fn logout_command() -> Result<()> {
    logout()?;
    Ok(())
}
