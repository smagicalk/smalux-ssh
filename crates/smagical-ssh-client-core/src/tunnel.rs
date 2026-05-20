//! SSH 隧道运行句柄的纯状态。

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use smagical_core::SessionId;

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
