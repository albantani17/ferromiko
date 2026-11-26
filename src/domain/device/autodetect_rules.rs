use regex::RegexBuilder;

use super::device_type::DeviceType;

/// A match at this priority ends the search, as in netmiko.
pub const CERTAIN_PRIORITY: u8 = 99;

/// One netmiko `SSH_MAPPER_DICT` entry: send `command`, and any of `patterns`
/// (case-insensitive) in the output means `device_type` with confidence `priority`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetectionRule {
    pub device_type: DeviceType,
    pub command: &'static str,
    pub patterns: &'static [&'static str],
    pub priority: u8,
}

/// In netmiko's try order: most shared command first, and within one command the reverse of
/// its dict order, so SmartAX is tried before plain Huawei and wins their tie.
pub const DETECTION_RULES: &[DetectionRule] = &[
    DetectionRule {
        device_type: DeviceType::Zte,
        command: "show version",
        // Not from netmiko: derived from ZXR10/ZXROS/ZXA10 `show version` banners.
        patterns: &[r"ZXR10", r"ZXROS", r"ZXA10", r"ZTE Corporation"],
        priority: 99,
    },
    DetectionRule {
        device_type: DeviceType::Junos,
        command: "show version",
        patterns: &[r"JUNOS Software Release", r"JUNOS .+ Software", r"JUNOS OS Kernel", r"JUNOS Base Version"],
        priority: 99,
    },
    DetectionRule {
        device_type: DeviceType::CiscoIos,
        command: "show version",
        patterns: &[r"Cisco IOS XE Software"],
        priority: 99,
    },
    DetectionRule {
        device_type: DeviceType::CiscoIos,
        command: "show version",
        patterns: &[r"Cisco IOS Software", r"Cisco Internetwork Operating System Software"],
        priority: 95,
    },
    DetectionRule {
        device_type: DeviceType::HuaweiOlt,
        command: "display version",
        patterns: &[r"Huawei Integrated Access Software"],
        priority: 99,
    },
    DetectionRule {
        device_type: DeviceType::Huawei,
        command: "display version",
        patterns: &[r"Huawei Technologies", r"Huawei Versatile Routing Platform Software"],
        priority: 99,
    },
    DetectionRule { device_type: DeviceType::Linux, command: "uname -a", patterns: &[r"Linux"], priority: 95 },
    DetectionRule {
        device_type: DeviceType::Mikrotik,
        command: "/system resource print",
        // Not from netmiko: RouterOS prints `platform: MikroTik` in its resource table.
        patterns: &[r"platform:\s*MikroTik"],
        priority: 99,
    },
];

/// Output that means the device rejected the command, so no pattern may count.
const INVALID_RESPONSES: &[&str] = &[
    r"% Invalid input detected",
    r"syntax error, expecting",
    r"Error: Unrecognized command",
    r"%Error",
    r"command not found",
    r"Syntax Error: unexpected argument",
    r"% Unrecognized command found at",
    r"% Unknown command, the error locates at",
    // Not from netmiko: RouterOS's rejection, e.g. `bad command name uname (line 1 column 1)`.
    r"bad command name",
];

/// True when `output` (the reply to `rule.command`) identifies `rule.device_type`.
pub fn rule_matches(rule: &DetectionRule, output: &str) -> bool {
    !matches_any(INVALID_RESPONSES, output) && matches_any(rule.patterns, output)
}

