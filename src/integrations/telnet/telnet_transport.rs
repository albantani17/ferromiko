use std::io::{self, ErrorKind, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use super::telnet_codec::{TelnetDecoder, escape_iac};
use crate::Result;
use crate::utils::would_block::write_all_retrying;

/// Telnet session over TCP. Reads are non-blocking (`WouldBlock` when idle) and return only
/// terminal data; option negotiation is answered internally. Login is done by the caller.
pub struct TelnetTransport {
    stream: TcpStream,
    decoder: TelnetDecoder,
}

impl TelnetTransport {
    pub fn open(host: &str, port: u16, timeout: Duration) -> Result<Self> {
        let address = (host, port)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::new(ErrorKind::NotFound, format!("cannot resolve {host}")))?;
        let stream = TcpStream::connect_timeout(&address, timeout)?;
        stream.set_nodelay(true)?;
        stream.set_nonblocking(true)?;
        Ok(Self { stream, decoder: TelnetDecoder::new() })
    }
}

impl Read for TelnetTransport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut raw = vec![0u8; buf.len()];
        loop {
            let count = self.stream.read(&mut raw)?;
            if count == 0 {
                return Ok(0);
            }
            let decoded = self.decoder.decode(&raw[..count]);
            write_all_retrying(&mut self.stream, &decoded.replies)?;
            // Decoding only removes bytes, so the data always fits in `buf`.
            if !decoded.data.is_empty() {
                buf[..decoded.data.len()].copy_from_slice(&decoded.data);
                return Ok(decoded.data.len());
            }
        }
    }
}

impl Write for TelnetTransport {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        write_all_retrying(&mut self.stream, &escape_iac(buf))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}
