use std::sync::LazyLock;

use regex::Regex;

/// IOS truncates long hostnames in config-mode prompts, so only a prefix is matched.
const PROMPT_PREFIX_CHARS: usize = 16;
const PROMPT_END: &str = r"[^\n]*[#>$%]\s*\z";

static ANY_PROMPT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"(?m)^{PROMPT_END}")).expect("static prompt regex is valid"));

/// Matches when the buffer ends with a prompt line starting with `base_prompt`.
/// Anchored to the end so a prompt-like line inside command output does not end the read.
pub fn prompt_pattern(base_prompt: &str) -> Regex {
    let prefix: String = base_prompt.chars().take(PROMPT_PREFIX_CHARS).collect();
    let pattern = format!(r"(?m)^{}{PROMPT_END}", regex::escape(&prefix));
    Regex::new(&pattern).expect("escaped prompt prefix is a valid regex")
}

/// Matches any buffer ending with a line that looks like a prompt; used before the prompt is known.
pub fn any_prompt_pattern() -> &'static Regex {
    &ANY_PROMPT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_prompt_at_end_of_buffer() {
        assert!(prompt_pattern("R1").is_match("show clock\n12:00:00\nR1#"));
    }

    #[test]
    fn ignores_prompt_like_line_in_middle_of_output() {
        assert!(!prompt_pattern("R1").is_match("R1#\nmore output still coming"));
    }

    #[test]
    fn matches_config_mode_prompt() {
        assert!(prompt_pattern("R1").is_match("interface Gi0/1\nR1(config-if)#"));
    }

    #[test]
    fn matches_prompt_with_trailing_space() {
        assert!(prompt_pattern("[admin@MT]").is_match("\n[admin@MT] > "));
    }

    #[test]
    fn matches_truncated_long_hostname() {
        let base = "core-router-jakarta-01";
        assert!(prompt_pattern(base).is_match("\ncore-router-jaka(config)#"));
    }

    #[test]
    fn ignores_output_line_without_terminator() {
        assert!(!prompt_pattern("R1").is_match("R1 uptime is 1 week"));
    }

    #[test]
    fn escapes_regex_characters_in_prompt() {
        assert!(!prompt_pattern("[admin@MT]").is_match("\na> "));
    }

    #[test]
    fn any_prompt_matches_each_vendor_prompt() {
        for prompt in ["R1#", "R1>", "user@srx> ", "[admin@MT] > ", "root@srx% ", "user@host:~$ "] {
            assert!(any_prompt_pattern().is_match(&format!("banner\n{prompt}")), "{prompt}");
        }
    }

    #[test]
    fn any_prompt_ignores_password_prompt() {
        assert!(!any_prompt_pattern().is_match("Password: "));
    }
}
