use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;

use crate::utils::{
    config::{auth_server_url, load_token},
    project_config::load_project_config,
};

#[derive(Deserialize)]
struct ProjectMemberResponse {
    #[serde(rename = "projectMember")]
    project_member: ProjectMember,
}

#[derive(Deserialize)]
struct ProjectMember {
    #[serde(rename = "userId")]
    user_id: String,
    role: String,
    #[serde(rename = "projectId")]
    project_id: String,
    nickname: Option<String>,
}

#[derive(Deserialize)]
struct ListMembersResponse {
    members: Vec<ProjectMember>,
}

#[derive(Deserialize)]
struct RemoveMemberResponse {
    #[allow(dead_code)]
    success: bool,
    #[serde(rename = "deletedMember")]
    deleted_member: ProjectMember,
}

#[derive(Deserialize)]
struct RemoveAllMembersResponse {
    #[allow(dead_code)]
    success: bool,
    #[serde(rename = "deletedCount")]
    deleted_count: u32,
}

pub async fn add_member(github_id: u64, nickname: &str) -> anyhow::Result<()> {
    let token = load_token()?;
    let project = load_project_config()?;
    let client = reqwest::Client::new();

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    spinner.set_message(format!("Adding member '{}'...", nickname));

    let response = client
        .post(format!(
            "{}/projects/{}/members",
            auth_server_url(),
            project.project_id
        ))
        .bearer_auth(token)
        .json(&serde_json::json!({
            "githubId": github_id.to_string(),
            "nickname": nickname
        }))
        .send()
        .await?
        .error_for_status()?
        .json::<ProjectMemberResponse>()
        .await?;

    spinner.finish_and_clear();

    let member = response.project_member;

    println!(
        "{} {}",
        style("✓").green().bold(),
        style("Member added successfully!").green()
    );
    if let Some(nick) = &member.nickname {
        println!(
            "  {} Nickname: {}",
            style("@").cyan(),
            style(nick).cyan().bold()
        );
    }
    println!(
        "  {} User ID: {}",
        style(">").cyan(),
        style(&member.user_id).dim()
    );
    println!(
        "  {} Role: {}",
        style("*").cyan(),
        style(&member.role).yellow()
    );
    println!(
        "  {} Project: {}",
        style("📦").cyan(),
        style(&member.project_id).dim()
    );
    Ok(())
}

pub async fn list_members() -> anyhow::Result<()> {
    let token = load_token()?;
    let project = load_project_config()?;
    let client = reqwest::Client::new();

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    spinner.set_message("Fetching project members...");

    let response = client
        .get(format!(
            "{}/projects/{}/members",
            auth_server_url(),
            project.project_id
        ))
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?
        .json::<ListMembersResponse>()
        .await?;

    spinner.finish_and_clear();

    let members = response.members;

    if members.is_empty() {
        println!(
            "{} {}",
            style("[i]").blue().bold(),
            style("No members found in this project").dim()
        );
        return Ok(());
    }

    println!(
        "\n{} {} {}",
        style(">").cyan().bold(),
        style("Project Members").bold(),
        style(format!("({})", members.len())).dim()
    );
    println!();

    for member in members {
        if let Some(nickname) = &member.nickname {
            println!(
                "  {} {}",
                style("@").cyan(),
                style(nickname).cyan().bold()
            );
        }
        println!(
            "    {} {}",
            style(">").dim(),
            style(&member.user_id).white()
        );
        println!(
            "    {} {}",
            style("*").dim(),
            style(&member.role).yellow()
        );
        println!();
    }

    Ok(())
}

pub async fn remove_member(user_id: &str) -> anyhow::Result<()> {
    let token = load_token()?;
    let project = load_project_config()?;
    let client = reqwest::Client::new();

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    spinner.set_message(format!("Removing member {}...", user_id));

    let response = client
        .delete(format!(
            "{}/projects/{}/members/{}",
            auth_server_url(),
            project.project_id,
            user_id
        ))
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?
        .json::<RemoveMemberResponse>()
        .await?;

    spinner.finish_and_clear();

    let deleted = response.deleted_member;

    println!(
        "{} {}",
        style("[-]").red(),
        style("Member removed successfully!").white()
    );
    println!(
        "  {} User ID: {}",
        style(">").cyan(),
        style(&deleted.user_id).cyan()
    );
    println!(
        "  {} Role: {}",
        style("*").cyan(),
        style(&deleted.role).dim()
    );
    Ok(())
}

pub async fn remove_all_members() -> anyhow::Result<()> {
    let token = load_token()?;
    let project = load_project_config()?;
    let client = reqwest::Client::new();

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    spinner.set_message("Removing all members...");

    let response = client
        .delete(format!(
            "{}/projects/{}/members",
            auth_server_url(),
            project.project_id
        ))
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?
        .json::<RemoveAllMembersResponse>()
        .await?;

    spinner.finish_and_clear();

    println!(
        "{} Removed {} member(s)",
        style("[-]").red(),
        style(response.deleted_count.to_string()).yellow().bold()
    );

    Ok(())
}
