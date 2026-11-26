use crate::{Error, Result};

const ACK: u8 = 0;
const WARNING: u8 = 1;
const FATAL: u8 = 2;

/// Checks the remote's answer: `\0` is success, `\x01`/`\x02` carry an error message up to `\n`.
pub fn parse_reply(reply: &[u8]) -> Result<()> {
    match reply.split_first() {
        Some((&ACK, _)) => Ok(()),
        // OpenSSH sends "no such file" and permission errors as warnings, but the file is still not copied.
        Some((&(WARNING | FATAL), message)) => Err(Error::Scp(String::from_utf8_lossy(message).trim().to_string())),
        Some(_) => Err(Error::Scp(format!("unexpected reply: {:?}", String::from_utf8_lossy(reply)))),
        None => Err(Error::Scp("empty reply".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(reply: &[u8]) -> String {
        match parse_reply(reply) {
            Err(Error::Scp(message)) => message,
            other => panic!("expected an SCP error, got {other:?}"),
        }
    }

    #[test]
    fn accepts_a_null_ack() {
        assert!(parse_reply(&[0]).is_ok());
    }

    #[test]
    fn returns_the_message_of_a_warning() {
        assert_eq!(message(b"\x01scp: /nope: No such file or directory\n"), "scp: /nope: No such file or directory");
    }

    #[test]
    fn returns_the_message_of_a_fatal_error() {
        assert_eq!(message(b"\x02scp: protocol error\n"), "scp: protocol error");
    }

    #[test]
    fn rejects_an_unknown_status_byte() {
        assert!(message(b"hello\n").starts_with("unexpected reply"));
    }

    #[test]
    fn rejects_an_empty_reply() {
        assert_eq!(message(b""), "empty reply");
    }
}
