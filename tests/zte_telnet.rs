//! End-to-end telnet test against an in-process fake ZTE OLT on 127.0.0.1.

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
struct FakeZte {
    stream: TcpStream,
    client_commands: Vec<Vec<u8>>,
}

impl FakeZte {
    fn send(&mut self, bytes: &[u8]) {
        self.stream.write_all(bytes).unwrap();
    }

    fn read_byte(&mut self) -> u8 {
        let mut byte = [0u8];
        self.stream.read_exact(&mut byte).unwrap();
        byte[0]
    }

    fn read_line(&mut self) -> String {
        let mut line = Vec::new();
        loop {
            match self.read_byte() {
                IAC => self.read_command(),
                b'\n' => break,
                byte => line.push(byte),
            }
        }
        String::from_utf8(line).unwrap().trim_end_matches('\r').to_string()
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

fn spawn_fake_zte(password: &'static str, session: fn(&mut FakeZte)) -> (u16, thread::JoinHandle<Vec<Vec<u8>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut device = FakeZte { stream, client_commands: Vec::new() };
        device.send(&[IAC, DO, WINDOW_SIZE, IAC, WILL, ECHO, IAC, WILL, SUPPRESS_GO_AHEAD]);
        device.send(b"\r\n******************\r\nUsername:");
        device.expect("admin", "admin\r\nPassword:");
        assert_eq!(device.read_line(), password);
        session(&mut device);
        device.client_commands
    });
    (port, server)
}

fn params(port: u16, password: &str) -> DeviceParams {
    let mut params = DeviceParams::new(DeviceType::ZteTelnet, "127.0.0.1", "admin", password);
    params.port = port;
    params.timeout = Duration::from_secs(3);
    params
}

#[test]
fn logs_in_and_runs_commands_over_telnet() {
    let (port, server) = spawn_fake_zte("zte123", |device| {
        device.send(b"\r\nZXAN#");
        device.expect("", "\r\nZXAN#");
        device.expect("terminal length 0", "terminal length 0\r\nZXAN#");
        device.expect("show card", "show card\r\nRack Shelf Slot\r\n1    1     2\r\nZXAN#");
        device.expect("write", "write\r\n.[OK]\r\nZXAN#");
    });

    let mut connection = Connection::connect(params(port, "zte123")).unwrap();
    assert_eq!(connection.send_command("show card").unwrap(), "Rack Shelf Slot\n1    1     2");
    assert_eq!(connection.save_config().unwrap(), ".[OK]");
    connection.disconnect();

    let client_commands = server.join().unwrap();
    assert!(client_commands.contains(&vec![WILL, WINDOW_SIZE]), "{client_commands:?}");
    assert!(client_commands.contains(&vec![SB, WINDOW_SIZE, 0x01, 0xF4, 0, 50, IAC, SE]), "{client_commands:?}");
    assert!(client_commands.contains(&vec![DO, ECHO]), "{client_commands:?}");
}

#[test]
fn reports_wrong_password_as_auth_error() {
    let (port, server) = spawn_fake_zte("wrong", |device| {
        device.send(b"\r\n%Error 20203: Incorrect password.\r\nUsername:");
    });

    let result = Connection::connect(params(port, "wrong"));
    assert!(matches!(result, Err(Error::Auth(_))));
    server.join().unwrap();
}
