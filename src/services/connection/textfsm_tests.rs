use std::path::Path;
use std::time::Duration;

use super::connection::Connection;
use super::fake_transport::FakeTransport;
use crate::{DeviceType, FieldValue, TextFsmOutput};

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
    let TextFsmOutput::Parsed(records) = connection.send_command_textfsm("show clock", Path::new(TEMPLATES_DIR)).unwrap()
    else {
        panic!("expected parsed rows");
    };
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["time"], FieldValue::Text("18:57:38.347".to_string()));
}

#[test]
fn returns_raw_output_when_no_template_matches() {
    let mut connection = connect(&[("show users", "show users\r\nLine User\r\nR1#")]);
    let output = connection.send_command_textfsm("show users", Path::new(TEMPLATES_DIR)).unwrap();
    assert_eq!(output, TextFsmOutput::Raw("Line User".to_string()));
}

#[test]
fn returns_raw_output_when_the_template_finds_no_rows() {
    let mut connection = connect(&[("show clock", "show clock\r\nclock not set\r\nR1#")]);
    let output = connection.send_command_textfsm("show clock", Path::new(TEMPLATES_DIR)).unwrap();
    assert_eq!(output, TextFsmOutput::Raw("clock not set".to_string()));
}
