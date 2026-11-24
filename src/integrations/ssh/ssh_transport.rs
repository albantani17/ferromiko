use std::io::{self, ErrorKind, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};

use ssh2::{Channel, KeyboardInteractivePrompt, Prompt, Session};

use crate::utils::would_block::retry_while_would_block;
use crate::{DeviceParams, Error, Result};

const PTY_TERMINAL: &str = "vt100";
const PTY_COLUMNS: u32 = 511;
const PTY_ROWS: u32 = 1000;

/// Interactive shell channel over SSH. Reads are non-blocking (`WouldBlock` when idle).
pub struct SshTransport {
    session: Session,
    channel: Channel,
}

impl SshTransport {
    pub fn open(params: &DeviceParams, username_suffix: &str) -> Result<Self> {
        let address = (params.host.as_str(), params.port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(ErrorKind::NotFound, format!("cannot resolve {}", params.host)))?;
        let tcp = TcpStream::connect_timeout(&address, params.timeout)?;

        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.set_timeout(params.timeout.as_millis().try_into().unwrap_or(u32::MAX));
        session.handshake()?;
        // shortcut: host key is not verified (MITM risk); add a known_hosts check before using on untrusted networks.

        let username = format!("{}{username_suffix}", params.username);
        authenticate(&session, &username, params)?;

        let mut channel = session.channel_session()?;
        channel.request_pty(PTY_TERMINAL, None, Some((PTY_COLUMNS, PTY_ROWS, 0, 0)))?;
        channel.shell()?;
        session.set_blocking(false);
        Ok(Self { session, channel })
    }
}

fn authenticate(session: &Session, username: &str, params: &DeviceParams) -> Result<()> {
    let attempt = if let Some(key_file) = &params.key_file {
        let passphrase = Some(params.password.as_str()).filter(|password| !password.is_empty());
        session.userauth_pubkey_file(username, None, key_file, passphrase)
    } else {
        // Some Cisco/ZTE images only offer keyboard-interactive.
        session
            .userauth_password(username, &params.password)
            .or_else(|_| session.userauth_keyboard_interactive(username, &mut PasswordResponder(&params.password)))
    };
    match attempt {
        Ok(()) if session.authenticated() => Ok(()),
        Ok(()) => Err(Error::Auth(format!("server rejected credentials for {username}"))),
        Err(error) => Err(Error::Auth(format!("{username}: {error}"))),
    }
}

struct PasswordResponder<'a>(&'a str);

impl KeyboardInteractivePrompt for PasswordResponder<'_> {
    fn prompt<'b>(&mut self, _username: &str, _instructions: &str, prompts: &[Prompt<'b>]) -> Vec<String> {
        prompts.iter().map(|_| self.0.to_string()).collect()
    }
}

impl Read for SshTransport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.channel.read(buf)
    }
}

impl Write for SshTransport {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        retry_while_would_block(|| self.channel.write(buf))
    }

    fn flush(&mut self) -> io::Result<()> {
        retry_while_would_block(|| self.channel.flush())
    }
}

impl Drop for SshTransport {
    fn drop(&mut self) {
        let _ = self.channel.close();
        let _ = self.session.disconnect(None, "ferromiko disconnect", None);
    }
}
