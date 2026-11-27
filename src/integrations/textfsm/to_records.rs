use textfsm_core::{ListItem, Value};

use crate::types::textfsm_record::{FieldValue, TextFsmRecord};
use crate::{Error, Result};

/// Pairs each row with the template's value names, lowercased as netmiko does.
pub fn to_records(header: &[impl AsRef<str>], rows: Vec<Vec<Value>>) -> Result<Vec<TextFsmRecord>> {
    rows.into_iter()
        .map(|row| {
            header
                .iter()
                .zip(row)
                .map(|(name, value)| Ok((name.as_ref().to_lowercase(), to_field_value(value)?)))
                .collect()
        })
        .collect()
}

fn to_field_value(value: Value) -> Result<FieldValue> {
    match value {
        Value::Empty => Ok(FieldValue::Text(String::new())),
        Value::Single(text) => Ok(FieldValue::Text(text)),
        Value::List(items) => items
            .into_iter()
            .map(|item| match item {
                ListItem::String(text) => Ok(text),
                // shortcut: List values with nested named groups (no ntc template uses them) are rejected; add a variant if needed.
                ListItem::Dict(_) => Err(Error::TextFsm("List values with nested named groups are not supported".into())),
            })
            .collect::<Result<_>>()
            .map(FieldValue::List),
    }
}
