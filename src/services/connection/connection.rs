use std::io::{Read, Write};
use std::sync::LazyLock;
use std::time::Duration;

use regex::{Regex, RegexBuilder};

use super::read_until::{drain, read_until, read_until_quiet};
use super::telnet_login::telnet_login;
use crate::domain::device::login_question::{answer_login_question, prompt_or_question_pattern};
use crate::domain::device::profile::{Profile, Protocol};
use crate::domain::output::clean_output::clean_output;
use crate::domain::prompt::base_prompt::base_prompt;
use crate::domain::prompt::login_prompts::{PASSWORD_PROMPT, ends_with_password_prompt};
use crate::domain::prompt::prompt_pattern::{any_prompt_pattern, prompt_pattern};
use crate::{DeviceParams, DeviceType, Error, Result, SshTransport, TelnetTransport, Transport};

const LOGIN_QUIET: Duration = Duration::from_millis(300);
const MAX_LOGIN_QUESTIONS: usize = 3;
const SAVE_TIMEOUT: Duration = Duration::from_secs(120);
const COMMIT_TIMEOUT: Duration = Duration::from_secs(120);
const ENABLE_PROMPT_TERMINATOR: char = '#';
const SHELL_PROMPT_TERMINATOR: char = '%';
const UNCOMMITTED_CHANGES_QUESTION: &str = r"\[yes,no\][^\n]*\z";
const COMMIT_SUCCESS: &str = "commit complete";

static UNCOMMITTED_CHANGES_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(UNCOMMITTED_CHANGES_QUESTION).expect("static question regex is valid"));

/// An interactive CLI session, the equivalent of a netmiko connection object.
pub struct Connection<T: Read + Write = Box<dyn Transport>> {
    transport: T,
    profile: Profile,
    prompt: Regex,
    any_prompt: Regex,
    secret: Option<String>,
    read_timeout: Duration,
}

impl Connection<Box<dyn Transport>> {
    /// Opens SSH or Telnet (per device type), logs in and prepares the session
    /// (prompt detection, paging off).
    pub fn connect(params: DeviceParams) -> Result<Self> {
        let profile = params.device_type.profile();
        match profile.protocol {
            Protocol::Ssh => {
                let transport = SshTransport::open(&params, profile.username_suffix)?;
                Connection::start(Box::new(transport), params.device_type, params.secret, params.timeout, None)
            }
            Protocol::Telnet => {
                let mut transport = TelnetTransport::open(&params.host, params.port, params.timeout)?;
                let login_output =
                    telnet_login(&mut transport, &params.username, &params.password, &profile, params.timeout)?;
                let transport: Box<dyn Transport> = Box::new(transport);
                Connection::start(transport, params.device_type, params.secret, params.timeout, Some(login_output))
            }
        }
    }
}

impl<T: Read + Write> Connection<T> {
    /// Builds a connection over any already-open shell stream.
    pub fn from_transport(
        transport: T,
        device_type: DeviceType,
        secret: Option<String>,
        read_timeout: Duration,
    ) -> Result<Self> {
        Self::start(transport, device_type, secret, read_timeout, None)
    }

    /// `login_output` is what a Telnet login already read; `None` reads the banner here.
    fn start(
        transport: T,
        device_type: DeviceType,
        secret: Option<String>,
        read_timeout: Duration,
        login_output: Option<String>,
    ) -> Result<Self> {
        let profile = device_type.profile();
        let any_prompt = any_prompt_pattern(profile.prompt_terminators);
        let mut connection = Self {
            transport,
            profile,
            prompt: any_prompt.clone(),
            any_prompt,
            secret,
            read_timeout,
        };
        connection.prepare_session(login_output)?;
        Ok(connection)
    }

    fn prepare_session(&mut self, login_output: Option<String>) -> Result<()> {
        let prompt_or_question = prompt_or_question_pattern(&self.any_prompt, self.profile.login_questions)?;
        let mut output = match login_output {
            Some(output) => output,
            // Login banners are optional; a device that stays silent until Enter is fine.
            None => match read_until(&mut self.transport, &prompt_or_question, self.read_timeout) {
                Ok(output) | Err(Error::Timeout { output, .. }) => output,
                Err(error) => return Err(error),
            },
        };
        for _ in 0..MAX_LOGIN_QUESTIONS {
            let Some(answer) = answer_login_question(self.profile.login_questions, &output)? else {
                break;
            };
            self.write_line(answer)?;
            output = read_until(&mut self.transport, &prompt_or_question, self.read_timeout)?;
        }
        // A banner line like "#####" also looks like a prompt; wait for silence so the real
        // prompt is consumed here and not mistaken for the reply to a later command.
        read_until_quiet(&mut self.transport, LOGIN_QUIET, self.read_timeout)?;
        let mut prompt = self.find_prompt()?;
        if let Some(cli_command) = self.profile.shell_to_cli_command
            && prompt.ends_with(SHELL_PROMPT_TERMINATOR)
        {
            self.write_line(cli_command)?;
            read_until(&mut self.transport, &self.any_prompt, self.read_timeout)?;
            prompt = self.find_prompt()?;
        }
        self.prompt = prompt_pattern(&base_prompt(&prompt), self.profile.prompt_terminators);
        if self.profile.enable_on_login {
            self.enable()?;
        }
        for command in self.profile.paging_commands {
            self.send_command(command)?;
        }
        Ok(())
    }

