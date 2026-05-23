//! 隧道 TCP 监听工具。

use std::net::SocketAddr;

use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

use crate::backend::BackendExecutionError;
use smagical_ssh_client_core::{TUNNEL_ACCEPT_TICK, tunnel_io_error};

pub(in crate::backend::ssh::client::session::tunnel) async fn bind_tcp_listener(
    bind_host: &str,
    bind_port: u16,
    rule_name: &str,
) -> Result<TcpListener, BackendExecutionError> {
    TcpListener::bind((bind_host, bind_port))
        .await
        .map_err(|error| tunnel_io_error(rule_name, error))
}

pub(in crate::backend::ssh::client::session::tunnel) async fn accept_with_tick(
    listener: &TcpListener,
) -> Result<Option<(TcpStream, SocketAddr)>, std::io::Error> {
    match timeout(TUNNEL_ACCEPT_TICK, listener.accept()).await {
        Ok(result) => result.map(Some),
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use smagical_ssh_client_core::tunnel_failure_parts;

    use super::{accept_with_tick, bind_tcp_listener};

    #[tokio::test]
    async fn bind_tcp_listener_reports_rule_name_on_failure() {
        let listener = bind_tcp_listener("127.0.0.1", 0, "primary").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let error = bind_tcp_listener("127.0.0.1", port, "duplicate")
            .await
            .unwrap_err();

        let (rule_name, reason) = tunnel_failure_parts(&error).expect("监听失败应该映射为隧道错误");
        assert_eq!(rule_name, "duplicate");
        assert!(!reason.is_empty());
    }

    #[tokio::test]
    async fn accept_with_tick_returns_none_without_connection() {
        let listener = bind_tcp_listener("127.0.0.1", 0, "idle").await.unwrap();

        let accepted = accept_with_tick(&listener).await.unwrap();

        assert!(accepted.is_none());
    }
}
