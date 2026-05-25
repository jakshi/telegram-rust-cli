use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tdlib_rs::{
    enums::{AuthorizationState, LogStream, Update},
    functions,
};
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::config::TelegramConfig;

pub type ClientRuntime = (
    i32,
    Arc<AtomicBool>,
    Sender<AuthorizationState>,
    Receiver<AuthorizationState>,
    tokio::task::JoinHandle<()>,
    Option<i32>,
);

fn prompt(msg: &str) -> String {
    eprint!("{msg} ");
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read from stdin");
    input.trim().to_string()
}

fn prompt_password(msg: &str) -> String {
    eprint!("{msg} ");
    rpassword::read_password().expect("Failed to read password from stdin")
}

async fn handle_update(update: Update, auth_tx: &Sender<AuthorizationState>) {
    if let Update::AuthorizationState(update) = update {
        auth_tx.send(update.authorization_state).await.unwrap();
    }
}

pub fn db_dir() -> std::path::PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("tgrc")
        .join("tdlib")
}

/// Result of waiting for auth state
pub enum AuthResult {
    Ready,
    NeedsLogin,
    Closed,
}

fn suppress_stderr() -> Option<i32> {
    // SAFETY: These are standard POSIX fd operations. We open /dev/null (a well-known
    // path that always exists), save a copy of stderr via dup(2), then redirect stderr
    // to /dev/null via dup2. All fds are valid at call time: fd 2 (stderr) is always
    // open, and devnull is freshly opened. We close devnull after dup2 since it's no
    // longer needed. The saved fd is returned so the caller can restore stderr later.
    unsafe {
        let devnull = libc::open(c"/dev/null".as_ptr(), libc::O_WRONLY);
        if devnull < 0 {
            return None;
        }
        let saved = libc::dup(2);
        libc::dup2(devnull, 2);
        libc::close(devnull);
        Some(saved)
    }
}

pub fn restore_stderr(saved_fd: Option<i32>) {
    if let Some(fd) = saved_fd {
        // SAFETY: fd was obtained from dup(2) in suppress_stderr and has not been
        // closed since. We restore stderr by dup2'ing the saved fd back to fd 2,
        // then close the saved fd since it's no longer needed.
        unsafe {
            libc::dup2(fd, 2);
            libc::close(fd);
        }
    }
}

