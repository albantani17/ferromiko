use std::collections::VecDeque;
use std::io::Write;
use std::time::Duration;

use russh::client::Msg;
use russh::{ChannelMsg, ChannelReadHalf, ChannelWriteHalf};

use super::connect_session::{SshSession, within};
use crate::domain::scp::parse_reply::parse_reply;
use crate::{Error, Result};

const ACK: u8 = 0;
const MAX_LINE_LENGTH: usize = 4096;

/// Byte-level SCP exchange over an exec channel, waiting at most `timeout` for each piece of remote data.
pub struct ScpChannel {
    reader: ChannelReadHalf,
    writer: ChannelWriteHalf<Msg>,
    pending: VecDeque<u8>,
    stderr: Vec<u8>,
    timeout: Duration,
}

impl ScpChannel {
    pub async fn open(session: &SshSession, command: &str, timeout: Duration) -> Result<Self> {
        let channel = within(timeout, session.channel_open_session()).await??;
        channel.exec(true, command).await?;
        let (reader, writer) = channel.split();
        Ok(Self { reader, writer, pending: VecDeque::new(), stderr: Vec::new(), timeout })
    }

    pub async fn send(&self, bytes: &[u8]) -> Result<()> {
        Ok(within(self.timeout, self.writer.data(bytes)).await??)
    }

    pub async fn send_ack(&self) -> Result<()> {
        self.send(&[ACK]).await
    }

    /// Waits for `\0`, or fails with the remote's `\x01`/`\x02` message.
    pub async fn expect_ack(&mut self) -> Result<()> {
        self.fill().await?;
        if self.pending.front() == Some(&ACK) {
            self.pending.pop_front();
            return Ok(());
        }
        parse_reply(&self.read_line().await?)
    }

    pub async fn read_line(&mut self) -> Result<Vec<u8>> {
        let mut line = Vec::new();
        loop {
            self.fill().await?;
            while let Some(byte) = self.pending.pop_front() {
                line.push(byte);
                if byte == b'\n' {
                    return Ok(line);
                }
            }
            if line.len() > MAX_LINE_LENGTH {
                return Err(Error::Scp("remote sent a line longer than 4096 bytes".to_string()));
            }
        }
    }

    pub async fn read_into(&mut self, size: u64, sink: &mut impl Write) -> Result<()> {
        let mut remaining = size;
        while remaining > 0 {
            self.fill().await?;
            let take = self.pending.len().min(usize::try_from(remaining).unwrap_or(usize::MAX));
            let (front, back) = self.pending.as_slices();
            let from_front = take.min(front.len());
            sink.write_all(&front[..from_front])?;
            sink.write_all(&back[..take - from_front])?;
            self.pending.drain(..take);
            remaining -= take as u64;
        }
        Ok(())
    }

    /// Sends EOF and waits for the remote to close, which is when devices like IOS commit the file.
    pub async fn finish(mut self) {
        let _ = self.writer.eof().await;
        let _ = within(self.timeout, async {
            while !matches!(self.reader.wait().await, Some(ChannelMsg::Close) | None) {}
        }).await;
    }

    async fn fill(&mut self) -> Result<()> {
        while self.pending.is_empty() {
            match within(self.timeout, self.reader.wait()).await? {
                Some(ChannelMsg::Data { data }) => self.pending.extend(&data[..]),
                Some(ChannelMsg::ExtendedData { data, .. }) => self.stderr.extend_from_slice(&data),
                Some(ChannelMsg::Failure) => return Err(Error::Scp("device refused to start scp".to_string())),
                Some(ChannelMsg::Eof | ChannelMsg::Close) | None => return Err(self.closed_early()),
                Some(_) => {}
            }
        }
        Ok(())
    }

    fn closed_early(&self) -> Error {
        let stderr = String::from_utf8_lossy(&self.stderr);
        Error::Scp(format!("remote closed the channel before the transfer finished: {}", stderr.trim()))
    }
}
