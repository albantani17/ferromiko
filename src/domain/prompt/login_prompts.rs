use std::sync::LazyLock;

use regex::Regex;

pub const USERNAME_PROMPT: &str = r"(?i)(?:user ?name|login|user)\s*:\s*\z";
pub const PASSWORD_PROMPT: &str = r"(?i)password\s*:?\s*\z";

static USERNAME_PROMPT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(USERNAME_PROMPT).expect("static username regex is valid"));
static PASSWORD_PROMPT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(PASSWORD_PROMPT).expect("static password regex is valid"));

pub fn ends_with_username_prompt(output: &str) -> bool {
    USERNAME_PROMPT_REGEX.is_match(output)
}

pub fn ends_with_password_prompt(output: &str) -> bool {
    PASSWORD_PROMPT_REGEX.is_match(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_username_prompt_variants() {
        for prompt in ["Username:", "username: ", "login: ", "User name:", "\r\nUser:"] {
            assert!(ends_with_username_prompt(prompt), "{prompt:?}");
        }
    }

    #[test]
    fn recognizes_password_prompt_variants() {
        for prompt in ["Password:", "password: ", "Password"] {
            assert!(ends_with_password_prompt(prompt), "{prompt:?}");
        }
    }

    #[test]
    fn ignores_device_prompt() {
        assert!(!ends_with_username_prompt("ZXAN#"));
        assert!(!ends_with_password_prompt("ZXAN#"));
    }

    #[test]
    fn ignores_prompt_word_in_middle_of_output() {
        assert!(!ends_with_password_prompt("Password: changed\nZXAN#"));
    }
}