/// Start the TDLib client and receive loop.
/// If verbosity is 0, stderr is suppressed until `wait_for_auth` configures TDLib logging.
pub fn start_client(verbosity: i32) -> ClientRuntime {
    let saved_stderr = if verbosity == 0 {
        suppress_stderr()
    } else {
        None
    };
    let client_id = tdlib_rs::create_client();

    let (auth_tx, auth_rx) = mpsc::channel(5);
    let run_flag = Arc::new(AtomicBool::new(true));
    let run_flag_clone = run_flag.clone();

    let auth_tx_clone = auth_tx.clone();
    let handle = tokio::spawn(async move {
        while run_flag_clone.load(Ordering::Acquire) {
            let result = tokio::task::spawn_blocking(tdlib_rs::receive)
                .await
                .unwrap();

            if let Some((update, _client_id)) = result {
                handle_update(update, &auth_tx_clone).await;
            } else {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }
    });

    (client_id, run_flag, auth_tx, auth_rx, handle, saved_stderr)
}

/// Set TDLib parameters and log level, then wait for auth state.
/// If interactive is false, returns NeedsLogin instead of prompting.
pub async fn wait_for_auth(
    client_id: i32,
    mut auth_rx: Receiver<AuthorizationState>,
    config: &TelegramConfig,
    run_flag: &Arc<AtomicBool>,
    interactive: bool,
    saved_stderr: Option<i32>,
) -> (AuthResult, Receiver<AuthorizationState>) {
    let verbosity = config.tdlib_log_verbosity.unwrap_or(0);
    if verbosity == 0 {
        let _ = functions::set_log_stream(LogStream::Empty, client_id).await;
    }
    let _ = functions::set_log_verbosity_level(verbosity, client_id).await;

    // Now that TDLib logging is configured, restore stderr
    restore_stderr(saved_stderr);

    let db_dir = db_dir();

    while let Some(state) = auth_rx.recv().await {
        match state {
            AuthorizationState::WaitTdlibParameters => {
                let response = functions::set_tdlib_parameters(
                    false,
                    db_dir.to_string_lossy().into_owned(),
                    String::new(),
                    String::new(),
                    false,
                    false,
                    false,
                    false,
                    config.api_id,
                    config.api_hash.clone(),
                    "en".into(),
                    "Desktop".into(),
                    String::new(),
                    env!("CARGO_PKG_VERSION").into(),
                    client_id,
                )
                .await;

                if let Err(e) = response {
                    eprintln!("Error setting TDLib parameters: {}", e.message);
                }
            }
            AuthorizationState::WaitPhoneNumber => {
                if !interactive {
                    return (AuthResult::NeedsLogin, auth_rx);
                }
                loop {
                    let phone =
                        prompt("Enter your phone number (with country code, e.g. +1234567890):");
                    match functions::set_authentication_phone_number(phone, None, client_id).await {
                        Ok(_) => break,
                        Err(e) => eprintln!("Error: {}", e.message),
                    }
                }
            }
            AuthorizationState::WaitCode(_) => {
                if !interactive {
                    return (AuthResult::NeedsLogin, auth_rx);
                }
                loop {
                    let code = prompt("Enter the verification code:");
                    match functions::check_authentication_code(code, client_id).await {
                        Ok(_) => break,
                        Err(e) => eprintln!("Error: {}", e.message),
                    }
                }
            }
            AuthorizationState::WaitPassword(_) => {
                if !interactive {
                    return (AuthResult::NeedsLogin, auth_rx);
                }
                loop {
                    let password = prompt_password("Enter your 2FA password:");
                    match functions::check_authentication_password(password, client_id).await {
                        Ok(_) => break,
                        Err(e) => eprintln!("Error: {}", e.message),
                    }
                }
            }
            AuthorizationState::WaitEmailAddress(_) | AuthorizationState::WaitEmailCode(_) => {
                if !interactive {
                    return (AuthResult::NeedsLogin, auth_rx);
                }
                eprintln!("Error: Email-based authentication is not supported yet.");
                eprintln!(
                    "Please disable email login verification in Telegram settings and try again."
                );
                return (AuthResult::Closed, auth_rx);
            }
            AuthorizationState::Ready => {
                return (AuthResult::Ready, auth_rx);
            }
            AuthorizationState::Closed => {
                run_flag.store(false, Ordering::Release);
                return (AuthResult::Closed, auth_rx);
            }
            _ => {}
        }
    }

    (AuthResult::Closed, auth_rx)
}

/// Wait for the TDLib client to reach the Closed state (no config needed).
async fn wait_for_closed(
    client_id: i32,
    mut auth_rx: Receiver<AuthorizationState>,
    run_flag: &Arc<AtomicBool>,
) {
    while let Some(state) = auth_rx.recv().await {
        match state {
            AuthorizationState::Closed => {
                run_flag.store(false, Ordering::Release);
                return;
            }
            AuthorizationState::WaitTdlibParameters => {
                // During shutdown after log_out, TDLib may re-enter WaitTdlibParameters.
                // Just close again.
                let _ = functions::close(client_id).await;
            }
            _ => {}
        }
    }
}

/// Gracefully close the client
pub async fn shutdown(
    client_id: i32,
    auth_rx: Receiver<AuthorizationState>,
    run_flag: Arc<AtomicBool>,
    handle: tokio::task::JoinHandle<()>,
) {
    let _ = functions::close(client_id).await;
    wait_for_closed(client_id, auth_rx, &run_flag).await;
    run_flag.store(false, Ordering::Release);
    handle.await.unwrap();
}

/// Log out from Telegram (revokes server session), then clean up local data.
pub async fn logout(
    client_id: i32,
    auth_rx: Receiver<AuthorizationState>,
    run_flag: Arc<AtomicBool>,
    handle: tokio::task::JoinHandle<()>,
) {
    let _ = functions::log_out(client_id).await;
    wait_for_closed(client_id, auth_rx, &run_flag).await;
    run_flag.store(false, Ordering::Release);
    handle.await.unwrap();

    // Remove local session data after server-side logout
    let db_dir = db_dir();
    if db_dir.exists() {
        let _ = std::fs::remove_dir_all(&db_dir);
    }
}
