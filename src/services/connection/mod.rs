#[allow(clippy::module_inception)]
pub mod connection;
pub mod read_until;
pub mod session_log;
pub mod telnet_login;

#[cfg(test)]
mod connection_tests;
#[cfg(test)]
mod fake_transport;
