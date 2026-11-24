use std::sync::LazyLock;

use regex::Regex;

// CSI sequences (colors, cursor moves, bracketed paste), OSC sequences (window title), and short escapes.
static ANSI_ESCAPE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\x1b(?:\[[0-9;?]*[ -/]*[@-~]|\][^\x07\x1b]*(?:\x07|\x1b\\)|[()][0-9A-Za-z]|[=>78DEHM])")
        .expect("static ansi regex is valid")
});

pub fn strip_ansi(text: &str) -> String {
    ANSI_ESCAPE.replace_all(text, "").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_color_codes() {
        assert_eq!(strip_ansi("\x1b[m\x1b[32mether1\x1b[0m"), "ether1");
    }

    #[test]
    fn removes_cursor_and_bracketed_paste_sequences() {
        assert_eq!(strip_ansi("\x1b[?2004huser@host:~$ \x1b[K"), "user@host:~$ ");
    }

    #[test]
    fn removes_window_title_sequence() {
        assert_eq!(strip_ansi("\x1b]0;user@host: ~\x07user@host:~$ "), "user@host:~$ ");
    }

    #[test]
    fn leaves_plain_text_unchanged() {
        assert_eq!(strip_ansi("R1#show ip int brief"), "R1#show ip int brief");
    }

    #[test]
    fn returns_empty_for_empty_input() {
        assert_eq!(strip_ansi(""), "");
    }
}
