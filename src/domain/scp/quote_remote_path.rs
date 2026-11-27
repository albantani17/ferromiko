use std::borrow::Cow;

/// Shell-quotes the path for the remote `scp` command line, leaving safe paths bare
/// (as Python's `shlex.quote` does) so device paths like `flash:/image.bin` reach IOS untouched.
pub fn quote_remote_path(path: &str) -> Cow<'_, str> {
    let is_safe = |c: char| c.is_ascii_alphanumeric() || "_@%+=:,./~-".contains(c);
    if !path.is_empty() && path.chars().all(is_safe) {
        return Cow::Borrowed(path);
    }
    Cow::Owned(format!("'{}'", path.replace('\'', r#"'"'"'"#)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_a_device_path_bare() {
        assert_eq!(quote_remote_path("flash:/c2960-lanbase.bin"), "flash:/c2960-lanbase.bin");
    }

    #[test]
    fn leaves_a_unix_path_bare() {
        assert_eq!(quote_remote_path("/tmp/backup_1.cfg"), "/tmp/backup_1.cfg");
    }

    #[test]
    fn quotes_a_path_with_spaces() {
        assert_eq!(quote_remote_path("my file.txt"), "'my file.txt'");
    }

    #[test]
    fn neutralises_shell_metacharacters() {
        assert_eq!(quote_remote_path("x; rm -rf ~"), "'x; rm -rf ~'");
    }

    #[test]
    fn escapes_a_single_quote() {
        assert_eq!(quote_remote_path("it's"), r#"'it'"'"'s'"#);
    }

    #[test]
    fn quotes_an_empty_path() {
        assert_eq!(quote_remote_path(""), "''");
    }
}
