const PROMPT_TERMINATORS: [char; 5] = ['#', '>', '$', '%', ']'];
// Huawei firewalls in HA mode prefix the prompt with `HRP_` and a role letter, e.g. `HRP_M<USG>`.
const HA_PREFIX: &str = "HRP_";
const PROMPT_OPENERS: [char; 2] = ['<', '['];
// Huawei VRPv8 marks the candidate config state with `~` or `*` after the bracket.
const PROMPT_STATE_MARKERS: [char; 2] = ['~', '*'];

/// The stable part of a prompt: no brackets, no terminator and no `(config...)` mode suffix,
/// so it still matches after the device changes mode.
pub fn base_prompt(prompt: &str) -> String {
    let without_terminator = prompt.trim().trim_end_matches(PROMPT_TERMINATORS).trim_end();
    let without_ha_prefix = match without_terminator.strip_prefix(HA_PREFIX) {
        Some(rest) => rest.get(1..).unwrap_or_default(),
        None => without_terminator,
    };
    let without_opener = without_ha_prefix
        .trim_start_matches(PROMPT_OPENERS)
        .trim_start_matches(PROMPT_STATE_MARKERS);
    let without_mode = match without_opener.find('(') {
        Some(mode_start) if mode_start > 0 => &without_opener[..mode_start],
        _ => without_opener,
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
    fn strips_mikrotik_opening_bracket() {
        assert_eq!(base_prompt("[admin@MT] > "), "admin@MT]");
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
    fn strips_huawei_user_and_system_view_brackets() {
        assert_eq!(base_prompt("<HUAWEI>"), "HUAWEI");
        assert_eq!(base_prompt("[HUAWEI]"), "HUAWEI");
    }

    #[test]
    fn strips_huawei_vrpv8_state_marker() {
        assert_eq!(base_prompt("[~HUAWEI]"), "HUAWEI");
        assert_eq!(base_prompt("[*HUAWEI]"), "HUAWEI");
    }

    #[test]
    fn strips_huawei_firewall_ha_prefix() {
        assert_eq!(base_prompt("HRP_M<USG>"), "USG");
        assert_eq!(base_prompt("HRP_S[USG]"), "USG");
    }

    #[test]
    fn strips_huawei_olt_mode_suffix() {
        assert_eq!(base_prompt("MA5800-X7(config-if-gpon-0/1)#"), "MA5800-X7");
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