    /// Sends Enter and returns the prompt line, e.g. `R1#`.
    pub fn find_prompt(&mut self) -> Result<String> {
        drain(&mut self.transport)?;
        self.write_line("")?;
        let output = read_until(&mut self.transport, &self.any_prompt, self.read_timeout)?;
        let prompt = output.lines().rev().map(str::trim).find(|line| !line.is_empty());
        Ok(prompt.unwrap_or_default().to_string())
    }

    /// Sends a command and waits for the prompt. Returns output without echo and prompt.
    pub fn send_command(&mut self, command: &str) -> Result<String> {
        let prompt = self.prompt.clone();
        let raw = self.send_and_read(command, &prompt, self.read_timeout)?;
        Ok(clean_output(&raw, command, Some(&prompt)))
    }

    /// Sends a command and waits for `pattern` (a regex) instead of the prompt.
    pub fn send_command_expect(&mut self, command: &str, pattern: &str) -> Result<String> {
        let expected = Regex::new(pattern)?;
        let raw = self.send_and_read(command, &expected, self.read_timeout)?;
        Ok(clean_output(&raw, command, Some(&self.prompt)))
    }

    /// Sends a command and returns whatever arrived until the device was silent for `quiet`.
    pub fn send_command_timing(&mut self, command: &str, quiet: Duration) -> Result<String> {
        drain(&mut self.transport)?;
        self.write_line(command)?;
        let raw = read_until_quiet(&mut self.transport, quiet, self.read_timeout.max(quiet))?;
        Ok(clean_output(&raw, command, Some(&self.prompt)))
    }

    /// True when privileged (`#`), or when the vendor has no enable mode.
    pub fn check_enable_mode(&mut self) -> Result<bool> {
        if self.profile.enable_command.is_none() {
            return Ok(true);
        }
        Ok(self.find_prompt()?.ends_with(ENABLE_PROMPT_TERMINATOR))
    }

    /// Enters privileged mode using the configured secret. No-op for vendors without it.
    pub fn enable(&mut self) -> Result<()> {
        let Some(enable_command) = self.profile.enable_command else {
            return Ok(());
        };
        if self.check_enable_mode()? {
            return Ok(());
        }
        let password_or_prompt = Regex::new(&format!("(?:{PASSWORD_PROMPT})|(?:{})", self.prompt.as_str()))?;
        let output = self.send_and_read(enable_command, &password_or_prompt, self.read_timeout)?;
        if ends_with_password_prompt(&output) {
            let secret = self.secret.clone().unwrap_or_default();
            self.write_line(&secret)?;
            let after_secret = read_until(&mut self.transport, &password_or_prompt, self.read_timeout)?;
            if ends_with_password_prompt(&after_secret) {
                return Err(Error::Auth("enable secret rejected".to_string()));
            }
        }
        if !self.check_enable_mode()? {
            return Err(Error::ModeChange(format!("could not enter enable mode:\n{output}")));
        }
        Ok(())
    }

    /// False for vendors without a configuration mode.
    pub fn check_config_mode(&mut self) -> Result<bool> {
        let Some(marker) = self.profile.config_prompt_marker else {
            return Ok(false);
        };
        Ok(self.find_prompt()?.contains(marker))
    }

    /// Enters configuration mode. No-op for MikroTik and Linux.
    pub fn config_mode(&mut self) -> Result<String> {
        let Some(enter_command) = self.profile.config_enter_command else {
            return Ok(String::new());
        };
        if self.check_config_mode()? {
            return Ok(String::new());
        }
        let output = self.send_command(enter_command)?;
        if !self.check_config_mode()? {
            return Err(Error::ModeChange(format!("could not enter config mode:\n{output}")));
        }
        Ok(output)
    }

