#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Ssh,
    Telnet,
}

impl Protocol {
    pub fn default_port(self) -> u16 {
        match self {
            Protocol::Ssh => 22,
            Protocol::Telnet => 23,
        }
    }
}

/// Vendor-specific CLI behaviour. `None` means the vendor has no such step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub protocol: Protocol,
    /// Appended to the login name (MikroTik uses it to disable colors and terminal detection).
    pub username_suffix: &'static str,
    pub enter: &'static str,
    /// Sent when the login lands in a Unix shell (prompt ends with `%`) instead of the CLI.
    pub shell_to_cli_command: Option<&'static str>,
    pub paging_commands: &'static [&'static str],
    pub enable_command: Option<&'static str>,
    pub config_enter_command: Option<&'static str>,
    pub config_exit_command: Option<&'static str>,
    /// Text present in the prompt while in configuration mode.
    pub config_prompt_marker: Option<&'static str>,
    pub commit_command: Option<&'static str>,
    pub save_command: Option<&'static str>,
}

pub const CISCO_IOS: Profile = Profile {
    protocol: Protocol::Ssh,
    username_suffix: "",
    enter: "\n",
    shell_to_cli_command: None,
    paging_commands: &["terminal length 0", "terminal width 511"],
    enable_command: Some("enable"),
    config_enter_command: Some("configure terminal"),
    config_exit_command: Some("end"),
    config_prompt_marker: Some("(config"),
    commit_command: None,
    save_command: Some("write memory"),
};

// shortcut: assumed Cisco-like (as netmiko's zte_zxros), untested on ZXROS routers and ZXAN OLTs.
pub const ZTE: Profile = Profile {
    paging_commands: &["terminal length 0"],
    save_command: Some("write"),
    ..CISCO_IOS
};

// Telnet uses CRLF as the line ending (netmiko does the same for `*_telnet` types).
pub const ZTE_TELNET: Profile = Profile {
    protocol: Protocol::Telnet,
    enter: "\r\n",
    ..ZTE
};

pub const JUNOS: Profile = Profile {
    protocol: Protocol::Ssh,
    username_suffix: "",
    enter: "\n",
    shell_to_cli_command: Some("cli"),
    paging_commands: &["set cli complete-on-space off", "set cli screen-length 0", "set cli screen-width 511"],
    enable_command: None,
    config_enter_command: Some("configure"),
    config_exit_command: Some("exit configuration-mode"),
    config_prompt_marker: Some("#"),
    commit_command: Some("commit"),
    save_command: None,
};

pub const MIKROTIK: Profile = Profile {
    protocol: Protocol::Ssh,
    username_suffix: "+ct511w4098h",
    enter: "\r\n",
    shell_to_cli_command: None,
    paging_commands: &[],
    enable_command: None,
    config_enter_command: None,
    config_exit_command: None,
    config_prompt_marker: None,
    commit_command: None,
    save_command: None,
};

pub const LINUX: Profile = Profile {
    username_suffix: "",
    enter: "\n",
    paging_commands: &["export TERM=dumb"],
    ..MIKROTIK
};
