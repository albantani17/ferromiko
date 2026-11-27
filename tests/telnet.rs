//! End-to-end telnet tests against in-process fake devices on 127.0.0.1.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use ferromiko::{Connection, DeviceParams, DeviceType, Error};

const IAC: u8 = 255;
const WILL: u8 = 251;
const DO: u8 = 253;
const SB: u8 = 250;
const SE: u8 = 240;
const ECHO: u8 = 1;
const SUPPRESS_GO_AHEAD: u8 = 3;
const WINDOW_SIZE: u8 = 31;

/// Server side of the fake device: strips the client's telnet commands and remembers them.
struct FakeDevice {
    stream: TcpStream,
    client_commands: Vec<Vec<u8>>,
    skip_next_lf: bool,
}

impl FakeDevice {
    fn send(&mut self, bytes: &[u8]) {
        self.stream.write_all(bytes).unwrap();
    }

    fn read_byte(&mut self) -> u8 {
        let mut byte = [0u8];
        self.stream.read_exact(&mut byte).unwrap();
        byte[0]
    }

    /// A line ends at CR, LF or CRLF; Huawei clients send a bare CR after the password.
    fn read_line(&mut self) -> String {
        let mut line = Vec::new();
        loop {
            match self.read_byte() {
                IAC => self.read_command(),
                b'\n' if std::mem::take(&mut self.skip_next_lf) && line.is_empty() => {}
                b'\r' => {
                    self.skip_next_lf = true;
                    break;
                }
                b'\n' => break,
                byte => line.push(byte),
            }
        }
        String::from_utf8(line).unwrap()
    }

    fn read_command(&mut self) {
        let mut command = vec![self.read_byte()];
        if command[0] == SB {
            while !command.ends_with(&[IAC, SE]) {
                command.push(self.read_byte());
            }
        } else {
            command.push(self.read_byte());
        }
        self.client_commands.push(command);
    }

    fn expect(&mut self, command: &str, reply: &str) {
        assert_eq!(self.read_line(), command);
        self.send(reply.as_bytes());
    }
}

struct LoginPrompts {
    username: &'static str,
    password: &'static str,
}

const ZTE_LOGIN: LoginPrompts = LoginPrompts { username: "\r\n******************\r\nUsername:", password: "Password:" };

fn spawn_fake_device(
    prompts: LoginPrompts,
    password: &'static str,
    session: fn(&mut FakeDevice),
) -> (u16, thread::JoinHandle<Vec<Vec<u8>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut device = FakeDevice { stream, client_commands: Vec::new(), skip_next_lf: false };
        device.send(&[IAC, DO, WINDOW_SIZE, IAC, WILL, ECHO, IAC, WILL, SUPPRESS_GO_AHEAD]);
        device.send(prompts.username.as_bytes());
        device.expect("admin", &format!("admin\r\n{}", prompts.password));
        assert_eq!(device.read_line(), password);
        session(&mut device);
        // Closing with unread client bytes would send a TCP reset and drop our last reply.
        let _ = device.stream.read_to_end(&mut Vec::new());
        device.client_commands
    });
    (port, server)
}

fn params(device_type: DeviceType, port: u16, password: &str) -> DeviceParams {
    let mut params = DeviceParams::new(device_type, "127.0.0.1", "admin", password);
    params.port = port;
    params.timeout = Duration::from_secs(3);
    params
}

#[test]
fn zte_logs_in_and_runs_commands_over_telnet() {
    let (port, server) = spawn_fake_device(ZTE_LOGIN, "zte123", |device| {
        device.send(b"\r\nZXAN#");
        device.expect("", "\r\nZXAN#");
        device.expect("terminal length 0", "terminal length 0\r\nZXAN#");
        device.expect("show card", "show card\r\nRack Shelf Slot\r\n1    1     2\r\nZXAN#");
        device.expect("write", "write\r\n.[OK]\r\nZXAN#");
    });

    let mut connection = Connection::connect(params(DeviceType::ZteTelnet, port, "zte123")).unwrap();
    assert_eq!(connection.send_command("show card").unwrap(), "Rack Shelf Slot\n1    1     2");
    assert_eq!(connection.save_config().unwrap(), ".[OK]");
    connection.disconnect();

    let client_commands = server.join().unwrap();
    assert!(client_commands.contains(&vec![WILL, WINDOW_SIZE]), "{client_commands:?}");
    assert!(client_commands.contains(&vec![SB, WINDOW_SIZE, 0x01, 0xF4, 0, 50, IAC, SE]), "{client_commands:?}");
    assert!(client_commands.contains(&vec![DO, ECHO]), "{client_commands:?}");
}

