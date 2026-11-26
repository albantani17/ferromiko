use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

use regex::bytes::{NoExpand, Regex};

const MASK: &[u8] = b"********";

/// Tees every byte read from `inner` into a log file, like netmiko's `session_log`.
/// Writes are not recorded (netmiko's `record_writes=False` default) and secrets are masked.
pub struct SessionLog<T> {
    inner: T,
    file: File,
    secrets: Option<Regex>,
}

impl<T> SessionLog<T> {
    /// Creates (truncates) the log file at `path`; empty secrets are ignored.
    pub fn create(inner: T, path: &Path, secrets: &[&str]) -> io::Result<Self> {
        Ok(Self { inner, file: File::create(path)?, secrets: secrets_pattern(secrets) })
    }
}

fn secrets_pattern(secrets: &[&str]) -> Option<Regex> {
    let mut secrets: Vec<&str> = secrets.iter().copied().filter(|secret| !secret.is_empty()).collect();
    if secrets.is_empty() {
        return None;
    }
    // Longest first, so a secret containing another one is masked whole.
    secrets.sort_by_key(|secret| std::cmp::Reverse(secret.len()));
    let alternation = secrets.iter().map(|secret| regex::escape(secret)).collect::<Vec<_>>().join("|");
    Some(Regex::new(&alternation).expect("escaped literals form a valid regex"))
}

impl<T: Read> Read for SessionLog<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(buf)?;
        let chunk = &buf[..count];
        // shortcut: masks each read on its own, so a secret split across two reads is logged in clear; upgrade to netmiko's hold-back buffer if devices echo secrets in pieces.
        match &self.secrets {
            Some(secrets) => self.file.write_all(&secrets.replace_all(chunk, NoExpand(MASK)))?,
            None => self.file.write_all(chunk)?,
        }
        Ok(count)
    }
}

impl<T: Write> Write for SessionLog<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::services::connection::fake_transport::FakeTransport;

    fn temp_log(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ferromiko-{name}-{}.log", std::process::id()))
    }

    fn read_everything<T: Read>(stream: &mut T) {
        let mut buf = [0u8; 1024];
        while stream.read(&mut buf).is_ok_and(|count| count > 0) {}
    }

    fn logged(path: &Path) -> String {
        let text = std::fs::read_to_string(path).unwrap();
        std::fs::remove_file(path).unwrap();
        text
    }

    #[test]
    fn records_bytes_read_from_the_device() {
        let path = temp_log("reads");
        let mut log = SessionLog::create(FakeTransport::new("banner\r\nR1#", &[]), &path, &[]).unwrap();
        read_everything(&mut log);
        assert_eq!(logged(&path), "banner\r\nR1#");
    }

    #[test]
    fn does_not_record_bytes_written_to_the_device() {
        let path = temp_log("writes");
        let fake = FakeTransport::new("", &[("show version", "Version 1\r\nR1#")]);
        let mut log = SessionLog::create(fake, &path, &[]).unwrap();
        log.write_all(b"show version\n").unwrap();
        read_everything(&mut log);
        assert_eq!(logged(&path), "Version 1\r\nR1#");
    }

    #[test]
    fn masks_password_and_secret_in_reads() {
        let path = temp_log("masked");
        let fake = FakeTransport::new("pw=hunter2 enable=s3cret\r\nR1#", &[]);
        let mut log = SessionLog::create(fake, &path, &["hunter2", "s3cret"]).unwrap();
        read_everything(&mut log);
        assert_eq!(logged(&path), "pw=******** enable=********\r\nR1#");
    }

    #[test]
    fn ignores_empty_secrets() {
        assert!(secrets_pattern(&["", ""]).is_none());
    }

    #[test]
    fn masks_the_longer_secret_whole_when_one_contains_another() {
        let pattern = secrets_pattern(&["abc", "abcdef"]).unwrap();
        assert_eq!(pattern.replace_all(b"xabcdefx", NoExpand(MASK)).as_ref(), b"x********x");
    }
}
