mod auth;
mod channel;
mod config;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tgrc", about = "Telegram Rust CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage authentication
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },

    /// Read messages from a chat (public channel, private group, or "saved")
    Read {
        /// Chat: username, chat ID, or "saved"
        chat: Option<String>,

        /// Number of messages to fetch
        #[arg(short, long)]
        limit: Option<i32>,

        /// Number of most recent messages to skip
        #[arg(short, long, default_value = "0")]
        skip: i32,
    },

    /// List your chats with their IDs
    List {
        /// Number of chats to show
        #[arg(short, long, default_value = "50")]
        limit: i32,
    },

    /// Show diagnostic info (config path, session path, auth status, version)
    Status,
}

#[derive(Subcommand)]
enum AuthAction {
    /// Log in to Telegram
    Login,
    /// Log out (revokes server session and deletes local data)
    Logout,
    /// Show auth status
    Status,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let (config, config_path) = match config::Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    let verbosity = config.telegram.tdlib_log_verbosity.unwrap_or(0);
    let (client_id, run_flag, _auth_tx, auth_rx, handle, saved_stderr) =
        auth::start_client(verbosity);

    match cli.command {
        Commands::Auth { action } => match action {
            AuthAction::Login => {
                let (result, auth_rx) = auth::wait_for_auth(
                    client_id,
                    auth_rx,
                    &config.telegram,
                    &run_flag,
                    true,
                    saved_stderr,
                )
                .await;
                match result {
                    auth::AuthResult::Ready => eprintln!("Authenticated successfully!"),
                    _ => eprintln!("Authentication failed."),
                }
                auth::shutdown(client_id, auth_rx, run_flag, handle).await;
            }
            AuthAction::Logout => {
                let (result, auth_rx) = auth::wait_for_auth(
                    client_id,
                    auth_rx,
                    &config.telegram,
                    &run_flag,
                    false,
                    saved_stderr,
                )
                .await;
                match result {
                    auth::AuthResult::Ready => {
                        eprintln!("Logging out...");
                        auth::logout(client_id, auth_rx, run_flag, handle).await;
                        eprintln!("Logged out. Server session revoked and local data removed.");
                    }
                    auth::AuthResult::NeedsLogin => {
                        eprintln!("Not logged in.");
                        auth::shutdown(client_id, auth_rx, run_flag, handle).await;
                    }
                    auth::AuthResult::Closed => {
                        eprintln!("Session already closed.");
                        run_flag.store(false, std::sync::atomic::Ordering::Release);
                        handle.await.unwrap();
                    }
                }
            }
            AuthAction::Status => {
                let (result, auth_rx) = auth::wait_for_auth(
                    client_id,
                    auth_rx,
                    &config.telegram,
                    &run_flag,
                    false,
                    saved_stderr,
                )
                .await;
                match result {
                    auth::AuthResult::Ready => eprintln!("logged in"),
                    auth::AuthResult::NeedsLogin => {
                        eprintln!("not logged in");
                        eprintln!("Run `tgrc auth login` to log in.");
                    }
                    auth::AuthResult::Closed => eprintln!("session closed"),
                }
                auth::shutdown(client_id, auth_rx, run_flag, handle).await;
            }
        },

        Commands::Read { chat, limit, skip } => {
            let (result, auth_rx) = auth::wait_for_auth(
                client_id,
                auth_rx,
                &config.telegram,
                &run_flag,
                false,
                saved_stderr,
            )
            .await;

            if !matches!(result, auth::AuthResult::Ready) {
                eprintln!("Not logged in. Run `tgrc auth login` first.");
                run_flag.store(false, std::sync::atomic::Ordering::Release);
                handle.abort();
                std::process::exit(1);
            }

            let chat = chat
                .or(config.channel.as_ref().map(|c| c.chat.clone()))
                .unwrap_or_else(|| {
                    eprintln!("Error: No chat specified. Pass a chat name/ID or set it in config.");
                    std::process::exit(1);
                });
            let limit = limit
                .or(config.channel.as_ref().and_then(|c| c.message_limit))
                .unwrap_or(20);

            if let Err(e) = channel::read_channel(client_id, &chat, limit, skip).await {
                eprintln!("Error: {e}");
                auth::shutdown(client_id, auth_rx, run_flag, handle).await;
                std::process::exit(1);
            }

            auth::shutdown(client_id, auth_rx, run_flag, handle).await;
        }

        Commands::Status => {
            let (result, auth_rx) = auth::wait_for_auth(
                client_id,
                auth_rx,
                &config.telegram,
                &run_flag,
                false,
                saved_stderr,
            )
            .await;
            eprintln!("tgrc v{}", env!("CARGO_PKG_VERSION"));
            eprintln!("Config:              {}", config_path.display());
            eprintln!("Session data:        {}", auth::db_dir().display());
            eprintln!("TDLib log verbosity: {}", verbosity);
            match result {
                auth::AuthResult::Ready => eprintln!("Auth:                logged in"),
                auth::AuthResult::NeedsLogin => eprintln!("Auth:                not logged in"),
                auth::AuthResult::Closed => eprintln!("Auth:                session closed"),
            }
            auth::shutdown(client_id, auth_rx, run_flag, handle).await;
        }

        Commands::List { limit } => {
            let (result, auth_rx) = auth::wait_for_auth(
                client_id,
                auth_rx,
                &config.telegram,
                &run_flag,
                false,
                saved_stderr,
            )
            .await;

            if !matches!(result, auth::AuthResult::Ready) {
                eprintln!("Not logged in. Run `tgrc auth login` first.");
                run_flag.store(false, std::sync::atomic::Ordering::Release);
                handle.abort();
                std::process::exit(1);
            }

            if let Err(e) = channel::list_chats(client_id, limit).await {
                eprintln!("Error: {e}");
                auth::shutdown(client_id, auth_rx, run_flag, handle).await;
                std::process::exit(1);
            }

            auth::shutdown(client_id, auth_rx, run_flag, handle).await;
        }
    }
}
