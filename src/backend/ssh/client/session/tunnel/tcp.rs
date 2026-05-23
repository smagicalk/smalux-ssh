//! 隧道 TCP 监听、转发和 SOCKS5 连接处理。

mod listener;
mod pipe;
mod socks5;

pub(super) use listener::{accept_with_tick, bind_tcp_listener};
pub(super) use pipe::{pipe_direct_tcpip, pipe_forwarded_tcpip};
pub(super) use socks5::serve_socks5_connection;
