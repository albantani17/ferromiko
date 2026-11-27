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

fn connect(script: &[(&str, &str)]) -> Connection<FakeTransport> {
    let transport = FakeTransport::new("\r\nR1#", &[CISCO_LOGIN, script].concat());
    Connection::from_transport(transport, DeviceType::CiscoIos, None, TIMEOUT).unwrap()
}

#[test]
fn multi_step_waits_for_each_custom_expect_string() {
    let mut connection = connect(&[
        ("copy running-config startup-config", "copy running-config startup-config\r\nDestination filename [startup-config]?"),
        ("", "\r\nBuilding configuration...\r\n[OK]\r\nR1#"),
    ]);
    let output = connection
        .send_multiline(&[("copy running-config startup-config", r"Destination filename"), ("", r"\[OK\]")])
        .unwrap();
    assert_eq!(output, "Destination filename [startup-config]?\nBuilding configuration...\n[OK]\n");
}

#[test]
fn empty_expect_string_waits_for_the_prompt() {
    let mut connection = connect(&[("reload", "reload\r\nProceed with reload? [confirm]"), ("y", "y\r\nR1#")]);
    let output = connection.send_multiline(&[("reload", r"\[confirm\]"), ("y", "")]).unwrap();
    assert_eq!(output, "Proceed with reload? [confirm]\n\n");
}

#[test]
fn invalid_expect_regex_errors_before_sending() {
    let mut connection = connect(&[]);
    assert!(matches!(connection.send_multiline(&[("reload", "[confirm")]), Err(Error::Regex(_))));
}
