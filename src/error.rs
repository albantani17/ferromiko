pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("ssh error: {0}")]
    Ssh(#[from] ssh2::Error),
    #[error("invalid pattern: {0}")]
    Regex(#[from] regex::Error),
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("timed out waiting for pattern `{pattern}`; output so far:\n{output}")]
    Timeout { pattern: String, output: String },
    #[error("{0} is not supported for this device type")]
    Unsupported(&'static str),
    #[error("failed to change mode: {0}")]
    ModeChange(String),
    #[error("commit failed:\n{0}")]
    CommitFailed(String),
    #[error("unknown device type: {0}")]
    UnknownDeviceType(String),
    #[error("config command `{command}` failed:\n{output}")]
    ConfigInvalid { command: String, output: String },
}
