use std::collections::VecDeque;
use std::io::{self, ErrorKind, Read, Write};

/// Scripted device: each written line must match the next expected command, which queues its reply.
pub struct FakeTransport {
    pending: VecDeque<u8>,
    partial_line: String,
    script: VecDeque<(String, String)>,
}

impl FakeTransport {
    pub fn new(banner: &str, script: &[(&str, &str)]) -> Self {
        Self {
            pending: banner.bytes().collect(),
            partial_line: String::new(),
            script: script.iter().map(|(command, reply)| (command.to_string(), reply.to_string())).collect(),
        }
    }
}

impl Read for FakeTransport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pending.is_empty() {
            return Err(ErrorKind::WouldBlock.into());
        }
        let count = buf.len().min(self.pending.len());
        for (slot, byte) in buf.iter_mut().zip(self.pending.drain(..count)) {
            *slot = byte;
        }
        Ok(count)
    }
}

impl Write for FakeTransport {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.partial_line.push_str(&String::from_utf8_lossy(buf));
        // A line ends at CR, LF or CRLF (Huawei sends a bare CR after the Telnet password).
        while let Some(line_end) = self.partial_line.find(['\r', '\n']) {
            let ends_with_crlf = self.partial_line[line_end..].starts_with("\r\n");
            let consumed = line_end + if ends_with_crlf { 2 } else { 1 };
            let line: String = self.partial_line.drain(..consumed).collect();
            let line = line.trim_end_matches(['\r', '\n']);
            let (expected, reply) = self.script.pop_front().unwrap_or_else(|| panic!("unexpected command {line:?}"));
            assert_eq!(line, expected, "device received the wrong command");
            self.pending.extend(reply.bytes());
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for FakeTransport {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            assert!(self.script.is_empty(), "commands never sent: {:?}", self.script);
        }
    }
}
