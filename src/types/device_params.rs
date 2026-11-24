use std::path::PathBuf;
use std::time::Duration;

use crate::DeviceType;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Connection settings, the equivalent of netmiko's `ConnectHandler(**device)` kwargs.
#[derive(Debug, Clone)]
pub struct DeviceParams {
    pub device_type: DeviceType,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    /// Enable secret (Cisco/ZTE).
    pub secret: Option<String>,
    /// Private key file; when set, public-key auth is used and `password` is the key passphrase.
    pub key_file: Option<PathBuf>,
    /// Used for TCP connect, SSH handshake and each wait for the device prompt.
    pub timeout: Duration,
}

impl DeviceParams {
    pub fn new(device_type: DeviceType, host: &str, username: &str, password: &str) -> Self {
        Self {
            device_type,
            host: host.to_string(),
            port: device_type.profile().protocol.default_port(),
            username: username.to_string(),
            password: password.to_string(),
            secret: None,
            key_file: None,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}
