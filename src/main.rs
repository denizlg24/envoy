use clap::{Parser, Subcommand};
use dialoguer::Password;

use crate::commands::{auth::logout_command, status::status};

pub mod commands;
pub mod utils;

#[derive(Parser)]
#[command(name = "envy")]
#[command(about = "Secure .env storage with client-side encryption")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum RemoteCommand {
    Add { name: String, url: String },
}

#[derive(Subcommand)]
enum Commands {
    Encrypt {
        #[arg(short, long, default_value = ".env")]
        input: String,
    },
    Decrypt {},
    Init {
        name: Option<String>,
    },
    Login {},
    Logout {},
    Update {},
    Push {
        remote: Option<String>,
    },
    Pull {
        remote: Option<String>,
    },
    Status {},
    Remote {
        #[command(subcommand)]
        command: RemoteCommand,
    },
}

fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    match cli.command {
        Commands::Update {} => {
            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async { commands::update::update().await });

            if let Err(e) = result {
                eprintln!(
                    "{} {}",
                    console::style("✗").red().bold(),
                    console::style(format!("Update failed: {}", e)).red()
                );
                std::process::exit(1);
            }
        }
        Commands::Init { name } => {
            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async { commands::init::init_project(name).await });

            if let Err(e) = result {
                eprintln!(
                    "{} {}",
                    console::style("✗").red().bold(),
                    console::style(format!("Initialization failed: {}", e)).red()
                );
                std::process::exit(1);
            }
        }
        Commands::Login {} => {
            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async { commands::auth::login().await });

            if let Err(e) = result {
                eprintln!(
                    "{} {}",
                    console::style("✗").red().bold(),
                    console::style(format!("Login failed: {}", e)).red()
                );
                std::process::exit(1);
            }
        }
        Commands::Logout {} => {
            logout_command()?;
        }
        Commands::Remote { command } => match command {
            RemoteCommand::Add { name, url } => {
                commands::remote::add_remote(&name, &url)?;
            }
        },
        Commands::Encrypt { input } => {
            utils::initialized::check_initialized()?;
            let passphrase = Password::new()
                .with_prompt("Enter passphrase")
                .with_confirmation("Confirm passphrase", "Passphrases do not match")
                .interact()?;

            commands::crypto::encrypt_file(&input, &passphrase)?;
            println!(
                "{} {}",
                console::style("✓").green().bold(),
                console::style("File encrypted successfully").green()
            );
        }

        Commands::Decrypt {} => {
            utils::initialized::check_initialized()?;
            let passphrase = Password::new().with_prompt("Enter passphrase").interact()?;

            commands::crypto::decrypt_files(&passphrase)?;
            println!(
                "{} {}",
                console::style("✓").green().bold(),
                console::style("File decrypted successfully").green()
            );
        }

        Commands::Push { remote } => {
            utils::initialized::check_initialized()?;
            let passphrase = Password::new().with_prompt("Enter passphrase").interact()?;

            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async { commands::push::push(&passphrase, remote.as_deref()).await });

            if let Err(e) = result {
                eprintln!(
                    "{} {}",
                    console::style("✗").red().bold(),
                    console::style(format!("Push failed: {}", e)).red()
                );
                std::process::exit(1);
            }
        }

        Commands::Pull { remote } => {
            utils::initialized::check_initialized()?;
            let passphrase = Password::new().with_prompt("Enter passphrase").interact()?;

            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async { commands::pull::pull(&passphrase, remote.as_deref()).await });

            if let Err(e) = result {
                eprintln!(
                    "{} {}",
                    console::style("✗").red().bold(),
                    console::style(format!("Pull failed: {}", e)).red()
                );
                std::process::exit(1);
            }
        }
        Commands::Status {} => {
            utils::initialized::check_initialized()?;
            let passphrase = Password::new().with_prompt("Enter passphrase").interact()?;
            status(&passphrase)?;
        }
    }

    Ok(())
}
