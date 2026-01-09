use std::fs::remove_dir_all;
use std::path::Path;

use clap::{Parser, Subcommand};

use crate::commands::update::{check_for_update, print_update_notification};
use crate::commands::{auth::logout_command, status::status};
use crate::utils::session::set_passphrase_override;
use crate::utils::ui::{
    generate_secure_passphrase, print_error, print_info, print_success, prompt_input_with_default,
};

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
        #[arg(short, long)]
        passphrase: Option<String>,
    },
    // Decrypt {},
    Init {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        passphrase: Option<String>,
    },
    Login {},
    Logout {},
    Update {},
    Push {
        remote: Option<String>,
        #[arg(short, long)]
        passphrase: Option<String>,
    },
    Pull {
        remote: Option<String>,
        #[arg(short, long)]
        passphrase: Option<String>,
    },
    Status {
        #[arg(short, long)]
        passphrase: Option<String>,
    },
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
        Commands::Init {
            name: cli_name,
            passphrase: cli_passphrase,
        } => {
            let default_passphrase = generate_secure_passphrase(16);
            let root = Path::new(".envoy");

            if root.exists() {
                print_info("Envoy project already initialized.");
                return Ok(());
            }

            let project_name = if let Some(name) = cli_name {
                name
            } else {
                println!();
                match prompt_input_with_default("Enter project name", "My Envoy Project", None) {
                    Ok(name) => name,
                    Err(e) => {
                        print_error(&format!("Failed to read project name: {}", e));
                        std::process::exit(1);
                    }
                }
            };

            let passphrase = if let Some(pass) = cli_passphrase {
                if pass.len() < 6 {
                    print_error("Passphrase must be at least 6 characters long");
                    std::process::exit(1);
                }
                pass
            } else {
                match prompt_input_with_default(
                    "Enter project passphrase",
                    &default_passphrase,
                    Some(|input: &String| {
                        if input.len() < 6 {
                            Err("Must be at least 6 characters long".to_string())
                        } else {
                            Ok(())
                        }
                    }),
                ) {
                    Ok(pass) => pass,
                    Err(e) => {
                        print_error(&format!("Failed to read passphrase: {}", e));
                        std::process::exit(1);
                    }
                }
            };

            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    commands::init::init_project(Some(project_name), &passphrase).await
                });

            if let Err(e) = result {
                print_error(&format!("Initialization failed: {}", e));
                let root = Path::new(".envoy");
                remove_dir_all(root)?;
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
        Commands::Encrypt {
            input,
            passphrase: cli_passphrase,
        } => {
            utils::initialized::check_initialized()?;
            let default_passphrase = generate_secure_passphrase(16);

            let passphrase = if let Some(pass) = cli_passphrase {
                if pass.len() < 6 {
                    print_error("Passphrase must be at least 6 characters long");
                    std::process::exit(1);
                }
                pass
            } else {
                match prompt_input_with_default(
                    &format!("Enter passphrase to encrypt {}", input),
                    &default_passphrase,
                    Some(|input: &String| {
                        if input.len() < 6 {
                            Err("Must be at least 6 characters long".to_string())
                        } else {
                            Ok(())
                        }
                    }),
                ) {
                    Ok(pass) => pass,
                    Err(e) => {
                        print_error(&format!("Failed to read passphrase: {}", e));
                        std::process::exit(1);
                    }
                }
            };

            commands::crypto::encrypt_file(&input, &passphrase)?;
            print_success("File encrypted successfully");
        }

        // Deprecated since v0.1.5
        // Commands::Decrypt {} => {
        //     utils::initialized::check_initialized()?;
        //     let passphrase = Password::new().with_prompt("Enter passphrase").interact()?;

        //     commands::crypto::decrypt_files(&passphrase)?;
        //     print_success("File decrypted successfully");
        // }
        Commands::Push {
            remote,
            passphrase: cli_passphrase,
        } => {
            utils::initialized::check_initialized()?;

            if cli_passphrase.is_some() {
                set_passphrase_override(cli_passphrase);
            }

            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async { commands::push::push(remote.as_deref()).await });

            if let Err(e) = result {
                print_error(&format!("Push failed: {}", e));
                std::process::exit(1);
            }
        }

        Commands::Pull {
            remote,
            passphrase: cli_passphrase,
        } => {
            utils::initialized::check_initialized()?;

            if cli_passphrase.is_some() {
                set_passphrase_override(cli_passphrase);
            }

            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async { commands::pull::pull(remote.as_deref()).await });

            if let Err(e) = result {
                print_error(&format!("Pull failed: {}", e));
                std::process::exit(1);
            }
        }
        Commands::Status {
            passphrase: cli_passphrase,
        } => {
            utils::initialized::check_initialized()?;

            if cli_passphrase.is_some() {
                set_passphrase_override(cli_passphrase);
            }

            status()?;
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
