use clap::{Parser, Subcommand};
use dialoguer::Password;

use crate::commands::update::{check_for_update, print_update_notification};
use crate::commands::{auth::logout_command, status::status};
use crate::utils::ui::{print_error, print_success};

pub mod commands;
pub mod utils;

#[derive(Parser)]
#[command(name = "envy")]
#[command(about = "Secure .env storage with client-side encryption")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum RemoteCommand {
    Add { name: String, url: String },
}

#[derive(Subcommand)]
enum MemberCommand {
    Add {
        github: String,
        #[arg(short, long)]
        nickname: String,
    },
    List {},
    Remove {
        user_id: String,
    },
    RemoveAll {},
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
    Member {
        #[command(subcommand)]
        command: MemberCommand,
    },
}

fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    let is_update_command = matches!(cli.command, Commands::Update {});

    match cli.command {
        Commands::Update {} => {
            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async { commands::update::update().await });

            if let Err(e) = result {
                print_error(&format!("Update failed: {}", e));
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
                print_error(&format!("Initialization failed: {}", e));
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
                print_error(&format!("Login failed: {}", e));
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
            print_success("File encrypted successfully");
        }

        Commands::Decrypt {} => {
            utils::initialized::check_initialized()?;
            let passphrase = Password::new().with_prompt("Enter passphrase").interact()?;

            commands::crypto::decrypt_files(&passphrase)?;
            print_success("File decrypted successfully");
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
                print_error(&format!("Push failed: {}", e));
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
                print_error(&format!("Pull failed: {}", e));
                std::process::exit(1);
            }
        }
        Commands::Status {} => {
            utils::initialized::check_initialized()?;
            let passphrase = Password::new().with_prompt("Enter passphrase").interact()?;
            status(&passphrase)?;
        }
        Commands::Member { command } => match command {
            MemberCommand::Add { github, nickname } => {
                utils::initialized::check_initialized()?;
                let username = utils::members::parse_github_username(&github)?;

                let result = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async {
                        let github_id = utils::members::resolve_github_user(&username).await?;
                        commands::member::add_member(github_id, &nickname).await
                    });

                if let Err(e) = result {
                    print_error(&format!("Add member failed: {}", e));
                    std::process::exit(1);
                }
            }
            MemberCommand::List {} => {
                utils::initialized::check_initialized()?;

                let result = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async { commands::member::list_members().await });

                if let Err(e) = result {
                    print_error(&format!("List members failed: {}", e));
                    std::process::exit(1);
                }
            }
            MemberCommand::Remove { user_id } => {
                utils::initialized::check_initialized()?;

                let result = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async { commands::member::remove_member(&user_id).await });

                if let Err(e) = result {
                    print_error(&format!("Remove member failed: {}", e));
                    std::process::exit(1);
                }
            }
            MemberCommand::RemoveAll {} => {
                utils::initialized::check_initialized()?;

                let result = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async { commands::member::remove_all_members().await });

                if let Err(e) = result {
                    print_error(&format!("Remove all members failed: {}", e));
                    std::process::exit(1);
                }
            }
        },
    }

    if !is_update_command {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        if let Some(latest) = rt.block_on(check_for_update()) {
            print_update_notification(&latest);
        }
    }

    Ok(())
}
