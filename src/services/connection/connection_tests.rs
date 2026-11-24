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
