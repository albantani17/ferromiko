use std::time::Duration;

use super::connection::Connection;
use super::fake_transport::FakeTransport;
use crate::{DeviceType, Error};

const TIMEOUT: Duration = Duration::from_secs(2);

const CISCO_LOGIN: &[(&str, &str)] = &[
    ("", "\r\nR1#"),
    ("terminal length 0", "terminal length 0\r\nR1#"),
    ("terminal width 511", "terminal width 511\r\nR1#"),
];

fn connect(device_type: DeviceType, banner: &str, script: &[&[(&str, &str)]]) -> Connection<FakeTransport> {
    let transport = FakeTransport::new(banner, &script.concat());
    Connection::from_transport(transport, device_type, Some("secret".to_string()), TIMEOUT).unwrap()
}

#[test]
fn cisco_send_command_strips_echo_and_prompt() {
    let mut connection = connect(
        DeviceType::CiscoIos,
        "Authorized access only\r\nR1#",
        &[CISCO_LOGIN, &[("show version", "show version\r\nCisco IOS Software\r\nR1 uptime is 1 week\r\nR1#")]],
    );
    assert_eq!(connection.send_command("show version").unwrap(), "Cisco IOS Software\nR1 uptime is 1 week");
}

#[test]
fn cisco_enable_sends_secret_at_password_prompt() {
    let mut connection = connect(
        DeviceType::CiscoIos,
        "\r\nR1>",
        &[
            &[
                ("", "\r\nR1>"),
                ("terminal length 0", "terminal length 0\r\nR1>"),
                ("terminal width 511", "terminal width 511\r\nR1>"),
            ],
            &[("", "\r\nR1>"), ("enable", "enable\r\nPassword: "), ("secret", "\r\nR1#"), ("", "\r\nR1#")],
        ],
    );
    connection.enable().unwrap();
}

#[test]
fn cisco_enable_fails_when_secret_is_rejected() {
    let mut connection = connect(
        DeviceType::CiscoIos,
        "\r\nR1>",
        &[
            &[
                ("", "\r\nR1>"),
                ("terminal length 0", "terminal length 0\r\nR1>"),
                ("terminal width 511", "terminal width 511\r\nR1>"),
            ],
            &[("", "\r\nR1>"), ("enable", "enable\r\nPassword: "), ("secret", "\r\nPassword: ")],
        ],
    );
    assert!(matches!(connection.enable(), Err(Error::Auth(_))));
}

#[test]
fn cisco_send_config_set_enters_and_exits_config_mode() {
    let mut connection = connect(
        DeviceType::CiscoIos,
        "\r\nR1#",
        &[
            CISCO_LOGIN,
            &[
                ("", "\r\nR1#"),
                ("configure terminal", "configure terminal\r\nEnter configuration commands.\r\nR1(config)#"),
                ("", "\r\nR1(config)#"),
                ("interface Gi0/1", "interface Gi0/1\r\nR1(config-if)#"),
                ("description uplink", "description uplink\r\nR1(config-if)#"),
                ("", "\r\nR1(config-if)#"),
                ("end", "end\r\nR1#"),
                ("", "\r\nR1#"),
            ],
        ],
    );
    let output = connection.send_config_set(&["interface Gi0/1", "description uplink"]).unwrap();
    assert!(output.contains("Enter configuration commands."));
}

#[test]
fn cisco_save_config_runs_write_memory() {
    let mut connection = connect(
        DeviceType::CiscoIos,
        "\r\nR1#",
        &[CISCO_LOGIN, &[("write memory", "write memory\r\nBuilding configuration...\r\n[OK]\r\nR1#")]],
    );
    assert_eq!(connection.save_config().unwrap(), "Building configuration...\n[OK]");
}

#[test]
fn zte_save_config_runs_write() {
    let mut connection = connect(
        DeviceType::Zte,
        "\r\nZXAN#",
        &[&[("", "\r\nZXAN#"), ("terminal length 0", "terminal length 0\r\nZXAN#"), ("write", "write\r\n[OK]\r\nZXAN#")]],
    );
    assert_eq!(connection.save_config().unwrap(), "[OK]");
}

#[test]
fn mikrotik_send_config_set_needs_no_config_mode() {
    let mut connection = connect(
        DeviceType::Mikrotik,
        "\r\n[admin@MT] > ",
        &[&[
            ("", "\r\n[admin@MT] > "),
            ("/ip address add address=10.0.0.1/24 interface=ether1", "/ip address add address=10.0.0.1/24 interface=ether1\r\n[admin@MT] > "),
        ]],
    );
    let output = connection.send_config_set(&["/ip address add address=10.0.0.1/24 interface=ether1"]).unwrap();
    assert_eq!(output.trim(), "");
}

