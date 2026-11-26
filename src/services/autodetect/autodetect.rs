use std::collections::HashMap;
use std::io::{Read, Write};
use std::time::Duration;

use crate::domain::device::autodetect_rules::{CERTAIN_PRIORITY, DETECTION_RULES, DetectionRule, rule_matches};
use crate::domain::prompt::prompt_pattern::any_prompt_pattern;
use crate::services::connection::read_until::{drain, read_until, read_until_quiet};
use crate::{DeviceParams, DeviceType, Error, Result, SshTransport};

/// Every terminator of the supported vendors, since the vendor is not known yet.
const ANY_VENDOR_PROMPT_TERMINATORS: &str = "#>$%]";
const LOGIN_QUIET: Duration = Duration::from_millis(300);
/// netmiko waits 1s after each command and then reads until the device is quiet.
const COMMAND_QUIET: Duration = Duration::from_secs(2);
/// SmartAX asks for optional parameters (`{ <cr>|... }:`) until `undo smart`, which autodetect never sends.
const PARAMETER_PROMPT_END: &str = "}:";

/// Guesses the device type the way netmiko's `SSHDetect` does: logs in over SSH (ignoring
/// `params.device_type`), sends each candidate's version command and returns the
/// highest-priority match, or `None` when nothing matched.
///
/// Paging is not disabled (netmiko does not either); the first page of a version command is
/// enough. MikroTik is logged in without its `+ct511w4098h` username suffix, so its output may
/// carry colors and terminal-detection noise.
pub fn autodetect(params: &DeviceParams) -> Result<Option<DeviceType>> {
    let mut transport = SshTransport::open(params, "")?;
    detect_device_type(&mut transport, params.timeout, COMMAND_QUIET)
}

fn detect_device_type<T: Read + Write>(
    transport: &mut T,
    timeout: Duration,
    command_quiet: Duration,
) -> Result<Option<DeviceType>> {
    wait_for_prompt(transport, timeout)?;
    let mut replies: HashMap<&str, String> = HashMap::new();
    let mut best: Option<&DetectionRule> = None;
    for rule in DETECTION_RULES {
        if best.is_some_and(|found| found.priority >= CERTAIN_PRIORITY) {
            break;
        }
        if !replies.contains_key(rule.command) {
            let reply = send_and_read_quiet(transport, rule, command_quiet, timeout)?;
            replies.insert(rule.command, reply);
        }
        let is_better = best.is_none_or(|found| rule.priority > found.priority);
        if is_better && rule_matches(rule, &replies[rule.command]) {
            best = Some(rule);
        }
    }
    Ok(best.map(|rule| rule.device_type))
}

fn wait_for_prompt<T: Read>(transport: &mut T, timeout: Duration) -> Result<()> {
    let any_prompt = any_prompt_pattern(ANY_VENDOR_PROMPT_TERMINATORS);
    // A device that stays silent until Enter is fine; the commands below still get answers.
    match read_until(transport, &any_prompt, timeout) {
        Ok(_) | Err(Error::Timeout { .. }) => {}
        Err(error) => return Err(error),
    }
    read_until_quiet(transport, LOGIN_QUIET, timeout)?;
    Ok(())
}

fn send_and_read_quiet<T: Read + Write>(
    transport: &mut T,
    rule: &DetectionRule,
    quiet: Duration,
    timeout: Duration,
) -> Result<String> {
    let enter = rule.device_type.profile().enter;
    drain(transport)?;
    write_line(transport, rule.command, enter)?;
    let mut reply = read_until_quiet(transport, quiet, timeout.max(quiet))?;
    if reply.trim_end().ends_with(PARAMETER_PROMPT_END) {
        write_line(transport, "", enter)?;
        reply.push_str(&read_until_quiet(transport, quiet, timeout.max(quiet))?);
    }
    Ok(reply)
}

fn write_line<T: Write>(transport: &mut T, line: &str, enter: &str) -> Result<()> {
    transport.write_all(format!("{line}{enter}").as_bytes())?;
    transport.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::connection::fake_transport::FakeTransport;

    const TIMEOUT: Duration = Duration::from_millis(500);
    const QUIET: Duration = Duration::from_millis(20);
    const CISCO_INVALID: &str = "\r\n% Invalid input detected at '^' marker.\r\nR1#";
    const ROUTEROS_INVALID: &str = "\r\nbad command name (line 1 column 1)\r\n[admin@MikroTik] > ";

    fn detect(banner: &str, script: &[(&str, &str)]) -> Option<DeviceType> {
        let mut transport = FakeTransport::new(banner, script);
        detect_device_type(&mut transport, TIMEOUT, QUIET).unwrap()
    }

    #[test]
    fn ios_xe_stops_after_the_first_certain_match() {
        let detected = detect(
            "R1#",
            &[("show version", "show version\r\nCisco IOS XE Software, Version 17.03.04a\r\nR1#")],
        );
        assert_eq!(detected, Some(DeviceType::CiscoIos));
    }

    #[test]
    fn classic_ios_tries_every_command_and_keeps_its_95_match() {
        let detected = detect(
            "R1#",
            &[
                ("show version", "show version\r\nCisco IOS Software, Version 15.2(7)E4\r\nR1#"),
                ("display version", CISCO_INVALID),
                ("uname -a", CISCO_INVALID),
                ("/system resource print", CISCO_INVALID),
            ],
        );
        assert_eq!(detected, Some(DeviceType::CiscoIos));
    }

    #[test]
    fn unknown_device_returns_none() {
        let detected = detect(
            "R1#",
            &[
                ("show version", CISCO_INVALID),
                ("display version", CISCO_INVALID),
                ("uname -a", CISCO_INVALID),
                ("/system resource print", CISCO_INVALID),
            ],
        );
        assert_eq!(detected, None);
    }

    #[test]
    fn smartax_parameter_prompt_is_answered_with_enter() {
        let detected = detect(
            "OLT1>",
            &[
                ("show version", "\r\n% Unknown command, the error locates at '^'\r\nOLT1>"),
                ("display version", "display version\r\n{ <cr>|backplane<K>|frameid/slotid<S><Length 1-15> }:"),
                ("", "\r\nHuawei Integrated Access Software (MA5600T).\r\nOLT1>"),
            ],
        );
        assert_eq!(detected, Some(DeviceType::HuaweiOlt));
    }

    #[test]
    fn mikrotik_is_found_by_its_resource_table() {
        let detected = detect(
            "[admin@MikroTik] > ",
            &[
                ("show version", ROUTEROS_INVALID),
                ("display version", ROUTEROS_INVALID),
                ("uname -a", ROUTEROS_INVALID),
                ("/system resource print", "\r\n                 platform: MikroTik\r\n[admin@MikroTik] > "),
            ],
        );
        assert_eq!(detected, Some(DeviceType::Mikrotik));
    }

    #[test]
    fn silent_device_without_banner_is_still_probed() {
        let detected = detect("", &[("show version", "show version\r\nJUNOS OS Kernel 64-bit\r\nadmin@mx1> ")]);
        assert_eq!(detected, Some(DeviceType::Junos));
    }
}
