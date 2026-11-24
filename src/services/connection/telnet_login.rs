use std::io::{Read, Write};
use std::time::Duration;

use regex::Regex;

use super::read_until::read_until;
use crate::domain::device::login_question::prompt_or_question_pattern;
use crate::domain::device::profile::Profile;
use crate::domain::prompt::login_prompts::{
    PASSWORD_PROMPT, USERNAME_PROMPT, ends_with_password_prompt, ends_with_username_prompt,
};
use crate::domain::prompt::prompt_pattern::any_prompt_pattern;
use crate::utils::would_block::write_all_retrying;
use crate::{Error, Result};

/// Answers the `Username:` / `Password:` prompts and returns what was read after them, once a
/// device prompt or a login question (e.g. Huawei "Change now?") appears.
/// Telnet has no protocol-level auth, so a repeated login prompt is the only failure signal.
pub fn telnet_login<T: Read + Write>(
    transport: &mut T,
    username: &str,
    password: &str,
    profile: &Profile,
    timeout: Duration,
) -> Result<String> {
    let ready = prompt_or_question_pattern(&any_prompt_pattern(profile.prompt_terminators), profile.login_questions)?;
    let login_step = Regex::new(&format!("(?:{USERNAME_PROMPT})|(?:{PASSWORD_PROMPT})|(?:{})", ready.as_str()))?;
    let mut output = read_until(transport, &login_step, timeout)?;
    if ends_with_username_prompt(&output) {
        send_line(transport, username, profile.enter)?;
        output = read_until(transport, &login_step, timeout)?;
    }
    if ends_with_password_prompt(&output) {
        send_line(transport, password, profile.telnet_password_enter)?;
        output = read_until(transport, &login_step, timeout)?;
    }
    if ends_with_username_prompt(&output) || ends_with_password_prompt(&output) {
        return Err(Error::Auth(format!("telnet login rejected for {username}")));
    }
    Ok(output)
}

fn send_line<T: Write>(transport: &mut T, text: &str, enter: &str) -> Result<()> {
    write_all_retrying(transport, format!("{text}{enter}").as_bytes())?;
    transport.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DeviceType;
    use crate::services::connection::fake_transport::FakeTransport;

    const TIMEOUT: Duration = Duration::from_secs(1);

    fn zte() -> Profile {
        DeviceType::ZteTelnet.profile()
    }

    #[test]
    fn sends_username_then_password_until_prompt() {
        let mut transport = FakeTransport::new("\r\nUsername:", &[("admin", "admin\r\nPassword:"), ("zte", "\r\nZXAN#")]);
        let output = telnet_login(&mut transport, "admin", "zte", &zte(), TIMEOUT).unwrap();
        assert!(output.ends_with("ZXAN#"));
    }

    #[test]
    fn handles_password_only_login() {
        let mut transport = FakeTransport::new("Password:", &[("zte", "\r\nZXAN#")]);
        telnet_login(&mut transport, "admin", "zte", &zte(), TIMEOUT).unwrap();
    }

    #[test]
    fn succeeds_without_prompts_when_device_needs_no_login() {
        let mut transport = FakeTransport::new("\r\nZXAN#", &[]);
        telnet_login(&mut transport, "admin", "zte", &zte(), TIMEOUT).unwrap();
    }

    #[test]
    fn rejects_when_device_asks_to_log_in_again() {
        let mut transport = FakeTransport::new(
            "Username:",
            &[("admin", "admin\r\nPassword:"), ("wrong", "\r\n%Error 20203: Incorrect password.\r\nUsername:")],
        );
        let result = telnet_login(&mut transport, "admin", "wrong", &zte(), TIMEOUT);
        assert!(matches!(result, Err(Error::Auth(_))));
    }

    #[test]
    fn times_out_when_no_prompt_arrives() {
        let mut transport = FakeTransport::new("", &[]);
        let result = telnet_login(&mut transport, "admin", "zte", &zte(), Duration::from_millis(50));
        assert!(matches!(result, Err(Error::Timeout { .. })));
    }

    #[test]
    fn stops_at_huawei_password_change_question() {
        let mut transport = FakeTransport::new(
            "Username:",
            &[("admin", "admin\r\nPassword:"), ("huawei", "\r\nThe password needs to be changed. Change now? [Y/N]:")],
        );
        let output = telnet_login(&mut transport, "admin", "huawei", &DeviceType::HuaweiTelnet.profile(), TIMEOUT).unwrap();
        assert!(output.ends_with("[Y/N]:"));
    }

    #[test]
    fn accepts_huawei_olt_user_name_prompts() {
        let mut transport = FakeTransport::new(
            "\r\n>>User name:",
            &[("admin", "admin\r\n>>User password:"), ("olt", "\r\n\r\nMA5800-X7>")],
        );
        telnet_login(&mut transport, "admin", "olt", &DeviceType::HuaweiOltTelnet.profile(), TIMEOUT).unwrap();
    }
}
