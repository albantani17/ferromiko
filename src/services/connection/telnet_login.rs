use std::io::{Read, Write};
use std::time::Duration;

use regex::Regex;

use super::read_until::read_until;
use crate::domain::prompt::login_prompts::{
    PASSWORD_PROMPT, USERNAME_PROMPT, ends_with_password_prompt, ends_with_username_prompt,
};
use crate::domain::prompt::prompt_pattern::any_prompt_pattern;
use crate::utils::would_block::write_all_retrying;
use crate::{Error, Result};

/// Answers the `Username:` / `Password:` prompts and returns once the device prompt appears.
/// Telnet has no protocol-level auth, so a repeated login prompt is the only failure signal.
pub fn telnet_login<T: Read + Write>(
    transport: &mut T,
    username: &str,
    password: &str,
    enter: &str,
    timeout: Duration,
) -> Result<()> {
    let login_step = Regex::new(&format!(
        "(?:{USERNAME_PROMPT})|(?:{PASSWORD_PROMPT})|(?:{})",
        any_prompt_pattern().as_str()
    ))?;
    let mut output = read_until(transport, &login_step, timeout)?;
    if ends_with_username_prompt(&output) {
        send_line(transport, username, enter)?;
        output = read_until(transport, &login_step, timeout)?;
    }
    if ends_with_password_prompt(&output) {
        send_line(transport, password, enter)?;
        output = read_until(transport, &login_step, timeout)?;
    }
    if ends_with_username_prompt(&output) || ends_with_password_prompt(&output) {
        return Err(Error::Auth(format!("telnet login rejected for {username}")));
    }
    Ok(())
}

fn send_line<T: Write>(transport: &mut T, text: &str, enter: &str) -> Result<()> {
    write_all_retrying(transport, format!("{text}{enter}").as_bytes())?;
    transport.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::connection::fake_transport::FakeTransport;

    const TIMEOUT: Duration = Duration::from_secs(1);

    #[test]
    fn sends_username_then_password_until_prompt() {
        let mut transport = FakeTransport::new("\r\nUsername:", &[("admin", "admin\r\nPassword:"), ("zte", "\r\nZXAN#")]);
        telnet_login(&mut transport, "admin", "zte", "\r\n", TIMEOUT).unwrap();
    }

    #[test]
    fn handles_password_only_login() {
        let mut transport = FakeTransport::new("Password:", &[("zte", "\r\nZXAN#")]);
        telnet_login(&mut transport, "admin", "zte", "\r\n", TIMEOUT).unwrap();
    }

    #[test]
    fn succeeds_without_prompts_when_device_needs_no_login() {
        let mut transport = FakeTransport::new("\r\nZXAN#", &[]);
        telnet_login(&mut transport, "admin", "zte", "\r\n", TIMEOUT).unwrap();
    }

    #[test]
    fn rejects_when_device_asks_to_log_in_again() {
        let mut transport = FakeTransport::new(
            "Username:",
            &[("admin", "admin\r\nPassword:"), ("wrong", "\r\n%Error 20203: Incorrect password.\r\nUsername:")],
        );
        let result = telnet_login(&mut transport, "admin", "wrong", "\r\n", TIMEOUT);
        assert!(matches!(result, Err(Error::Auth(_))));
    }

    #[test]
    fn times_out_when_no_prompt_arrives() {
        let mut transport = FakeTransport::new("", &[]);
        let result = telnet_login(&mut transport, "admin", "zte", "\r\n", Duration::from_millis(50));
        assert!(matches!(result, Err(Error::Timeout { .. })));
    }
}