fn matches_any(patterns: &[&str], output: &str) -> bool {
    patterns.iter().any(|pattern| {
        RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .expect("static detection regex is valid")
            .is_match(output)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CISCO_IOS_SHOW_VERSION: &str = "show version\nCisco IOS Software, C2960X Software (C2960X-UNIVERSALK9-M), Version 15.2(7)E4, RELEASE SOFTWARE (fc2)\nTechnical Support: http://www.cisco.com/techsupport\nR1 uptime is 3 weeks, 2 days\nR1#";
    const CISCO_XE_SHOW_VERSION: &str = "show version\nCisco IOS XE Software, Version 17.03.04a\nCisco IOS Software [Amsterdam], ISR Software (X86_64_LINUX_IOSD-UNIVERSALK9-M), Version 17.3.4a\nR1#";
    const JUNOS_SHOW_VERSION: &str = "show version\nHostname: mx1\nModel: mx204\nJunos: 21.4R3.15\nJUNOS OS Kernel 64-bit  [20220722.6b3e8d8_builder_stable_12]\nJUNOS Base Version [21.4R3.15]\n\nadmin@mx1>";
    const ZTE_SHOW_VERSION: &str = "show version\nZXROS Version V5.00.10(1.0.0)\nCopyright (c) 2000-2018 by ZTE Corporation\nZXR10 M6000-8S uptime is 120 days\nM6000#";
    const HUAWEI_VRP_DISPLAY_VERSION: &str = "display version\nHuawei Versatile Routing Platform Software\nVRP (R) software, Version 8.180 (NE40E V800R011C00SPC200)\nCopyright (C) 2012-2018 Huawei Technologies Co., Ltd.\n<NE40E>";
    const HUAWEI_OLT_DISPLAY_VERSION: &str = "display version\nHuawei Integrated Access Software (MA5600T).\nCopyright(C) Huawei Technologies Co., Ltd. 2002-2019. All rights reserved.\nVERSION : MA5600V800R018C10\nOLT1#";
    const LINUX_UNAME: &str = "uname -a\nLinux jump01 6.1.0-18-amd64 #1 SMP PREEMPT_DYNAMIC Debian 6.1.76-1 x86_64 GNU/Linux\nadmin@jump01:~$";
    const MIKROTIK_RESOURCE: &str = "/system resource print\n                   uptime: 2w1d4h\n                  version: 7.12.1 (stable)\n               board-name: CCR2004-1G-12S+2XS\n                 platform: MikroTik\n[admin@MikroTik] >";
    const CISCO_REJECTS_DISPLAY: &str = "display version\n             ^\n% Invalid input detected at '^' marker.\n\nR1#";

    fn rule_for(device_type: DeviceType, priority: u8) -> &'static DetectionRule {
        DETECTION_RULES
            .iter()
            .find(|rule| rule.device_type == device_type && rule.priority == priority)
            .unwrap()
    }

    #[test]
    fn classic_ios_matches_at_95() {
        assert!(rule_matches(rule_for(DeviceType::CiscoIos, 95), CISCO_IOS_SHOW_VERSION));
        assert!(!rule_matches(rule_for(DeviceType::CiscoIos, 99), CISCO_IOS_SHOW_VERSION));
    }

    #[test]
    fn ios_xe_matches_at_99() {
        assert!(rule_matches(rule_for(DeviceType::CiscoIos, 99), CISCO_XE_SHOW_VERSION));
    }

    #[test]
    fn junos_show_version_matches_junos() {
        assert!(rule_matches(rule_for(DeviceType::Junos, 99), JUNOS_SHOW_VERSION));
    }

    #[test]
    fn zxros_show_version_matches_zte() {
        assert!(rule_matches(rule_for(DeviceType::Zte, 99), ZTE_SHOW_VERSION));
    }

    #[test]
    fn vrp_display_version_matches_huawei_but_not_olt() {
        assert!(rule_matches(rule_for(DeviceType::Huawei, 99), HUAWEI_VRP_DISPLAY_VERSION));
        assert!(!rule_matches(rule_for(DeviceType::HuaweiOlt, 99), HUAWEI_VRP_DISPLAY_VERSION));
    }

    #[test]
    fn smartax_display_version_matches_olt() {
        assert!(rule_matches(rule_for(DeviceType::HuaweiOlt, 99), HUAWEI_OLT_DISPLAY_VERSION));
    }

    #[test]
    fn olt_rule_is_tried_before_vrp_rule_because_both_match_smartax_output() {
        let olt = DETECTION_RULES.iter().position(|rule| rule.device_type == DeviceType::HuaweiOlt);
        let vrp = DETECTION_RULES.iter().position(|rule| rule.device_type == DeviceType::Huawei);
        assert!(olt < vrp);
        assert!(rule_matches(rule_for(DeviceType::Huawei, 99), HUAWEI_OLT_DISPLAY_VERSION));
    }

    #[test]
    fn uname_matches_linux() {
        assert!(rule_matches(rule_for(DeviceType::Linux, 95), LINUX_UNAME));
    }

    #[test]
    fn resource_print_matches_mikrotik() {
        assert!(rule_matches(rule_for(DeviceType::Mikrotik, 99), MIKROTIK_RESOURCE));
    }

    #[test]
    fn matching_ignores_case() {
        assert!(rule_matches(rule_for(DeviceType::Linux, 95), "uname -a\nLINUX box\n$"));
    }

    #[test]
    fn rejected_command_never_matches_even_if_pattern_appears() {
        let output = format!("{CISCO_REJECTS_DISPLAY}\nHuawei Technologies");
        assert!(!rule_matches(rule_for(DeviceType::Huawei, 99), &output));
    }

    #[test]
    fn routeros_rejection_counts_as_invalid() {
        let output = "uname -a\nbad command name uname (line 1 column 1)\n[admin@linux-gw] >";
        assert!(!rule_matches(rule_for(DeviceType::Linux, 95), output));
    }

    #[test]
    fn empty_output_matches_nothing() {
        assert!(DETECTION_RULES.iter().all(|rule| !rule_matches(rule, "")));
    }

    #[test]
    fn every_rule_targets_an_ssh_device_type() {
        use super::super::profile::Protocol;
        assert!(DETECTION_RULES.iter().all(|rule| rule.device_type.profile().protocol == Protocol::Ssh));
    }
}
