//! Needs a reachable sshd:
//! FERROMIKO_HOST=localhost FERROMIKO_USER=me FERROMIKO_PASS=... [FERROMIKO_PORT=22] [FERROMIKO_KEY=~/.ssh/id_ed25519] cargo test -- --ignored

use std::env;

use ferromiko::{Connection, DeviceParams, DeviceType};

#[test]
#[ignore = "needs a live SSH server"]
fn runs_a_command_on_a_linux_host() {
    let var = |name: &str| env::var(name).unwrap_or_else(|_| panic!("{name} is not set"));
    let password = env::var("FERROMIKO_PASS").unwrap_or_default();
    let mut params = DeviceParams::new(DeviceType::Linux, &var("FERROMIKO_HOST"), &var("FERROMIKO_USER"), &password);
    params.key_file = env::var("FERROMIKO_KEY").ok().map(Into::into);
    if let Ok(port) = env::var("FERROMIKO_PORT") {
        params.port = port.parse().expect("FERROMIKO_PORT must be a number");
    }
    let mut connection = Connection::connect(params).unwrap();
    assert_eq!(connection.send_command("echo ferromiko-ok").unwrap(), "ferromiko-ok");
}
