//! SSH 隧道运行句柄的纯状态。

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use smagical_backend_core::BackendExecutionError;
use smagical_core::SessionId;
use tokio::io::{AsyncRead, AsyncWrite};

/// 运行中的 SSH 隧道句柄。
pub struct RemoteTunnel {
    session_id: SessionId,
    rule_name: String,
    running: Arc<AtomicBool>,
    bind_host: String,
    bind_port: u16,
}

impl RemoteTunnel {
    /// 返回隧道所属的会话标识。
    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// 返回关联的隧道规则名称。
    pub fn rule_name(&self) -> &str {
        &self.rule_name
    }

    /// 返回本地或远端监听地址。
    pub fn bind_endpoint(&self) -> String {
        format!("{}:{}", self.bind_host, self.bind_port)
    }

    /// 请求隧道循环停止。已建立的连接会自然结束。
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

/// 创建运行中的 SSH 隧道句柄。
pub fn remote_tunnel(
    session_id: SessionId,
    rule_name: String,
    running: Arc<AtomicBool>,
    bind_host: String,
    bind_port: u16,
) -> RemoteTunnel {
    RemoteTunnel {
        session_id,
        rule_name,
        running,
        bind_host,
        bind_port,
    }
}

/// 将 russh TCP forwarding 错误转换成后端 tunnel 错误。
pub fn tunnel_error(rule_name: &str, error: russh::Error) -> BackendExecutionError {
    tunnel_reason_error(rule_name, error)
}

/// 将 IO 错误转换成后端 tunnel 错误。
pub fn tunnel_io_error(rule_name: &str, error: std::io::Error) -> BackendExecutionError {
    tunnel_reason_error(rule_name, error)
}

/// 将错误原因转换成后端 tunnel 错误。
pub fn tunnel_reason_error(
    rule_name: &str,
    reason: impl std::fmt::Display,
) -> BackendExecutionError {
    BackendExecutionError::TunnelFailed {
        rule_name: rule_name.to_owned(),
        reason: reason.to_string(),
    }
}

/// 双向复制两个异步流，忽略成功时的字节统计。
pub async fn copy_bidirectional<A, B>(left: &mut A, right: &mut B) -> std::io::Result<()>
where
    A: AsyncRead + AsyncWrite + Unpin,
    B: AsyncRead + AsyncWrite + Unpin,
{
    let _ = tokio::io::copy_bidirectional(left, right).await?;
    Ok(())
}
