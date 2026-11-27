use crate::{Error, Result};

// shortcut: always sends rw-r--r-- instead of the local file's mode (scp -p); add it if devices need other modes.
const FILE_MODE: &str = "0644";

/// The `C<mode> <size> <name>\n` record that announces one file to `scp -t`.
pub fn build_file_header(size: u64, name: &str) -> Result<String> {
    if name.is_empty() || name.contains(['/', '\n']) {
        return Err(Error::Scp(format!("invalid file name for the SCP header: {name:?}")));
    }
    Ok(format!("C{FILE_MODE} {size} {name}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn announces_mode_size_and_name() {
        assert_eq!(build_file_header(1234, "config.txt").unwrap(), "C0644 1234 config.txt\n");
    }

    #[test]
    fn announces_an_empty_file() {
        assert_eq!(build_file_header(0, "empty").unwrap(), "C0644 0 empty\n");
    }

    #[test]
    fn keeps_spaces_in_the_name() {
        assert_eq!(build_file_header(1, "my file").unwrap(), "C0644 1 my file\n");
    }

    #[test]
    fn rejects_an_empty_name() {
        assert!(matches!(build_file_header(1, ""), Err(Error::Scp(_))));
    }

    #[test]
    fn rejects_a_name_with_a_slash() {
        assert!(matches!(build_file_header(1, "dir/file"), Err(Error::Scp(_))));
    }

    #[test]
    fn rejects_a_name_with_a_newline_that_would_inject_a_record() {
        assert!(matches!(build_file_header(1, "a\nC0644 1 b"), Err(Error::Scp(_))));
    }
}
