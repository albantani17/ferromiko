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

/// A question the device may ask before showing its first prompt: (regex, answer).
pub type LoginQuestion = (&'static str, &'static str);

/// Vendor-specific CLI behaviour. `None` means the vendor has no such step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub protocol: Protocol,
    /// Appended to the login name (MikroTik uses it to disable colors and terminal detection).
    pub username_suffix: &'static str,
    pub enter: &'static str,
    /// Line ending after the password in a Telnet login.
    pub telnet_password_enter: &'static str,
    /// Characters a prompt line can end with.
    pub prompt_terminators: &'static str,
    pub login_questions: &'static [LoginQuestion],
    /// Sent when the login lands in a Unix shell (prompt ends with `%`) instead of the CLI.
    pub shell_to_cli_command: Option<&'static str>,
    /// Enter enable mode before the paging commands, because some of them need it.
    pub enable_on_login: bool,
    pub paging_commands: &'static [&'static str],
    pub enable_command: Option<&'static str>,
    pub config_enter_command: Option<&'static str>,
    pub config_exit_command: Option<&'static str>,
    /// Text present in the prompt while in configuration mode.
    pub config_prompt_marker: Option<&'static str>,
    pub commit_command: Option<&'static str>,
    pub save_command: Option<&'static str>,
    /// Confirmation the save command asks for: (question regex, answer).
    pub save_confirmation: Option<(&'static str, &'static str)>,
}

const DEFAULT_PROMPT_TERMINATORS: &str = "#>$%";

pub const CISCO_IOS: Profile = Profile {
    protocol: Protocol::Ssh,
    username_suffix: "",
    enter: "\n",
    telnet_password_enter: "\r\n",
    prompt_terminators: DEFAULT_PROMPT_TERMINATORS,
    login_questions: &[],
    shell_to_cli_command: None,
    enable_on_login: false,
    paging_commands: &["terminal length 0", "terminal width 511"],
    enable_command: Some("enable"),
    config_enter_command: Some("configure terminal"),
    config_exit_command: Some("end"),
    config_prompt_marker: Some("(config"),
    commit_command: None,
    save_command: Some("write memory"),
    save_confirmation: None,
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

// Commands and prompts follow netmiko's HuaweiSSH / HuaweiTelnet.
pub const HUAWEI_VRP: Profile = Profile {
    prompt_terminators: ">]",
    // Expired-password notice; netmiko declines the change.
    login_questions: &[(r"(?:Change now|Please choose)[^\n]*\z", "N")],
    enable_on_login: false,
    paging_commands: &["screen-length 0 temporary"],
    enable_command: None,
    config_enter_command: Some("system-view"),
    config_exit_command: Some("return"),
    config_prompt_marker: Some("]"),
    save_command: Some("save"),
    save_confirmation: Some((r"[Cc]ontinue\?[^\n]*\z", "y")),
    ..CISCO_IOS
};

// netmiko notes Huawei may need a bare CR after the Telnet password.
pub const HUAWEI_VRP_TELNET: Profile = Profile {
    protocol: Protocol::Telnet,
    enter: "\r\n",
    telnet_password_enter: "\r",
    ..HUAWEI_VRP
};

// Commands and prompts follow netmiko's HuaweiSmartAXSSH (MA5600/MA5800 OLTs).
pub const HUAWEI_OLT: Profile = Profile {
    prompt_terminators: ">#",
    login_questions: &[(r"YES.NO[^\n]*\z", "yes")],
    // `infoswitch cli OFF` needs privileged mode; `undo smart` stops the `{ <cr>|... }:` parameter prompts.
    enable_on_login: true,
    paging_commands: &["undo smart", "infoswitch cli OFF", "scroll"],
    enable_command: Some("enable"),
    config_enter_command: Some("config"),
    config_exit_command: Some("return"),
    config_prompt_marker: Some(")#"),
    save_command: Some("save"),
    save_confirmation: None,
    ..CISCO_IOS
};

pub const HUAWEI_OLT_TELNET: Profile = Profile {
    protocol: Protocol::Telnet,
    enter: "\r\n",
    ..HUAWEI_OLT
};

pub const JUNOS: Profile = Profile {
    shell_to_cli_command: Some("cli"),
    paging_commands: &["set cli complete-on-space off", "set cli screen-length 0", "set cli screen-width 511"],
    enable_command: None,
    config_enter_command: Some("configure"),
    config_exit_command: Some("exit configuration-mode"),
    config_prompt_marker: Some("#"),
    commit_command: Some("commit"),
    save_command: None,
    ..CISCO_IOS
};

pub const MIKROTIK: Profile = Profile {
    username_suffix: "+ct511w4098h",
    enter: "\r\n",
    paging_commands: &[],
    enable_command: None,
    config_enter_command: None,
    config_exit_command: None,
    config_prompt_marker: None,
    save_command: None,
    ..CISCO_IOS
};

pub const LINUX: Profile = Profile {
    username_suffix: "",
    enter: "\n",
    paging_commands: &["export TERM=dumb"],
    ..MIKROTIK
};
