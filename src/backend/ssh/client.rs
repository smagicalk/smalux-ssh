//! 基于 `russh` 的真实 SSH 客户端边界。

mod auth;
mod connection;
mod connector;
mod handler;
mod host_key;
mod session;
mod settings;

pub use connection::{RusshConnection, RusshConnectionReport};
pub use connector::RusshConnector;
pub use handler::SshClientHandler;
pub use host_key::*;
pub use session::*;
pub use settings::RusshClientSettings;

#[cfg(test)]
mod tests;
