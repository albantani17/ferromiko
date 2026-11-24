use std::io::{self, ErrorKind, Write};
use std::thread::sleep;
use std::time::Duration;

const RETRY_INTERVAL: Duration = Duration::from_millis(10);

pub fn retry_while_would_block<R>(mut operation: impl FnMut() -> io::Result<R>) -> io::Result<R> {
    loop {
        match operation() {
            Err(error) if error.kind() == ErrorKind::WouldBlock => sleep(RETRY_INTERVAL),
            result => return result,
        }
    }
}

/// `write_all` for non-blocking writers: std's version gives up on `WouldBlock` mid-buffer.
pub fn write_all_retrying<W: Write>(writer: &mut W, mut bytes: &[u8]) -> io::Result<()> {
    while !bytes.is_empty() {
        let written = retry_while_would_block(|| writer.write(bytes))?;
        if written == 0 {
            return Err(ErrorKind::WriteZero.into());
        }
        bytes = &bytes[written..];
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FlakyWriter {
        written: Vec<u8>,
        calls: usize,
    }

    impl Write for FlakyWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.calls += 1;
            if self.calls % 2 == 1 {
                return Err(ErrorKind::WouldBlock.into());
            }
            let count = buf.len().min(2);
            self.written.extend_from_slice(&buf[..count]);
            Ok(count)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn writes_everything_through_would_block_and_partial_writes() {
        let mut writer = FlakyWriter { written: Vec::new(), calls: 0 };
        write_all_retrying(&mut writer, b"hello").unwrap();
        assert_eq!(writer.written, b"hello");
    }

    #[test]
    fn writes_nothing_for_empty_input() {
        let mut writer = FlakyWriter { written: Vec::new(), calls: 0 };
        write_all_retrying(&mut writer, b"").unwrap();
        assert_eq!(writer.calls, 0);
    }
}
