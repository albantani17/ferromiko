//! TextFSM parsing against a template and fixture copied from ntc-templates.
#![cfg(feature = "textfsm")]

use std::path::Path;

use ferromiko::{DeviceType, Error, FieldValue, TextFsmRecord, parse_ntc, parse_textfsm};

const TEMPLATES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/textfsm");
const SHOW_CLOCK_OUTPUT: &str = include_str!("fixtures/textfsm/cisco_ios_show_clock.raw");

fn expected_clock() -> Vec<TextFsmRecord> {
    let fields = [("time", "18:57:38.347"), ("timezone", "UTC"), ("dayweek", "Mon"), ("month", "Oct"), ("day", "19"), ("year", "2015")];
    vec![fields.iter().map(|(name, value)| (name.to_string(), FieldValue::Text(value.to_string()))).collect()]
}

#[test]
fn index_lookup_parses_show_clock_like_the_ntc_fixture() {
    let records = parse_ntc(Path::new(TEMPLATES_DIR), DeviceType::CiscoIos, "show clock", SHOW_CLOCK_OUTPUT).unwrap();
    assert_eq!(records, Some(expected_clock()));
}

#[test]
fn abbreviated_command_finds_the_same_template() {
    let records = parse_ntc(Path::new(TEMPLATES_DIR), DeviceType::CiscoIos, " sh clo ", SHOW_CLOCK_OUTPUT).unwrap();
    assert_eq!(records, Some(expected_clock()));
}

#[test]
fn command_without_template_returns_none() {
    let records = parse_ntc(Path::new(TEMPLATES_DIR), DeviceType::CiscoIos, "show version", SHOW_CLOCK_OUTPUT).unwrap();
    assert_eq!(records, None);
}

#[test]
fn other_platform_does_not_use_cisco_templates() {
    let records = parse_ntc(Path::new(TEMPLATES_DIR), DeviceType::Huawei, "show clock", SHOW_CLOCK_OUTPUT).unwrap();
    assert_eq!(records, None);
}

#[test]
fn directory_without_index_is_a_textfsm_error() {
    let result = parse_ntc(Path::new(env!("CARGO_MANIFEST_DIR")), DeviceType::CiscoIos, "show clock", "");
    assert!(matches!(result, Err(Error::TextFsm(_))));
}

#[test]
fn template_file_parses_directly() {
    let template = Path::new(TEMPLATES_DIR).join("cisco_ios_show_clock.textfsm");
    assert_eq!(parse_textfsm(&template, SHOW_CLOCK_OUTPUT).unwrap(), expected_clock());
}
