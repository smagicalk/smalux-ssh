//! SSH 端口转发和隧道运行时。

use crate::backend::{BackendEvent, BackendExecutionError, TunnelStartRequest};
use crate::model::SessionId;

use super::super::RusshConnection;

mod dynamic;
mod handle;
mod local;
mod remote;
mod socks5;
mod tcp;

pub use handle::RemoteTunnel;

impl RusshConnection {
    /// 消费当前连接并启动端口转发或动态隧道。
    pub async fn into_tunnel(
        self,
        session_id: SessionId,
        request: TunnelStartRequest,
    ) -> Result<(RemoteTunnel, Vec<BackendEvent>), BackendExecutionError> {
        match request.rule.kind {
            crate::model::TunnelKind::Local => self.start_local_tunnel(session_id, request).await,
            crate::model::TunnelKind::Dynamic => {
                self.start_dynamic_tunnel(session_id, request).await
            }
            crate::model::TunnelKind::Remote => self.start_remote_tunnel(session_id, request).await,
        }
    }
}
