use std::io::{ErrorKind, Read};
use std::thread::sleep;
use std::time::{Duration, Instant};

use regex::Regex;

use crate::domain::output::strip_ansi::strip_ansi;
use crate::{Error, Result};

const POLL_INTERVAL: Duration = Duration::from_millis(20);
const CHUNK_SIZE: usize = 16 * 1024;

/// Reads until the cleaned output matches `pattern`, or fails with `Error::Timeout`.
pub fn read_until<T: Read>(transport: &mut T, pattern: &Regex, timeout: Duration) -> Result<String> {
    let started = Instant::now();
    let mut raw = Vec::new();
    loop {
        let received = read_available(transport, &mut raw)?;
        // shortcut: re-cleans the whole buffer per chunk (quadratic), fine for CLI-sized output; go incremental if multi-MB dumps get slow.
        if received && pattern.is_match(&clean(&raw)) {
            return Ok(clean(&raw));
        }
        if started.elapsed() >= timeout {
            return Err(Error::Timeout { pattern: pattern.to_string(), output: clean(&raw) });
        }
        if !received {
            sleep(POLL_INTERVAL);
        }
    }
}

/// Reads until no data arrived for `quiet`, or `max` elapsed. Never fails on silence.
pub fn read_until_quiet<T: Read>(transport: &mut T, quiet: Duration, max: Duration) -> Result<String> {
    let started = Instant::now();
    let mut last_data = Instant::now();
    let mut raw = Vec::new();
    loop {
        if read_available(transport, &mut raw)? {
            last_data = Instant::now();
        } else if last_data.elapsed() >= quiet || started.elapsed() >= max {
            return Ok(clean(&raw));
        } else {
            sleep(POLL_INTERVAL);
        }
    }
}

/// Discards whatever is already buffered so the next read starts clean.
pub fn drain<T: Read>(transport: &mut T) -> Result<()> {
    read_available(transport, &mut Vec::new()).map(|_| ())
}

fn read_available<T: Read>(transport: &mut T, raw: &mut Vec<u8>) -> Result<bool> {
    let mut chunk = [0u8; CHUNK_SIZE];
    let mut received = false;
    loop {
        match transport.read(&mut chunk) {
            Ok(0) => return Ok(received),
            Ok(count) => {
                raw.extend_from_slice(&chunk[..count]);
                received = true;
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => return Ok(received),
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        }
    }
}

fn clean(raw: &[u8]) -> String {
    strip_ansi(&String::from_utf8_lossy(raw)).replace('\r', "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn returns_cleaned_output_once_pattern_matches() {
        let mut transport = Cursor::new(b"show clock\r\n\x1b[32m12:00\x1b[0m\r\nR1#".to_vec());
        let pattern = Regex::new(r"R1#\z").unwrap();
        let output = read_until(&mut transport, &pattern, Duration::from_secs(1)).unwrap();
        assert_eq!(output, "show clock\n12:00\nR1#");
    }

    #[test]
    fn times_out_with_partial_output() {
        let mut transport = Cursor::new(b"partial".to_vec());
        let pattern = Regex::new(r"R1#\z").unwrap();
        let error = read_until(&mut transport, &pattern, Duration::from_millis(50)).unwrap_err();
        assert!(matches!(error, Error::Timeout { output, .. } if output == "partial"));
    }

    #[test]
    fn quiet_read_returns_everything_after_silence() {
        let mut transport = Cursor::new(b"Proceed? [y/n]".to_vec());
        let output = read_until_quiet(&mut transport, Duration::from_millis(30), Duration::from_secs(1)).unwrap();
        assert_eq!(output, "Proceed? [y/n]");
    }
}
