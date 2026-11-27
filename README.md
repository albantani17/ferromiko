# ferromiko

A Rust library for automating network device CLIs over SSH and Telnet, with an API modeled on [netmiko](https://github.com/ktbyers/netmiko). Send commands, enter enable/config mode, commit, and save the configuration without handling prompts, paging, or escape codes yourself.

The API is synchronous (blocking). To work with many devices at once, run one `Connection` per thread.

## Supported devices

| `DeviceType` | netmiko name | Protocol | Default port |
|---|---|---|---|
| `CiscoIos` | `cisco_ios`, `cisco_xe` | SSH | 22 |
| `Zte` | `zte_zxros` | SSH | 22 |
| `ZteTelnet` | `zte_zxros_telnet` | Telnet | 23 |
| `Huawei` | `huawei`, `huawei_vrp` | SSH | 22 |
| `HuaweiTelnet` | `huawei_telnet` | Telnet | 23 |
| `HuaweiOlt` | `huawei_olt`, `huawei_smartax` | SSH | 22 |
| `HuaweiOltTelnet` | `huawei_olt_telnet` | Telnet | 23 |
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

SSH uses [russh](https://crates.io/crates/russh) (pure Rust protocol, no OpenSSL). Its crypto backend `aws-lc-rs` needs a C compiler and CMake at build time.

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

### Huawei OLT (MA5600/MA5800)

```rust,no_run
use ferromiko::{Connection, DeviceParams, DeviceType};

fn main() -> ferromiko::Result<()> {
    let params = DeviceParams::new(DeviceType::HuaweiOltTelnet, "10.0.0.6", "admin", "password");
    let mut conn = Connection::connect(params)?; // enters enable mode and disables paging on login
    println!("{}", conn.send_command("display ont info 0 1 0 all")?);
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

    // Several steps, each waiting for its own regex; an empty regex waits for the prompt.
    conn.send_multiline(&[("reload", r"\[confirm\]"), ("", "")])?;
    Ok(())
}
```

### Stop on config errors

```rust,no_run
use ferromiko::{Connection, DeviceParams, DeviceType, Error};

fn main() -> ferromiko::Result<()> {
    let params = DeviceParams::new(DeviceType::CiscoIos, "10.0.0.1", "admin", "password");
    let mut conn = Connection::connect(params)?;
    match conn.send_config_set_checked(&["interface Gi0/1", "descripton typo"], r"^% Invalid") {
        Err(Error::ConfigInvalid { command, output }) => eprintln!("{command} failed: {output}"),
        other => println!("{}", other?),
    }
    Ok(())
}
```

Like netmiko, it stops at the first failing command and leaves the session in config mode.

### Detect the device type

```rust,no_run
use ferromiko::{autodetect, DeviceParams, DeviceType};

fn main() -> ferromiko::Result<()> {
    // The device type given here is ignored while detecting.
    let mut params = DeviceParams::new(DeviceType::Linux, "10.0.0.1", "admin", "password");
    if let Some(device_type) = autodetect(&params)? {
        params.device_type = device_type;
        println!("detected {device_type:?}");
    }
    Ok(())
}
```

SSH only. It tries `show version`, `display version`, `uname -a` and `/system resource print`, using netmiko's patterns (the ZTE and MikroTik patterns are ferromiko's own).

### Copy files with SCP

```rust,no_run
use std::path::Path;
use ferromiko::{scp_download, scp_upload, DeviceParams, DeviceType};

fn main() -> ferromiko::Result<()> {
    let params = DeviceParams::new(DeviceType::CiscoIos, "10.0.0.1", "admin", "password");
    scp_upload(&params, Path::new("c2900.bin"), "flash:/c2900.bin")?;
    scp_download(&params, "flash:/config.text", Path::new("backup.cfg"))?;
    Ok(())
}
```

Each call opens its own SSH session. Single files only; the device must run an SCP server (Cisco: `ip scp server enable`).

### Parse output with TextFSM

Enable the optional `textfsm` feature to get netmiko's `use_textfsm`: the output is parsed with an [ntc-templates](https://github.com/networktocode/ntc-templates) template picked from its `index` by device type and command (abbreviations such as `sh ip int br` work too).

```toml
ferromiko = { path = "../ferromiko", features = ["textfsm"] }
```

```rust,no_run
# #[cfg(not(feature = "textfsm"))]
# fn main() {}
#[cfg(feature = "textfsm")]
fn main() -> ferromiko::Result<()> {
    use ferromiko::{ntc_templates_dir, Connection, DeviceParams, DeviceType, TextFsmOutput};

    let params = DeviceParams::new(DeviceType::CiscoIos, "10.0.0.1", "admin", "password");
    let mut conn = Connection::connect(params)?;
    // NET_TEXTFSM=~/ntc-templates/ntc_templates/templates, as for netmiko.
    let templates = ntc_templates_dir().expect("NET_TEXTFSM is not set");
    match conn.send_command_textfsm("show ip interface brief", &templates)? {
        TextFsmOutput::Parsed(rows) => rows.iter().for_each(|row| println!("{:?}", row["interface"])),
        TextFsmOutput::Raw(text) => println!("{text}"), // no template, or it found no rows
    }
    Ok(())
}
```

Each row is a `BTreeMap` from the lowercase value name to `FieldValue::Text` or, for `List` values, `FieldValue::List`. Templates are not bundled: clone ntc-templates and point `NET_TEXTFSM` (or any path) at its `templates` directory. Telnet device types use the same templates as SSH. `ferromiko::parse_textfsm(template_file, output)` parses with one template file, and `ferromiko::parse_ntc(dir, device_type, command, output)` does the index lookup on output you already have.

## Connection options

`DeviceParams::new(device_type, host, username, password)` fills in the defaults. Every field is public and can be changed:

| Field | Default | Notes |
|---|---|---|
| `port` | 22, or 23 for Telnet | |
| `secret` | `None` | Secret for `enable()` (Cisco/ZTE) |
| `key_file` | `None` | SSH private key. When set, `password` is used as the key passphrase |
| `timeout` | 10 seconds | Used for the TCP connect, the handshake, and every wait for the prompt |
| `session_log` | `None` | File that records everything read from the device; password and secret are masked as `********`. Writes are not recorded (netmiko's default) |
| `legacy_ssh_algorithms` | `false` | Also offer old algorithms after the modern ones: `diffie-hellman-group1/group14/group-exchange-sha1`, `ssh-dss`, `aes*-cbc`, `3des-cbc`, `hmac-sha1`. Turn it on for old routers and OLTs that reject the handshake |

For SSH, authentication tries password first, then keyboard-interactive (some Cisco/ZTE images only offer the latter).

## API

| Method | What it does |
|---|---|
| `Connection::connect(params)` | Connect, log in, detect the prompt, disable paging |
| `find_prompt()` | Send Enter and return the prompt line (e.g. `R1#`) |
| `send_command(cmd)` | Send a command and wait for the prompt |
| `send_command_expect(cmd, regex)` | Send a command and wait for a regex |
| `send_command_timing(cmd, quiet)` | Send a command and read until the device is silent for `quiet` |
| `enable()` / `check_enable_mode()` | Privileged mode; does nothing on vendors without enable (Huawei VRP, Junos, MikroTik, Linux) |
| `config_mode()` / `exit_config_mode()` / `check_config_mode()` | Configuration mode; does nothing on MikroTik and Linux |
| `send_multiline(&[(cmd, regex)])` | Send each command and wait for its regex (empty means the prompt) |
| `send_config_set(&[cmd])` | Enter config, send every command, exit (Junos stays in config) |
| `send_config_set_checked(&[cmd], error_regex)` | Same, but fails with `Error::ConfigInvalid` at the first command whose output matches `error_regex` |
| `commit()` | Junos only; fails with `Error::CommitFailed` unless the output says `commit complete` |
| `save_config()` | Cisco `write memory`, ZTE `write`, Huawei `save` (answers the VRP "continue?" question with `y`) |
| `disconnect()` | Close the session; dropping the `Connection` closes it too |
| `ferromiko::autodetect(&params)` | Log in over SSH and guess the `DeviceType` (`None` if nothing matches) |
| `ferromiko::scp_upload(&params, local, remote)` / `scp_download(&params, remote, local)` | Copy one file over SCP |
| `send_command_textfsm(cmd, templates_dir)` | `textfsm` feature: send a command and parse it with the matching ntc-templates template; `TextFsmOutput::Raw` with the text when there is none or it found no rows (as netmiko) |
| `ferromiko::parse_ntc(dir, device_type, cmd, output)` / `parse_textfsm(template, output)` | `textfsm` feature: parse output via the index, or with one template file |
| `ferromiko::ntc_templates_dir()` | `textfsm` feature: the templates directory from `NET_TEXTFSM` |

`Connection::from_transport(stream, device_type, secret, timeout)` accepts any stream that implements `Read + Write` (non-blocking, returning `WouldBlock` when no data is waiting). Use it for a custom transport or in tests.

## Per-vendor behavior

| Vendor | After login | Config enter / exit | Save |
|---|---|---|---|
| Cisco IOS | `terminal length 0`, `terminal width 511` | `configure terminal` / `end` | `write memory` |
| ZTE (SSH & Telnet) | `terminal length 0` | `configure terminal` / `end` | `write` |
| Huawei VRP (SSH & Telnet) | declines the "Change now?" password prompt with `N`, then `screen-length 0 temporary` | `system-view` / `return` | `save`, confirmed with `y` |
| Huawei OLT (SSH & Telnet) | answers a "YES/NO" login question with `yes`, then `enable`, `undo smart`, `infoswitch cli OFF`, `scroll` | `config` / `return` | `save` |
| Junos | `cli` if the login lands in a `%` shell, then `set cli complete-on-space off`, `set cli screen-length 0`, `set cli screen-width 511` | `configure` / `exit configuration-mode` | `commit` |
| MikroTik | username sent with the `+ct511w4098h` suffix (colors off), Enter is `\r\n` | none | none |
| Linux | `export TERM=dumb` | none | none |

Telnet uses `\r\n` as Enter, accepts server echo, and announces a 500x50 window, the same as netmiko. Huawei VRP over Telnet ends the password with a bare `\r`, as netmiko does.

Huawei VRP prompts look like `<HUAWEI>` and `[HUAWEI-GigabitEthernet0/0/1]`; VRPv8 state markers (`[~HUAWEI]`, `[*HUAWEI]`) are recognized too.

## Errors

Every method returns `ferromiko::Result<T>`. The `Error` variants worth handling:

- `Auth`: login rejected, wrong enable secret, or Telnet asked to log in again.
- `Timeout { pattern, output }`: the prompt or pattern did not appear within `timeout`. `output` holds what was received so far, which helps with debugging.
- `Unsupported`: for example `save_config()` on Linux or `commit()` on Cisco.
- `ModeChange`: could not enter or leave enable/config mode.
- `CommitFailed`: holds the Junos commit output.
- `ConfigInvalid { command, output }`: a command in `send_config_set_checked` matched the error pattern.
- `Scp`: the device rejected an SCP transfer (for example "No such file or directory").
- `TextFsm`: a TextFSM template or `index` is invalid or missing, or a template reached an `Error` action.
- `Io`, `Ssh`, `Regex`, `UnknownDeviceType`.

## CLI example

```sh
FERROMIKO_PASS=secret cargo run --example show -- cisco_ios 10.0.0.1 admin "show version"
FERROMIKO_PASS=secret FERROMIKO_SECRET=enable cargo run --example show -- zte_zxros_telnet 10.0.0.5 admin "show card"
```

The password is read from the environment, not from the arguments, so it does not end up in shell history. `enable()` is only called when `FERROMIKO_SECRET` is set.

## Development

```sh
cargo test             # unit tests + end-to-end Telnet tests against fake ZTE and Huawei servers on localhost
cargo test --features textfsm
cargo clippy --all-targets --all-features
```

The test against a real SSH server is ignored by default:

```sh
FERROMIKO_HOST=127.0.0.1 FERROMIKO_USER=$USER FERROMIKO_KEY=~/.ssh/id_ed25519 \
  [FERROMIKO_PORT=22] [FERROMIKO_PASS=...] cargo test --test linux_live -- --ignored
```

Code layout:

```text
src/domain/        pure rules: vendor profiles, prompt patterns, output cleanup, ntc platform names
src/integrations/  SSH transport and SCP (russh), Telnet transport (TCP + IAC negotiation), TextFSM (textfsm-core)
src/services/      Connection, read loop, Telnet login
src/types/         DeviceParams, Transport trait
src/utils/         write helpers for non-blocking sockets
```

## Limitations

- Not yet tested against real Cisco, ZTE, Huawei, Junos, or MikroTik devices. The SSH path has been tested against a Linux sshd, the Telnet path against fake ZTE and Huawei servers.
- SSH host keys are not verified, so connections are open to MITM. Use it on a trusted management network.
- Telnet sends the password unencrypted, and is only available for ZTE and Huawei.
- Huawei VRPv8 (`huawei_vrpv8`, which needs `commit`), the SmartAX MMI mode (`huawei_smartaxmmi`) and Huawei ONT are not supported yet. The VRP "security risks in the configuration file" save question is not answered.
- `legacy_ssh_algorithms` has been verified against OpenSSH restricted to SHA-1 key exchange, `ssh-rsa`, CBC/3DES and `hmac-sha1`, not against real old devices. `ssh-dss` is offered but untested, and russh refuses Diffie-Hellman groups below 2048 bits.
- The API is blocking and the SSH transport runs its own tokio runtime. From async code, call it inside `spawn_blocking`; calling it directly on a tokio worker panics.
- SCP: no recursive copy, no MD5 verification, no overwrite or free-space check. A download that fails midway leaves a partial local file.
- The session log masks a password only when it arrives in a single read; one split across two reads is written in clear.
- On Linux, a prompt that shows the current directory can make commands after `cd` time out.
- On Junos, `exit_config_mode()` answers "yes" when asked about uncommitted changes, so those changes are discarded (netmiko does the same).
- TextFSM uses the [textfsm-core](https://crates.io/crates/textfsm-core) port, which matches 661 of the 665 ntc-templates fixtures for the supported platforms. It differs from Python on `huawei_vrp` `display vlan` and on MikroTik `ping` (Python fills unmatched `List` entries with `None`). `parse_ntc` returns `None` when no template matches and `Some(vec![])` when the template finds no rows; only `send_command_textfsm` falls back to the raw text. The `index` is read once per directory per process (as in Python), and parsed templates are kept too, so an edited template is only picked up after a restart.
- Not implemented: SSH agent or jump host, async API, Telnet autodetection.

## License

MIT. See [LICENSE](LICENSE).
