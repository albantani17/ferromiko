#![doc = include_str!("../README.md")]

mod domain;
mod error;
mod integrations;
mod services;
mod types;
mod utils;

pub use domain::device::device_type::DeviceType;
pub use error::{Error, Result};
pub use integrations::ssh::scp::{scp_download, scp_upload};
pub use integrations::ssh::ssh_transport::SshTransport;
pub use integrations::telnet::telnet_transport::TelnetTransport;
#[cfg(feature = "textfsm")]
pub use integrations::textfsm::{
    ntc_templates_dir::ntc_templates_dir, parse_ntc::parse_ntc, parse_textfsm::parse_textfsm,
};
pub use services::autodetect::autodetect::autodetect;
pub use services::connection::connection::Connection;
pub use types::device_params::DeviceParams;
#[cfg(feature = "textfsm")]
pub use types::textfsm_record::{FieldValue, TextFsmOutput, TextFsmRecord};
pub use types::transport::Transport;
