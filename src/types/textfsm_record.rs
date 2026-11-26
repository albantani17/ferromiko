use std::collections::BTreeMap;

/// One TextFSM value: plain text, or a list for `List` values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValue {
    Text(String),
    List(Vec<String>),
}

/// One parsed row, keyed by lowercase value name (as netmiko returns it).
pub type TextFsmRecord = BTreeMap<String, FieldValue>;

/// What `send_command_textfsm` returns: parsed rows, or the raw output when no template
/// matched or the template found no rows (netmiko's fallback).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextFsmOutput {
    Parsed(Vec<TextFsmRecord>),
    Raw(String),
}