    /// Leaves configuration mode. On Junos, uncommitted changes are discarded (as netmiko does).
    pub fn exit_config_mode(&mut self) -> Result<String> {
        let Some(exit_command) = self.profile.config_exit_command else {
            return Ok(String::new());
        };
        if !self.check_config_mode()? {
            return Ok(String::new());
        }
        let prompt_or_question =
            Regex::new(&format!("(?:{UNCOMMITTED_CHANGES_QUESTION})|(?:{})", self.prompt.as_str()))?;
        let raw = self.send_and_read(exit_command, &prompt_or_question, self.read_timeout)?;
        let mut output = clean_output(&raw, exit_command, Some(&self.prompt));
        if UNCOMMITTED_CHANGES_REGEX.is_match(&raw) {
            output.push_str(&self.send_command("yes")?);
        }
        if self.check_config_mode()? {
            return Err(Error::ModeChange(format!("could not exit config mode:\n{output}")));
        }
        Ok(output)
    }

    /// Enters config mode, sends each command, then exits.
    /// Vendors with a commit step (Junos) stay in config mode so `commit()` can follow.
    pub fn send_config_set(&mut self, commands: &[&str]) -> Result<String> {
        self.send_config_commands(commands, None)
    }

    /// Like `send_config_set`, but stops at the first command whose output matches
    /// `error_pattern` (multi-line regex, as netmiko's `error_pattern`) and returns
    /// `Error::ConfigInvalid`. Like netmiko, the session is left in config mode on error.
    pub fn send_config_set_checked(&mut self, commands: &[&str], error_pattern: &str) -> Result<String> {
        let error_regex = RegexBuilder::new(error_pattern).multi_line(true).build()?;
        self.send_config_commands(commands, Some(&error_regex))
    }

    fn send_config_commands(&mut self, commands: &[&str], error_regex: Option<&Regex>) -> Result<String> {
        let mut output = self.config_mode()?;
        for command in commands {
            let command_output = self.send_command(command)?;
            if error_regex.is_some_and(|regex| regex.is_match(&command_output)) {
                return Err(Error::ConfigInvalid { command: command.to_string(), output: command_output });
            }
            output.push_str(&command_output);
            output.push('\n');
        }
        if self.profile.commit_command.is_none() {
            output.push_str(&self.exit_config_mode()?);
        }
        Ok(output)
    }

    /// Commits the candidate configuration and leaves config mode (Junos).
    pub fn commit(&mut self) -> Result<String> {
        let commit_command = self.profile.commit_command.ok_or(Error::Unsupported("commit"))?;
        let mut output = self.config_mode()?;
        let prompt = self.prompt.clone();
        let raw = self.send_and_read(commit_command, &prompt, COMMIT_TIMEOUT)?;
        let commit_output = clean_output(&raw, commit_command, Some(&prompt));
        if !commit_output.contains(COMMIT_SUCCESS) {
            return Err(Error::CommitFailed(commit_output));
        }
        output.push_str(&commit_output);
        output.push_str(&self.exit_config_mode()?);
        Ok(output)
    }

    /// Saves the configuration (Cisco `write memory`, ZTE `write`, Huawei `save`),
    /// answering the vendor's confirmation question when it asks one.
    pub fn save_config(&mut self) -> Result<String> {
        let save_command = self.profile.save_command.ok_or(Error::Unsupported("save_config"))?;
        let prompt = self.prompt.clone();
        let Some((question, answer)) = self.profile.save_confirmation else {
            let raw = self.send_and_read(save_command, &prompt, SAVE_TIMEOUT)?;
            return Ok(clean_output(&raw, save_command, Some(&prompt)));
        };
        let question = Regex::new(question)?;
        let prompt_or_question = Regex::new(&format!("(?:{})|(?:{})", question.as_str(), prompt.as_str()))?;
        let mut raw = self.send_and_read(save_command, &prompt_or_question, SAVE_TIMEOUT)?;
        if question.is_match(&raw) {
            self.write_line(answer)?;
            raw.push_str(&read_until(&mut self.transport, &prompt, SAVE_TIMEOUT)?);
        }
        Ok(clean_output(&raw, save_command, Some(&prompt)))
    }

    /// Closes the session. Dropping the connection does the same.
    pub fn disconnect(self) {}

    fn send_and_read(&mut self, command: &str, pattern: &Regex, timeout: Duration) -> Result<String> {
        drain(&mut self.transport)?;
        self.write_line(command)?;
        read_until(&mut self.transport, pattern, timeout)
    }

    fn write_line(&mut self, command: &str) -> Result<()> {
        let line = format!("{command}{}", self.profile.enter);
        self.transport.write_all(line.as_bytes())?;
        self.transport.flush()?;
        Ok(())
    }
}
