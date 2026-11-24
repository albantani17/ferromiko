//! Minimal RFC 854 option handling: accept server echo and suppress-go-ahead, announce the
//! window size, refuse everything else.

const IAC: u8 = 255;
const DONT: u8 = 254;
const DO: u8 = 253;
const WONT: u8 = 252;
const WILL: u8 = 251;
const SB: u8 = 250;
const SE: u8 = 240;
const OPTION_ECHO: u8 = 1;
const OPTION_SUPPRESS_GO_AHEAD: u8 = 3;
const OPTION_WINDOW_SIZE: u8 = 31;
// Same window netmiko announces to ZTE.
const WINDOW_WIDTH: u16 = 500;
const WINDOW_HEIGHT: u16 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Data,
    Iac,
    Negotiation(u8),
    Subnegotiation,
    SubnegotiationIac,
}

/// Separates terminal data from telnet commands; keeps state across reads so a command
/// split between two TCP packets is still recognized.
#[derive(Debug)]
pub struct TelnetDecoder {
    state: State,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Decoded {
    pub data: Vec<u8>,
    /// Negotiation answers that must be written back to the server.
    pub replies: Vec<u8>,
}

impl TelnetDecoder {
    pub fn new() -> Self {
        Self { state: State::Data }
    }

    pub fn decode(&mut self, input: &[u8]) -> Decoded {
        let mut decoded = Decoded::default();
        for &byte in input {
            self.state = match (self.state, byte) {
                (State::Data, IAC) => State::Iac,
                // NVT sends a bare CR as "\r\0"; the NUL is padding.
                (State::Data, 0) => State::Data,
                (State::Data, _) => {
                    decoded.data.push(byte);
                    State::Data
                }
                (State::Iac, IAC) => {
                    decoded.data.push(IAC);
                    State::Data
                }
                (State::Iac, WILL | WONT | DO | DONT) => State::Negotiation(byte),
                (State::Iac, SB) => State::Subnegotiation,
                (State::Iac, _) => State::Data,
                (State::Negotiation(command), option) => {
                    decoded.replies.extend(negotiation_reply(command, option));
                    State::Data
                }
                (State::Subnegotiation, IAC) => State::SubnegotiationIac,
                (State::Subnegotiation, _) => State::Subnegotiation,
                (State::SubnegotiationIac, SE) => State::Data,
                (State::SubnegotiationIac, _) => State::Subnegotiation,
            };
        }
        decoded
    }
}

// shortcut: WONT/DONT are not acknowledged (no per-option state kept), fine for CLI servers that negotiate once.
fn negotiation_reply(command: u8, option: u8) -> Vec<u8> {
    match (command, option) {
        (DO, OPTION_WINDOW_SIZE) => {
            let [width_high, width_low] = WINDOW_WIDTH.to_be_bytes();
            let [height_high, height_low] = WINDOW_HEIGHT.to_be_bytes();
            vec![
                IAC, WILL, OPTION_WINDOW_SIZE,
                IAC, SB, OPTION_WINDOW_SIZE, width_high, width_low, height_high, height_low, IAC, SE,
            ]
        }
        (DO, OPTION_SUPPRESS_GO_AHEAD) => vec![IAC, WILL, option],
        (DO, _) => vec![IAC, WONT, option],
        (WILL, OPTION_ECHO | OPTION_SUPPRESS_GO_AHEAD) => vec![IAC, DO, option],
        (WILL, _) => vec![IAC, DONT, option],
        _ => Vec::new(),
    }
}

/// Doubles 0xFF so data bytes are not read as telnet commands.
pub fn escape_iac(data: &[u8]) -> Vec<u8> {
    let mut escaped = Vec::with_capacity(data.len());
    for &byte in data {
        escaped.push(byte);
        if byte == IAC {
            escaped.push(IAC);
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(input: &[u8]) -> Decoded {
        TelnetDecoder::new().decode(input)
    }

    #[test]
    fn passes_plain_text_through() {
        assert_eq!(decode(b"ZXAN#"), Decoded { data: b"ZXAN#".to_vec(), replies: vec![] });
    }

    #[test]
    fn unescapes_doubled_iac_as_data() {
        assert_eq!(decode(&[b'a', IAC, IAC, b'b']).data, vec![b'a', IAC, b'b']);
    }

    #[test]
    fn answers_window_size_request_with_size() {
        let replies = decode(&[IAC, DO, OPTION_WINDOW_SIZE]).replies;
        assert_eq!(replies, vec![IAC, WILL, 31, IAC, SB, 31, 0x01, 0xF4, 0, 50, IAC, SE]);
    }

    #[test]
    fn accepts_server_echo_and_suppress_go_ahead() {
        let replies = decode(&[IAC, WILL, OPTION_ECHO, IAC, WILL, OPTION_SUPPRESS_GO_AHEAD]).replies;
        assert_eq!(replies, vec![IAC, DO, OPTION_ECHO, IAC, DO, OPTION_SUPPRESS_GO_AHEAD]);
    }

    #[test]
    fn refuses_unknown_options() {
        let terminal_type = 24;
        let replies = decode(&[IAC, DO, terminal_type, IAC, WILL, terminal_type]).replies;
        assert_eq!(replies, vec![IAC, WONT, terminal_type, IAC, DONT, terminal_type]);
    }

    #[test]
    fn ignores_wont_and_dont() {
        assert_eq!(decode(&[IAC, WONT, OPTION_ECHO, IAC, DONT, OPTION_ECHO]), Decoded::default());
    }

    #[test]
    fn skips_subnegotiation_payload() {
        assert_eq!(decode(&[IAC, SB, 24, 1, IAC, SE, b'o', b'k']).data, b"ok".to_vec());
    }

    #[test]
    fn handles_command_split_across_reads() {
        let mut decoder = TelnetDecoder::new();
        let first = decoder.decode(&[b'x', IAC]);
        let second = decoder.decode(&[WILL, OPTION_ECHO, b'y']);
        assert_eq!(first.data, b"x".to_vec());
        assert_eq!(second, Decoded { data: b"y".to_vec(), replies: vec![IAC, DO, OPTION_ECHO] });
    }

    #[test]
    fn drops_nul_padding_after_carriage_return() {
        assert_eq!(decode(b"a\r\0b").data, b"a\rb".to_vec());
    }

    #[test]
    fn escapes_iac_in_outgoing_data() {
        assert_eq!(escape_iac(&[b'a', IAC, b'b']), vec![b'a', IAC, IAC, b'b']);
    }

    #[test]
    fn escape_leaves_plain_text_unchanged() {
        assert_eq!(escape_iac(b"show card\r\n"), b"show card\r\n".to_vec());
    }
}
