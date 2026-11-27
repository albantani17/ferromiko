use std::io::{self, ErrorKind};
use std::sync::Arc;
use std::time::Duration;

use russh::Disconnect;
use russh::client::{self, Handle, Handler};
use russh::keys::PublicKeyOrCertificate;
use tokio::net::TcpStream;

use super::algorithms::client_config;
use super::authenticate::authenticate;
use crate::{DeviceParams, Result};

pub type SshSession = Handle<AcceptAnyHostKey>;

/// SSH handshake and login over an already connected socket; callers bound it with their own timeout.
pub async fn connect_session(tcp: TcpStream, params: &DeviceParams, username: &str) -> Result<SshSession> {
    let config = client_config(params.legacy_ssh_algorithms);
    let mut session = client::connect_stream(Arc::new(config), tcp, AcceptAnyHostKey).await?;
    authenticate(&mut session, username, params).await?;
    Ok(session)
}

pub async fn connect_tcp(params: &DeviceParams) -> Result<TcpStream> {
    Ok(within(params.timeout, TcpStream::connect((params.host.as_str(), params.port))).await??)
}

pub async fn disconnect(session: &mut SshSession) {
    let _ = session.disconnect(Disconnect::ByApplication, "ferromiko disconnect", "").await;
    let _ = session.await;
}

pub async fn within<T>(limit: Duration, operation: impl Future<Output = T>) -> io::Result<T> {
    tokio::time::timeout(limit, operation)
        .await
        .map_err(|_| io::Error::new(ErrorKind::TimedOut, format!("SSH took longer than {limit:?}")))
}

pub struct AcceptAnyHostKey;

impl Handler for AcceptAnyHostKey {
    type Error = russh::Error;

    async fn check_server_key(&mut self, _server_key: &PublicKeyOrCertificate) -> std::result::Result<bool, Self::Error> {
        // shortcut: host key is not verified (MITM risk); add a known_hosts check before using on untrusted networks.
        Ok(true)
    }
}
