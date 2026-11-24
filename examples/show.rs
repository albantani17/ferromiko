//! Usage: FERROMIKO_PASS=... [FERROMIKO_SECRET=...] cargo run --example show -- <device_type> <host> <username> "<command>"
//! device_type: cisco_ios | zte_zxros | zte_zxros_telnet | juniper_junos | mikrotik_routeros | linux

use std::env;

use ferromiko::{Connection, DeviceParams, DeviceType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let [device_type, host, username, command] = args.as_slice() else {
        return Err("usage: show <device_type> <host> <username> <command>".into());
    };
    let password = env::var("FERROMIKO_PASS").map_err(|_| "set FERROMIKO_PASS")?;

    let mut params = DeviceParams::new(device_type.parse::<DeviceType>()?, host, username, &password);
    params.secret = env::var("FERROMIKO_SECRET").ok();
    let has_secret = params.secret.is_some();

    let mut connection = Connection::connect(params)?;
    if has_secret {
        connection.enable()?;
    }
    println!("{}", connection.send_command(command)?);
    Ok(())
}
