# ferromiko

A Rust library for automating network device CLIs over SSH and Telnet, with an API modeled on [netmiko](https://github.com/ktbyers/netmiko). Send commands, enter enable/config mode, commit, and save the configuration without handling prompts, paging, or escape codes yourself.

The API is synchronous (blocking). To work with many devices at once, run one `Connection` per thread.

## Supported devices

| `DeviceType` | netmiko name | Protocol | Default port |
|---|---|---|---|
| `CiscoIos` | `cisco_ios`, `cisco_xe` | SSH | 22 |
| `Zte` | `zte_zxros` | SSH | 22 |
| `ZteTelnet` | `zte_zxros_telnet` | Telnet | 23 |
| `Junos` | `juniper_junos` | SSH | 22 |
| `Mikrotik` | `mikrotik_routeros` | SSH | 22 |
| `Linux` | `linux` | SSH | 22 |

netmiko names can be parsed directly: `"cisco_ios".parse::<DeviceType>()`.

## Installation

The crate is not on crates.io yet. Add it by path or git:

```toml
[dependencies]
ferromiko = { path = "../ferromiko" }
```

OpenSSL must be installed on the system (it is used by `libssh2` through the `ssh2` crate).

## Quick start

```rust,no_run
use ferromiko::{Connection, DeviceParams, DeviceType};

fn main() -> ferromiko::Result<()> {
    let mut params = DeviceParams::new(DeviceType::CiscoIos, "10.0.0.1", "admin", "password");
    params.secret = Some("enable-secret".to_string());

    let mut conn = Connection::connect(params)?;
    conn.enable()?;

    let version = conn.send_command("show version")?;
    println!("{version}");

    conn.send_config_set(&["interface Gi0/1", "description uplink"])?;
    conn.save_config()?;
    Ok(())
}
```

`send_command` waits for the prompt to come back, then returns the output without the command echo and without the prompt line.

### ZTE over Telnet

```rust,no_run
use ferromiko::{Connection, DeviceParams, DeviceType};

fn main() -> ferromiko::Result<()> {
    let params = DeviceParams::new(DeviceType::ZteTelnet, "10.0.0.5", "admin", "password");
    let mut conn = Connection::connect(params)?; // port 23, answers Username:/Password: automatically
    println!("{}", conn.send_command("show card")?);
    Ok(())
}
```

### Junos: configure, then commit

```rust,no_run
use ferromiko::{Connection, DeviceParams, DeviceType};

fn main() -> ferromiko::Result<()> {
    let params = DeviceParams::new(DeviceType::Junos, "10.0.0.2", "admin", "password");
    let mut conn = Connection::connect(params)?;
    conn.send_config_set(&["set system host-name srx2"])?; // stays in config mode
    conn.commit()?;                                        // commits, then exits config mode
    Ok(())
}
```

### Commands that ask for confirmation

```rust,no_run
use std::time::Duration;
use ferromiko::{Connection, DeviceParams, DeviceType};

fn main() -> ferromiko::Result<()> {
    let params = DeviceParams::new(DeviceType::CiscoIos, "10.0.0.1", "admin", "password");
    let mut conn = Connection::connect(params)?;

    // Wait for a specific regex instead of the prompt.
    conn.send_command_expect("copy running-config startup-config", r"\[startup-config\]\?")?;
    conn.send_command("")?;

    // Or send and read until the device has been silent for 2 seconds.
    conn.send_command_timing("reload", Duration::from_secs(2))?;
    Ok(())
}
```

## Connection options

`DeviceParams::new(device_type, host, username, password)` fills in the defaults. Every field is public and can be changed:

| Field | Default | Notes |
|---|---|---|
| `port` | 22, or 23 for Telnet | |
| `secret` | `None` | Secret for `enable()` (Cisco/ZTE) |
| `key_file` | `None` | SSH private key. When set, `password` is used as the key passphrase |
| `timeout` | 10 seconds | Used for the TCP connect, the handshake, and every wait for the prompt |

For SSH, authentication tries password first, then keyboard-interactive (some Cisco/ZTE images only offer the latter).

## API

| Method | What it does |
|---|---|
| `Connection::connect(params)` | Connect, log in, detect the prompt, disable paging |
| `find_prompt()` | Send Enter and return the prompt line (e.g. `R1#`) |
| `send_command(cmd)` | Send a command and wait for the prompt |
| `send_command_expect(cmd, regex)` | Send a command and wait for a regex |
| `send_command_timing(cmd, quiet)` | Send a command and read until the device is silent for `quiet` |
| `enable()` / `check_enable_mode()` | Privileged mode; does nothing on vendors without enable |
| `config_mode()` / `exit_config_mode()` / `check_config_mode()` | Configuration mode; does nothing on MikroTik and Linux |
| `send_config_set(&[cmd])` | Enter config, send every command, exit (Junos stays in config) |
| `commit()` | Junos only; fails with `Error::CommitFailed` unless the output says `commit complete` |
| `save_config()` | Cisco `write memory`, ZTE `write` |
| `disconnect()` | Close the session; dropping the `Connection` closes it too |

`Connection::from_transport(stream, device_type, secret, timeout)` accepts any stream that implements `Read + Write` (non-blocking, returning `WouldBlock` when no data is waiting). Use it for a custom transport or in tests.

## Per-vendor behavior

| Vendor | After login | Config enter / exit | Save |
|---|---|---|---|
| Cisco IOS | `terminal length 0`, `terminal width 511` | `configure terminal` / `end` | `write memory` |
| ZTE (SSH & Telnet) | `terminal length 0` | `configure terminal` / `end` | `write` |
| Junos | `cli` if the login lands in a `%` shell, then `set cli complete-on-space off`, `set cli screen-length 0`, `set cli screen-width 511` | `configure` / `exit configuration-mode` | `commit` |
| MikroTik | username sent with the `+ct511w4098h` suffix (colors off), Enter is `\r\n` | none | none |
| Linux | `export TERM=dumb` | none | none |

Telnet uses `\r\n` as Enter, accepts server echo, and announces a 500x50 window, the same as netmiko.

## Errors

Every method returns `ferromiko::Result<T>`. The `Error` variants worth handling:

- `Auth`: login rejected, wrong enable secret, or Telnet asked to log in again.
- `Timeout { pattern, output }`: the prompt or pattern did not appear within `timeout`. `output` holds what was received so far, which helps with debugging.
- `Unsupported`: for example `save_config()` on Linux or `commit()` on Cisco.
- `ModeChange`: could not enter or leave enable/config mode.
- `CommitFailed`: holds the Junos commit output.
- `Io`, `Ssh`, `Regex`, `UnknownDeviceType`.

## CLI example

```sh
FERROMIKO_PASS=secret cargo run --example show -- cisco_ios 10.0.0.1 admin "show version"
FERROMIKO_PASS=secret FERROMIKO_SECRET=enable cargo run --example show -- zte_zxros_telnet 10.0.0.5 admin "show card"
```

The password is read from the environment, not from the arguments, so it does not end up in shell history. `enable()` is only called when `FERROMIKO_SECRET` is set.

## Development

```sh
cargo test             # unit tests + end-to-end Telnet test against a fake ZTE server on localhost
cargo clippy --all-targets
```

The test against a real SSH server is ignored by default:

```sh
FERROMIKO_HOST=127.0.0.1 FERROMIKO_USER=$USER FERROMIKO_KEY=~/.ssh/id_ed25519 \
  [FERROMIKO_PORT=22] [FERROMIKO_PASS=...] cargo test --test linux_live -- --ignored
```

Code layout:

```text
src/domain/        pure rules: vendor profiles, prompt patterns, output cleanup
src/integrations/  SSH transport (ssh2) and Telnet transport (TCP + IAC negotiation)
src/services/      Connection, read loop, Telnet login
src/types/         DeviceParams, Transport trait
src/utils/         write helpers for non-blocking sockets
```

## Limitations

- Not yet tested against real Cisco, ZTE, Junos, or MikroTik devices. The SSH path has been tested against a Linux sshd, the Telnet path against a fake ZTE server.
- SSH host keys are not verified, so connections are open to MITM. Use it on a trusted management network.
- Telnet sends the password unencrypted, and is only available for ZTE.
- Legacy SSH algorithms (`diffie-hellman-group1-sha1`, `ssh-rsa`) needed by some older devices have not been verified.
- On Linux, a prompt that shows the current directory can make commands after `cd` time out.
- On Junos, `exit_config_mode()` answers "yes" when asked about uncommitted changes, so those changes are discarded (netmiko does the same).
- Not implemented: SCP, TextFSM parsing, SSH agent or jump host, async, session logging, device type auto-detection.

## License

MIT. See [LICENSE](LICENSE).
