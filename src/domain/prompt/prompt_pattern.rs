use regex::Regex;

/// IOS truncates long hostnames in config-mode prompts, so only a prefix is matched.
const PROMPT_PREFIX_CHARS: usize = 16;
/// The Huawei HA prefix, brackets and VRPv8 state markers that `base_prompt` strips,
/// e.g. `HRP_M[USG]` or `[~HUAWEI]`.
const PROMPT_OPENING: &str = r"(?:HRP_.)?[<\[]?[~*]?";

/// Matches when the buffer ends with a prompt line starting with `base_prompt` and ending with
/// one of `terminators`. Anchored to the end so a prompt-like line inside output does not end the read.
pub fn prompt_pattern(base_prompt: &str, terminators: &str) -> Regex {
    let prefix: String = base_prompt.chars().take(PROMPT_PREFIX_CHARS).collect();
    let pattern = format!(r"(?m)^{PROMPT_OPENING}{}{}", regex::escape(&prefix), prompt_end(terminators));
    Regex::new(&pattern).expect("escaped prompt prefix is a valid regex")
}

/// Matches any buffer ending with a line that looks like a prompt; used before the prompt is known.
pub fn any_prompt_pattern(terminators: &str) -> Regex {
    Regex::new(&format!(r"(?m)^{}", prompt_end(terminators))).expect("escaped terminators are a valid regex")
}

fn prompt_end(terminators: &str) -> String {
    format!(r"[^\n]*[{}]\s*\z", regex::escape(terminators))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT: &str = "#>$%";
    const HUAWEI_VRP: &str = ">]";

    #[test]
    fn matches_prompt_at_end_of_buffer() {
        assert!(prompt_pattern("R1", DEFAULT).is_match("show clock\n12:00:00\nR1#"));
    }

    #[test]
    fn ignores_prompt_like_line_in_middle_of_output() {
        assert!(!prompt_pattern("R1", DEFAULT).is_match("R1#\nmore output still coming"));
    }

    #[test]
    fn matches_config_mode_prompt() {
        assert!(prompt_pattern("R1", DEFAULT).is_match("interface Gi0/1\nR1(config-if)#"));
    }

    #[test]
    fn matches_mikrotik_prompt_with_brackets_and_trailing_space() {
        assert!(prompt_pattern("admin@MT]", DEFAULT).is_match("\n[admin@MT] > "));
    }

    #[test]
    fn matches_truncated_long_hostname() {
        let base = "core-router-jakarta-01";
        assert!(prompt_pattern(base, DEFAULT).is_match("\ncore-router-jaka(config)#"));
    }

    #[test]
    fn ignores_output_line_without_terminator() {
        assert!(!prompt_pattern("R1", DEFAULT).is_match("R1 uptime is 1 week"));
    }

    #[test]
    fn escapes_regex_characters_in_prompt() {
        assert!(!prompt_pattern("admin@MT]", DEFAULT).is_match("\nadmin@MTX> "));
    }

    #[test]
    fn matches_huawei_user_system_and_interface_views() {
        let prompt = prompt_pattern("HUAWEI", HUAWEI_VRP);
        assert!(prompt.is_match("\n<HUAWEI>"));
        assert!(prompt.is_match("\n[HUAWEI]"));
        assert!(prompt.is_match("\n[~HUAWEI-GigabitEthernet0/0/1]"));
    }

    #[test]
    fn matches_huawei_firewall_ha_prompts_in_both_views() {
        let prompt = prompt_pattern("USG", HUAWEI_VRP);
        assert!(prompt.is_match("\nHRP_M<USG>"));
        assert!(prompt.is_match("\nHRP_M[USG-GigabitEthernet1/0/1]"));
    }

    #[test]
    fn ignores_huawei_bracketed_output_without_hostname() {
        assert!(!prompt_pattern("HUAWEI", HUAWEI_VRP).is_match("save\n[OK]"));
    }

    #[test]
    fn any_prompt_matches_each_vendor_prompt() {
        for prompt in ["R1#", "R1>", "user@srx> ", "[admin@MT] > ", "root@srx% ", "user@host:~$ ", "MA5800-X7>"] {
            assert!(any_prompt_pattern(DEFAULT).is_match(&format!("banner\n{prompt}")), "{prompt}");
        }
    }

    #[test]
    fn any_prompt_uses_given_terminators() {
        assert!(any_prompt_pattern(HUAWEI_VRP).is_match("\n[HUAWEI]"));
        assert!(!any_prompt_pattern(DEFAULT).is_match("\n[HUAWEI]"));
    }

    #[test]
    fn any_prompt_ignores_password_prompt() {
        assert!(!any_prompt_pattern(DEFAULT).is_match("Password: "));
    }
}