#[test]
fn zte_reports_wrong_password_as_auth_error() {
    let (port, server) = spawn_fake_device(ZTE_LOGIN, "wrong", |device| {
        device.send(b"\r\n%Error 20203: Incorrect password.\r\nUsername:");
    });

    let result = Connection::connect(params(DeviceType::ZteTelnet, port, "wrong"));
    assert!(matches!(result, Err(Error::Auth(_))));
    server.join().unwrap();
}

#[test]
fn huawei_vrp_declines_password_change_and_saves_over_telnet() {
    let prompts = LoginPrompts { username: "\r\nLogin authentication\r\n\r\nUsername:", password: "Password:" };
    let (port, server) = spawn_fake_device(prompts, "huawei123", |device| {
        device.send(b"\r\nThe password needs to be changed. Change now? [Y/N]:");
        device.expect("N", "N\r\nInfo: The max number of VTY users is 5.\r\n<HUAWEI>");
        device.expect("", "\r\n<HUAWEI>");
        device.expect("screen-length 0 temporary", "screen-length 0 temporary\r\n<HUAWEI>");
        device.expect("save", "save\r\n  Are you sure to continue?[Y/N]");
        device.expect("y", "y\r\n  Save the configuration successfully.\r\n<HUAWEI>");
    });

    let mut connection = Connection::connect(params(DeviceType::HuaweiTelnet, port, "huawei123")).unwrap();
    assert!(connection.save_config().unwrap().contains("successfully"));
    connection.disconnect();
    server.join().unwrap();
}

#[test]
fn huawei_olt_logs_in_and_enables_over_telnet() {
    let prompts = LoginPrompts { username: "\r\n>>User name:", password: ">>User password:" };
    let (port, server) = spawn_fake_device(prompts, "olt123", |device| {
        device.send(b"\r\n\r\nMA5800-X7>");
        device.expect("", "\r\nMA5800-X7>");
        device.expect("", "\r\nMA5800-X7>");
        device.expect("enable", "enable\r\n\r\nMA5800-X7#");
        device.expect("", "\r\nMA5800-X7#");
        device.expect("undo smart", "undo smart\r\n\r\nMA5800-X7#");
        device.expect("infoswitch cli OFF", "infoswitch cli OFF\r\n\r\nMA5800-X7#");
        device.expect("scroll", "scroll\r\n\r\nMA5800-X7#");
        device.expect("display ont info 0 1 0 all", "display ont info 0 1 0 all\r\n  F/S/P   ONT-ID\r\n  0/1/0   1\r\nMA5800-X7#");
    });

    let mut connection = Connection::connect(params(DeviceType::HuaweiOltTelnet, port, "olt123")).unwrap();
    assert_eq!(connection.send_command("display ont info 0 1 0 all").unwrap(), "  F/S/P   ONT-ID\n  0/1/0   1");
    connection.disconnect();
    server.join().unwrap();
}

#[test]
fn session_log_records_telnet_login_with_password_masked() {
    let (port, server) = spawn_fake_device(ZTE_LOGIN, "zte123", |device| {
        device.send(b"zte123\r\nZXAN#");
        device.expect("", "\r\nZXAN#");
        device.expect("terminal length 0", "terminal length 0\r\nZXAN#");
    });
    let log_path = std::env::temp_dir().join(format!("ferromiko-telnet-session-{}.log", std::process::id()));
    let mut device = params(DeviceType::ZteTelnet, port, "zte123");
    device.session_log = Some(log_path.clone());

    Connection::connect(device).unwrap().disconnect();
    server.join().unwrap();

    let log = std::fs::read_to_string(&log_path).unwrap();
    std::fs::remove_file(&log_path).unwrap();
    assert!(log.contains("Username:"), "{log:?}");
    assert!(log.contains("********\r\nZXAN#"), "{log:?}");
    assert!(!log.contains("zte123"), "{log:?}");
}
