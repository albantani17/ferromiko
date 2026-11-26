use std::path::Path;

use textfsm_core::Template;

use super::to_records::to_records;
use crate::types::textfsm_record::TextFsmRecord;
use crate::{Error, Result};

/// Parses `output` with one TextFSM template file (netmiko's `textfsm_template`).
pub fn parse_textfsm(template: &Path, output: &str) -> Result<Vec<TextFsmRecord>> {
    parse_with_template_text(&std::fs::read_to_string(template)?, output)
}

fn parse_with_template_text(template_text: &str, output: &str) -> Result<Vec<TextFsmRecord>> {
    let template = Template::parse_str(template_text).map_err(|error| Error::TextFsm(error.to_string()))?;
    let rows = template.parser().parse_text(output).map_err(|error| Error::TextFsm(error.to_string()))?;
    to_records(&template.header(), rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::textfsm_record::FieldValue;

    const VLAN_TEMPLATE: &str = r"Value Filldown VLAN (\d+)
Value List PORTS (\S+)

Start
  ^VLAN ${VLAN}
  ^\s+${PORTS}
  ^end -> Record

EOF
";

    fn text(value: &str) -> FieldValue {
        FieldValue::Text(value.to_string())
    }

    fn list(values: &[&str]) -> FieldValue {
        FieldValue::List(values.iter().map(|value| value.to_string()).collect())
    }

    #[test]
    fn filldown_keeps_the_value_and_list_collects_every_match() {
        let output = "VLAN 10\n  Gi0/1\n  Gi0/2\nend\n  Gi0/3\nend\n";
        let records = parse_with_template_text(VLAN_TEMPLATE, output).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0]["vlan"], text("10"));
        assert_eq!(records[0]["ports"], list(&["Gi0/1", "Gi0/2"]));
        assert_eq!(records[1]["vlan"], text("10"));
        assert_eq!(records[1]["ports"], list(&["Gi0/3"]));
    }

    #[test]
    fn output_without_matches_gives_no_records() {
        assert!(parse_with_template_text(VLAN_TEMPLATE, "% Invalid input\n").unwrap().is_empty());
    }

    #[test]
    fn unset_value_is_empty_text() {
        let template = "Value NAME (\\S+)\nValue AGE (\\d+)\n\nStart\n  ^name ${NAME} -> Record\n";
        let records = parse_with_template_text(template, "name r1\n").unwrap();
        assert_eq!(records[0]["age"], text(""));
    }

    #[test]
    fn invalid_template_is_a_textfsm_error() {
        assert!(matches!(parse_with_template_text("Start\n  ^x -> Record\n", "x"), Err(Error::TextFsm(_))));
    }

    #[test]
    fn error_action_is_a_textfsm_error() {
        let template = "Value NAME (\\S+)\n\nStart\n  ^bad -> Error\n";
        assert!(matches!(parse_with_template_text(template, "bad\n"), Err(Error::TextFsm(_))));
    }

    #[test]
    fn missing_template_file_is_an_io_error() {
        assert!(matches!(parse_textfsm(Path::new("does/not/exist.textfsm"), ""), Err(Error::Io(_))));
    }
}
