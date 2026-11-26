use std::time::Duration;

use super::connection::Connection;
use super::fake_transport::FakeTransport;
use crate::{DeviceType, Error};

const TIMEOUT: Duration = Duration::from_secs(2);
const CISCO_ERROR_PATTERN: &str = r"^% Invalid input";

const CISCO_LOGIN: &[(&str, &str)] = &[
    ("", "\r\nR1#"),
    ("terminal length 0", "terminal length 0\r\nR1#"),
    ("terminal width 511", "terminal width 511\r\nR1#"),
];

const ENTER_CONFIG: &[(&str, &str)] = &[
    ("", "\r\nR1#"),
    ("configure terminal", "configure terminal\r\nEnter configuration commands.\r\nR1(config)#"),
    ("", "\r\nR1(config)#"),
];

fn connect(script: &[&[(&str, &str)]]) -> Connection<FakeTransport> {
    let transport = FakeTransport::new("\r\nR1#", &script.concat());
    Connection::from_transport(transport, DeviceType::CiscoIos, None, TIMEOUT).unwrap()
}

#[test]
fn clean_config_passes_and_exits_config_mode() {
    let mut connection = connect(&[
        CISCO_LOGIN,
        ENTER_CONFIG,
        &[
            ("interface Gi0/1", "interface Gi0/1\r\nR1(config-if)#"),
            ("description uplink", "description uplink\r\nR1(config-if)#"),
            ("", "\r\nR1(config-if)#"),
            ("end", "end\r\nR1#"),
            ("", "\r\nR1#"),
        ],
    ]);
    let output =
        connection.send_config_set_checked(&["interface Gi0/1", "description uplink"], CISCO_ERROR_PATTERN).unwrap();
    assert!(output.contains("Enter configuration commands."));
}

#[test]
fn error_output_stops_at_the_failing_command() {
    let mut connection = connect(&[
        CISCO_LOGIN,
        ENTER_CONFIG,
        &[(
            "interfce Gi0/1",
            "interfce Gi0/1\r\n        ^\r\n% Invalid input detected at '^' marker.\r\nR1(config)#",
        )],
    ]);
    let error = connection
        .send_config_set_checked(&["interfce Gi0/1", "description uplink"], CISCO_ERROR_PATTERN)
        .unwrap_err();
    let Error::ConfigInvalid { command, output } = error else {
        panic!("expected ConfigInvalid, got {error:?}");
    };
    assert_eq!(command, "interfce Gi0/1");
    assert!(output.contains("% Invalid input detected"));
}

#[test]
fn invalid_error_pattern_is_rejected_before_sending() {
    let mut connection = connect(&[CISCO_LOGIN]);
    let error = connection.send_config_set_checked(&["interface Gi0/1"], "(unclosed").unwrap_err();
    assert!(matches!(error, Error::Regex(_)));
}
