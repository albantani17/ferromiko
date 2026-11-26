use std::path::Path;
use std::sync::Arc;

use russh::client::{Handle, Handler, KeyboardInteractiveAuthResponse};
use russh::keys::{PrivateKeyWithHashAlg, load_secret_key};

use crate::{DeviceParams, Error, Result};

/// Public key when `key_file` is set, otherwise password with a keyboard-interactive fallback.
pub async fn authenticate<H: Handler>(session: &mut Handle<H>, username: &str, params: &DeviceParams) -> Result<()> {
    match try_auth_methods(session, username, params).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(Error::Auth(format!("server rejected credentials for {username}"))),
        // Devices often drop the connection after too many bad attempts; that is still a login failure.
        Err(Error::Ssh(error)) => Err(Error::Auth(format!("{username}: {error}"))),
        Err(error) => Err(error),
    }
}

async fn try_auth_methods<H: Handler>(session: &mut Handle<H>, username: &str, params: &DeviceParams) -> Result<bool> {
    match &params.key_file {
        Some(key_file) => authenticate_with_key(session, username, key_file, &params.password).await,
        None => Ok(session.authenticate_password(username, &params.password).await?.success()
            // Some Cisco/ZTE images only offer keyboard-interactive.
            || authenticate_keyboard_interactive(session, username, &params.password).await?),
    }
}

async fn authenticate_with_key<H: Handler>(
    session: &mut Handle<H>,
    username: &str,
    key_file: &Path,
    password: &str,
) -> Result<bool> {
    let passphrase = Some(password).filter(|password| !password.is_empty());
    let key = load_secret_key(key_file, passphrase).map_err(|error| Error::Auth(format!("{}: {error}", key_file.display())))?;
    // None means the server sent no server-sig-algs, so old devices get ssh-rsa (SHA-1).
    let rsa_hash = session.best_supported_rsa_hash().await?.flatten();
    let key = PrivateKeyWithHashAlg::new(Arc::new(key), rsa_hash);
    Ok(session.authenticate_publickey(username, key).await?.success())
}

async fn authenticate_keyboard_interactive<H: Handler>(
    session: &mut Handle<H>,
    username: &str,
    password: &str,
) -> Result<bool> {
    let mut response = session.authenticate_keyboard_interactive_start(username, None).await?;
    loop {
        response = match response {
            KeyboardInteractiveAuthResponse::Success => return Ok(true),
            KeyboardInteractiveAuthResponse::Failure { .. } => return Ok(false),
            KeyboardInteractiveAuthResponse::InfoRequest { prompts, .. } => {
                let answers = vec![password.to_string(); prompts.len()];
                session.authenticate_keyboard_interactive_respond(answers).await?
            }
        };
    }
}
