use std::io::{self, Read, Write};
use std::sync::mpsc;
use std::time::Duration;

use russh::client;
use russh::{ChannelMsg, ChannelReadHalf, ChannelWriteHalf};
use tokio::runtime::{Builder, Runtime};

use super::channel_output::ChannelOutput;
use super::connect_session::{SshSession, connect_session, connect_tcp, disconnect, within};
use crate::{DeviceParams, Result};

const PTY_TERMINAL: &str = "vt100";
const PTY_COLUMNS: u32 = 511;
const PTY_ROWS: u32 = 1000;
const DISCONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// Interactive shell channel over SSH. Reads are non-blocking (`WouldBlock` when idle).
pub struct SshTransport {
    // One worker thread keeps the async russh session running between our sync calls.
    // shortcut: write/drop use `block_on`, which panics inside a tokio runtime; call from spawn_blocking there.
    runtime: Runtime,
    session: SshSession,
    input: ChannelWriteHalf<client::Msg>,
    output: ChannelOutput,
}

impl SshTransport {
    pub fn open(params: &DeviceParams, username_suffix: &str) -> Result<Self> {
        let runtime = Builder::new_multi_thread().worker_threads(1).enable_all().build()?;
        let username = format!("{}{username_suffix}", params.username);
        let (session, input, reader, early_output) = runtime.block_on(open_shell(params, &username))?;
        let (chunk_sender, chunks) = mpsc::channel();
        runtime.spawn(forward_output(reader, chunk_sender));
        Ok(Self { runtime, session, input, output: ChannelOutput::new(chunks, early_output) })
    }
}

type OpenShell = (SshSession, ChannelWriteHalf<client::Msg>, ChannelReadHalf, Vec<u8>);

async fn open_shell(params: &DeviceParams, username: &str) -> Result<OpenShell> {
    let tcp = connect_tcp(params).await?;
    within(params.timeout, async {
        let session = connect_session(tcp, params, username).await?;
        let (mut reader, input) = session.channel_open_session().await?.split();
        let mut early_output = Vec::new();
        input.request_pty(true, PTY_TERMINAL, PTY_COLUMNS, PTY_ROWS, 0, 0, &[]).await?;
        wait_for_reply(&mut reader, &mut early_output).await?;
        input.request_shell(true).await?;
        wait_for_reply(&mut reader, &mut early_output).await?;
        Ok((session, input, reader, early_output))
    })
    .await?
}

/// Waits for the server's answer to a channel request, keeping any shell output that arrives first.
async fn wait_for_reply(reader: &mut ChannelReadHalf, early_output: &mut Vec<u8>) -> Result<()> {
    loop {
        match reader.wait().await {
            Some(ChannelMsg::Success) => return Ok(()),
            Some(ChannelMsg::Failure) => return Err(russh::Error::RequestDenied.into()),
            Some(ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. }) => early_output.extend_from_slice(&data),
            Some(ChannelMsg::Eof | ChannelMsg::Close) | None => return Err(russh::Error::Disconnect.into()),
            Some(_) => {}
        }
    }
}

/// Pumps shell output to the sync side; dropping `chunks` on EOF makes reads return `Ok(0)`.
async fn forward_output(mut reader: ChannelReadHalf, chunks: mpsc::Sender<Vec<u8>>) {
    while let Some(message) = reader.wait().await {
        let data = match message {
            ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => data,
            ChannelMsg::Eof | ChannelMsg::Close => break,
            _ => continue,
        };
        if chunks.send(data.to_vec()).is_err() {
            break;
        }
    }
}

impl Read for SshTransport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.output.read(buf)
    }
}

impl Write for SshTransport {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.runtime.block_on(self.input.data_bytes(buf.to_vec())).map_err(io::Error::other)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for SshTransport {
    fn drop(&mut self) {
        let _ = self.runtime.block_on(within(DISCONNECT_TIMEOUT, async {
            let _ = self.input.close().await;
            disconnect(&mut self.session).await;
        }));
    }
}
