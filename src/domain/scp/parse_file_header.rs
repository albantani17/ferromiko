use super::parse_reply::parse_reply;
use crate::{Error, Result};

/// Reads the size from the `C<mode> <size> <name>\n` record `scp -f` sends before the file bytes.
pub fn parse_file_header(line: &[u8]) -> Result<u64> {
    let text = String::from_utf8_lossy(line);
    let malformed = || Error::Scp(format!("malformed file header: {text:?}"));
    match line.first() {
        Some(b'C') => {
            let mut fields = text[1..].trim_end_matches('\n').splitn(3, ' ');
            let (mode, size, name) = (fields.next(), fields.next(), fields.next());
            let valid_mode = mode.is_some_and(|mode| u32::from_str_radix(mode, 8).is_ok());
            if !valid_mode || name.is_none_or(str::is_empty) {
                return Err(malformed());
            }
            size.and_then(|size| size.parse().ok()).ok_or_else(malformed)
        }
        Some(b'D') => Err(Error::Scp("remote path is a directory; recursive copy is not supported".to_string())),
        Some(b'T') => Err(Error::Scp("unexpected timestamp record; preserving times is not supported".to_string())),
        _ => parse_reply(line).and(Err(malformed())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(line: &[u8]) -> String {
        match parse_file_header(line) {
            Err(Error::Scp(message)) => message,
            other => panic!("expected an SCP error, got {other:?}"),
        }
    }

    #[test]
    fn reads_the_size_of_a_file_record() {
        assert_eq!(parse_file_header(b"C0644 1234 config.txt\n").unwrap(), 1234);
    }

    #[test]
    fn reads_a_zero_size() {
        assert_eq!(parse_file_header(b"C0600 0 empty\n").unwrap(), 0);
    }

    #[test]
    fn accepts_a_name_with_spaces() {
        assert_eq!(parse_file_header(b"C0644 7 my file\n").unwrap(), 7);
    }

    #[test]
    fn returns_the_remote_error_instead_of_a_header() {
        assert_eq!(message(b"\x01scp: /nope: No such file or directory\n"), "scp: /nope: No such file or directory");
    }

    #[test]
    fn rejects_a_directory_record() {
        assert!(message(b"D0755 0 dir\n").contains("directory"));
    }

    #[test]
    fn rejects_a_timestamp_record() {
        assert!(message(b"T1700000000 0 1700000000 0\n").contains("timestamp"));
    }

    #[test]
    fn rejects_a_size_that_is_not_a_number() {
        assert!(message(b"C0644 big file\n").starts_with("malformed"));
    }

    #[test]
    fn rejects_a_mode_that_is_not_octal() {
        assert!(message(b"C0899 1 file\n").starts_with("malformed"));
    }

    #[test]
    fn rejects_a_record_without_a_name() {
        assert!(message(b"C0644 1\n").starts_with("malformed"));
    }

    #[test]
    fn rejects_a_stray_ack() {
        assert!(message(&[0]).starts_with("malformed"));
    }
}