#[test]
fn mikrotik_send_command_returns_body() {
    let mut connection = connect(
        DeviceType::Mikrotik,
        "\r\n[admin@MT] > ",
        &[&[
            ("", "\r\n[admin@MT] > "),
            ("/system identity print", "/system identity print\r\n  name: MT\r\n[admin@MT] > "),
        ]],
    );
    assert_eq!(connection.send_command("/system identity print").unwrap(), "  name: MT");
}

const JUNOS_LOGIN: &[(&str, &str)] = &[
    ("", "\r\nuser@srx> "),
    ("set cli complete-on-space off", "set cli complete-on-space off\r\nDisabling complete-on-space\r\n\r\nuser@srx> "),
    ("set cli screen-length 0", "set cli screen-length 0\r\nScreen length set to 0\r\n\r\nuser@srx> "),
    ("set cli screen-width 511", "set cli screen-width 511\r\nScreen width set to 511\r\n\r\nuser@srx> "),
];

#[test]
fn junos_switches_from_shell_to_cli() {
    connect(
        DeviceType::Junos,
        "\r\nroot@srx% ",
        &[&[("", "\r\nroot@srx% "), ("cli", "cli\r\nuser@srx> ")], JUNOS_LOGIN],
    );
}

#[test]
fn junos_config_set_then_commit() {
    let mut connection = connect(
        DeviceType::Junos,
        "\r\nuser@srx> ",
        &[
            JUNOS_LOGIN,
            &[
                ("", "\r\nuser@srx> "),
                ("configure", "configure\r\nEntering configuration mode\r\n\r\n[edit]\r\nuser@srx# "),
                ("", "\r\n\r\n[edit]\r\nuser@srx# "),
                ("set system host-name srx2", "set system host-name srx2\r\n\r\n[edit]\r\nuser@srx# "),
                ("", "\r\n\r\n[edit]\r\nuser@srx# "),
                ("commit", "commit\r\ncommit complete\r\n\r\n[edit]\r\nuser@srx# "),
                ("", "\r\n\r\n[edit]\r\nuser@srx# "),
                ("exit configuration-mode", "exit configuration-mode\r\nExiting configuration mode\r\n\r\nuser@srx> "),
                ("", "\r\nuser@srx> "),
            ],
        ],
    );
    connection.send_config_set(&["set system host-name srx2"]).unwrap();
    assert!(connection.commit().unwrap().contains("commit complete"));
}

#[test]
fn junos_commit_failure_is_reported() {
    let mut connection = connect(
        DeviceType::Junos,
        "\r\nuser@srx> ",
        &[
            JUNOS_LOGIN,
            &[
                ("", "\r\nuser@srx> "),
                ("configure", "configure\r\nEntering configuration mode\r\n\r\n[edit]\r\nuser@srx# "),
                ("", "\r\n[edit]\r\nuser@srx# "),
                ("commit", "commit\r\nerror: configuration check-out failed\r\n\r\n[edit]\r\nuser@srx# "),
            ],
        ],
    );
    assert!(matches!(connection.commit(), Err(Error::CommitFailed(output)) if output.contains("check-out failed")));
}

#[test]
fn linux_save_config_is_unsupported() {
    let mut connection = connect(
        DeviceType::Linux,
        "\r\nuser@host:~$ ",
        &[&[("", "\r\nuser@host:~$ "), ("export TERM=dumb", "export TERM=dumb\r\nuser@host:~$ ")]],
    );
    assert!(matches!(connection.save_config(), Err(Error::Unsupported("save_config"))));
}

#[test]
fn connect_times_out_when_device_never_shows_a_prompt() {
    let transport = FakeTransport::new("", &[("", "")]);
    let result = Connection::from_transport(transport, DeviceType::CiscoIos, None, Duration::from_millis(100));
    assert!(matches!(result, Err(Error::Timeout { .. })));
}

const HUAWEI_VRP_LOGIN: &[(&str, &str)] = &[
    ("", "\r\n<HUAWEI>"),
    (
        "screen-length 0 temporary",
        "screen-length 0 temporary\r\nInfo: The configuration takes effect on the current user terminal interface only.\r\n<HUAWEI>",
    ),
];

#[test]
fn huawei_send_command_strips_echo_and_prompt() {
    let mut connection = connect(
        DeviceType::Huawei,
        "\r\nInfo: The max number of VTY users is 5.\r\n<HUAWEI>",
        &[
            HUAWEI_VRP_LOGIN,
            &[("display version", "display version\r\nHuawei Versatile Routing Platform Software\r\nVRP (R) software, Version 5.170\r\n<HUAWEI>")],
        ],
    );
    assert_eq!(
        connection.send_command("display version").unwrap(),
        "Huawei Versatile Routing Platform Software\nVRP (R) software, Version 5.170"
    );
}

