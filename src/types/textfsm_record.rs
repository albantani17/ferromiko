use std::collections::BTreeMap;

/// One TextFSM value: plain text, or a list for `List` values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValue {
    Text(String),
    List(Vec<String>),
}

/// One parsed row, keyed by lowercase value name (as netmiko returns it).
pub type TextFsmRecord = BTreeMap<String, FieldValue>;
