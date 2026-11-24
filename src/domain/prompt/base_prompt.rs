const PROMPT_TERMINATORS: [char; 4] = ['#', '>', '$', '%'];

/// The stable part of a prompt: no terminator and no `(config...)` mode suffix,
/// so it still matches after the device changes mode.
pub fn base_prompt(prompt: &str) -> String {
    let without_terminator = prompt.trim().trim_end_matches(PROMPT_TERMINATORS).trim_end();
    let without_mode = match without_terminator.find('(') {
        Some(mode_start) if mode_start > 0 => &without_terminator[..mode_start],
        _ => without_terminator,
    };
    without_mode.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_cisco_privileged_terminator() {
        assert_eq!(base_prompt("R1#"), "R1");
    }

    #[test]
    fn strips_cisco_config_mode_suffix() {
        assert_eq!(base_prompt("R1(config-if)#"), "R1");
    }

    #[test]
    fn keeps_mikrotik_brackets() {
        assert_eq!(base_prompt("[admin@MT] > "), "[admin@MT]");
    }

    #[test]
    fn strips_junos_operational_and_config_terminators() {
        assert_eq!(base_prompt("user@srx> "), "user@srx");
        assert_eq!(base_prompt("user@srx# "), "user@srx");
    }

    #[test]
    fn strips_zte_config_mode_suffix() {
        assert_eq!(base_prompt("ZXAN(config)#"), "ZXAN");
    }

    #[test]
    fn keeps_leading_parenthesis_of_linux_virtualenv_prompt() {
        assert_eq!(base_prompt("(venv) user@host:~$"), "(venv) user@host:~");
    }

    #[test]
    fn returns_empty_for_empty_prompt() {
        assert_eq!(base_prompt("  "), "");
    }
}