#[test]
fn huawei_declines_password_change_at_login() {
    connect(
        DeviceType::Huawei,
        "\r\nThe password needs to be changed. Change now? [Y/N]:",
        &[&[("N", "N\r\nInfo: The max number of VTY users is 5.\r\n<HUAWEI>")], HUAWEI_VRP_LOGIN],
    );
}

#[test]
fn huawei_send_config_set_uses_system_view_and_return() {
    let mut connection = connect(
        DeviceType::Huawei,
        "\r\n<HUAWEI>",
        &[
            HUAWEI_VRP_LOGIN,
            &[
                ("", "\r\n<HUAWEI>"),
                ("system-view", "system-view\r\nEnter system view, return user view with Ctrl+Z.\r\n[HUAWEI]"),
                ("", "\r\n[HUAWEI]"),
                ("interface GigabitEthernet0/0/1", "interface GigabitEthernet0/0/1\r\n[HUAWEI-GigabitEthernet0/0/1]"),
                ("description uplink", "description uplink\r\n[HUAWEI-GigabitEthernet0/0/1]"),
                ("", "\r\n[HUAWEI-GigabitEthernet0/0/1]"),
                ("return", "return\r\n<HUAWEI>"),
                ("", "\r\n<HUAWEI>"),
            ],
        ],
    );
    let output = connection.send_config_set(&["interface GigabitEthernet0/0/1", "description uplink"]).unwrap();
    assert!(output.contains("Enter system view"));
}

#[test]
fn huawei_save_config_confirms_with_y() {
    let mut connection = connect(
        DeviceType::Huawei,
        "\r\n<HUAWEI>",
        &[
            HUAWEI_VRP_LOGIN,
            &[
                (
                    "save",
                    "save\r\n  The current configuration will be written to the device.\r\n  Are you sure to continue?[Y/N]",
                ),
                ("y", "y\r\n  Now saving the current configuration to the slot 0.\r\n  Save the configuration successfully.\r\n<HUAWEI>"),
            ],
        ],
    );
    assert!(connection.save_config().unwrap().contains("Save the configuration successfully."));
}

#[test]
fn huawei_has_no_enable_mode() {
    let mut connection = connect(DeviceType::Huawei, "\r\n<HUAWEI>", &[HUAWEI_VRP_LOGIN]);
    connection.enable().unwrap();
    assert!(connection.check_enable_mode().unwrap());
}

const HUAWEI_OLT_LOGIN: &[(&str, &str)] = &[
    ("", "\r\nMA5800-X7>"),
    ("", "\r\nMA5800-X7>"),
    ("enable", "enable\r\n\r\nMA5800-X7#"),
    ("", "\r\nMA5800-X7#"),
    ("undo smart", "undo smart\r\n\r\nMA5800-X7#"),
    ("infoswitch cli OFF", "infoswitch cli OFF\r\n\r\nMA5800-X7#"),
    ("scroll", "scroll\r\n\r\nMA5800-X7#"),
];

#[test]
fn huawei_olt_enables_before_disabling_interaction_and_paging() {
    let mut connection = connect(
        DeviceType::HuaweiOlt,
        "\r\nMA5800-X7>",
        &[HUAWEI_OLT_LOGIN, &[("display board 0", "display board 0\r\n  SlotID  BoardName  Status\r\n  1       H901GPHF   Normal\r\n\r\nMA5800-X7#")]],
    );
    assert_eq!(
        connection.send_command("display board 0").unwrap(),
        "  SlotID  BoardName  Status\n  1       H901GPHF   Normal\n"
    );
}

#[test]
fn huawei_olt_answers_yes_no_question_at_login() {
    connect(
        DeviceType::HuaweiOlt,
        "\r\nThe user has logged on another terminal. Continue? (YES/NO)[YES]:",
        &[&[("yes", "yes\r\n\r\nMA5800-X7>")], HUAWEI_OLT_LOGIN],
    );
}

#[test]
fn huawei_olt_send_config_set_uses_config_and_return() {
    let mut connection = connect(
        DeviceType::HuaweiOlt,
        "\r\nMA5800-X7>",
        &[
            HUAWEI_OLT_LOGIN,
            &[
                ("", "\r\nMA5800-X7#"),
                ("config", "config\r\n\r\nMA5800-X7(config)#"),
                ("", "\r\nMA5800-X7(config)#"),
                ("interface gpon 0/1", "interface gpon 0/1\r\n\r\nMA5800-X7(config-if-gpon-0/1)#"),
                ("", "\r\nMA5800-X7(config-if-gpon-0/1)#"),
                ("return", "return\r\n\r\nMA5800-X7#"),
                ("", "\r\nMA5800-X7#"),
            ],
        ],
    );
    connection.send_config_set(&["interface gpon 0/1"]).unwrap();
}
