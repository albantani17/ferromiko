//! Needs a reachable sshd (with `scp` installed for the SCP test):
//! FERROMIKO_HOST=localhost FERROMIKO_USER=me FERROMIKO_PASS=... [FERROMIKO_PORT=22] [FERROMIKO_KEY=~/.ssh/id_ed25519] [FERROMIKO_LEGACY=1] cargo test -- --ignored

use std::{env, fs, process};

use ferromiko::{Connection, DeviceParams, DeviceType, Error, scp_download, scp_upload};

fn live_params() -> DeviceParams {
    let var = |name: &str| env::var(name).unwrap_or_else(|_| panic!("{name} is not set"));
    let password = env::var("FERROMIKO_PASS").unwrap_or_default();
    let mut params = DeviceParams::new(DeviceType::Linux, &var("FERROMIKO_HOST"), &var("FERROMIKO_USER"), &password);
    params.key_file = env::var("FERROMIKO_KEY").ok().map(Into::into);
    params.legacy_ssh_algorithms = env::var("FERROMIKO_LEGACY").is_ok();
    if let Ok(port) = env::var("FERROMIKO_PORT") {
        params.port = port.parse().expect("FERROMIKO_PORT must be a number");
    }
    params
}

#[test]
#[ignore = "needs a live SSH server"]
fn runs_a_command_on_a_linux_host() {
    let mut connection = Connection::connect(live_params()).unwrap();
    assert_eq!(connection.send_command("echo ferromiko-ok").unwrap(), "ferromiko-ok");
}

#[test]
#[ignore = "needs a live SSH server on this machine"]
fn scp_round_trips_a_file_through_a_linux_host() {
    let params = live_params();
    let dir = env::temp_dir().join(format!("ferromiko-scp-{}", process::id()));
    fs::create_dir_all(&dir).unwrap();
    // Larger than the SSH window, and full of the bytes the SCP framing uses (\0, \x01, \x02, \n).
    let original: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
    let (local, copied) = (dir.join("upload.bin"), dir.join("download.bin"));
    fs::write(&local, &original).unwrap();
    let remote = dir.join("remote copy.bin");

    let uploaded = scp_upload(&params, &local, remote.to_str().unwrap());
    let downloaded = scp_download(&params, remote.to_str().unwrap(), &copied);
    let missing = scp_download(&params, "/nonexistent/ferromiko.bin", &dir.join("missing.bin"));
    let unwritable = scp_upload(&params, &local, "/nonexistent/ferromiko.bin");
    let round_tripped = fs::read(&copied);
    fs::remove_dir_all(&dir).unwrap();

    uploaded.unwrap();
    downloaded.unwrap();
    assert!(round_tripped.unwrap() == original, "downloaded bytes differ from the uploaded file");
    assert!(matches!(missing, Err(Error::Scp(message)) if message.contains("No such file")));
    assert!(matches!(unwritable, Err(Error::Scp(ref message)) if message.contains("No such file")), "{unwritable:?}");
}
