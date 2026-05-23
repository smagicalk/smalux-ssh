use super::super::RusshBackendExecutor;
use super::common::{assert_channel_failure, session_id};
use crate::backend::{BackendCommand, BackendEvent, TunnelStartRequest, TunnelStopRequest};
use crate::model::{TunnelKind, TunnelRule, TunnelStatus};
use crate::security::MemorySecretStore;
use smagical_backend_core::BackendExecutor;

#[test]
fn start_tunnel_requires_connected_session() {
    let mut executor =
        RusshBackendExecutor::new(MemorySecretStore::new()).expect("执行器应该可以创建 runtime");
    let session_id = session_id();

    let error = executor
        .execute(BackendCommand::StartTunnel {
            session_id,
            request: TunnelStartRequest::new(TunnelRule {
                name: "proxy".to_owned(),
                kind: TunnelKind::Dynamic,
                bind_host: "127.0.0.1".to_owned(),
                bind_port: 1080,
                target_host: String::new(),
                target_port: 0,
                auto_start: false,
            })
            .expect("动态隧道请求应该有效"),
        })
        .expect_err("未连接会话不能启动隧道");

    assert_channel_failure(&error, "start tunnel", "session is not connected");
}

#[test]
fn stop_tunnel_without_runtime_is_idempotent() {
    let mut executor =
        RusshBackendExecutor::new(MemorySecretStore::new()).expect("执行器应该可以创建 runtime");
    let session_id = session_id();

    let events = executor
        .execute(BackendCommand::StopTunnel {
            session_id,
            request: TunnelStopRequest::by_name("proxy"),
        })
        .expect("停止缺失隧道应该保持幂等");

    assert_eq!(
        events,
        vec![BackendEvent::TunnelStatusChanged {
            session_id,
            rule_name: "proxy".to_owned(),
            status: TunnelStatus::Stopped,
        }]
    );
    assert_eq!(executor.tunnel_count(), 0);
}
