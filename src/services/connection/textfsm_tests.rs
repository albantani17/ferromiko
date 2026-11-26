use std::path::Path;
use std::time::Duration;

use super::connection::Connection;
use super::fake_transport::FakeTransport;
use crate::{DeviceType, FieldValue};

const TIMEOUT: Duration = Duration::from_secs(2);
const TEMPLATES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/textfsm");

fn connect(script: &[(&str, &str)]) -> Connection<FakeTransport> {
    let login = [
        ("", "\r\nR1#"),
        ("terminal length 0", "terminal length 0\r\nR1#"),
        ("terminal width 511", "terminal width 511\r\nR1#"),
    ];
    let transport = FakeTransport::new("\r\nR1#", &[&login[..], script].concat());
    Connection::from_transport(transport, DeviceType::CiscoIos, None, TIMEOUT).unwrap()
}

#[test]
fn parses_command_output_with_the_device_types_template() {
    let mut connection = connect(&[("show clock", "show clock\r\n*18:57:38.347 UTC Mon Oct 19 2015\r\nR1#")]);
    let records = connection.send_command_textfsm("show clock", Path::new(TEMPLATES_DIR)).unwrap().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["time"], FieldValue::Text("18:57:38.347".to_string()));
}

#[test]
fn returns_none_when_no_template_matches() {
    let mut connection = connect(&[("show users", "show users\r\nR1#")]);
    assert_eq!(connection.send_command_textfsm("show users", Path::new(TEMPLATES_DIR)).unwrap(), None);
}
