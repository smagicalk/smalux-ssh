use super::*;
use crate::backend::{
    BackendCommand, BackendCommandKind, BackendEvent, BackendExecutionError, BackendExecutor,
    NoopBackendExecutor, ScriptedBackendExecutor, ScriptedBackendResponse,
};
use crate::model::{
    AgentSource, AuthProfile, Host, HostKeyVerification, KeyAlgorithm, KnownHostEntry,
    SessionStatus, TransferStatus, TunnelKind, TunnelRule, TunnelStatus,
};

#[path = "backend_pump_tests/basic.rs"]
mod basic;
#[path = "backend_pump_tests/connect.rs"]
mod connect;
#[path = "backend_pump_tests/local_terminal.rs"]
mod local_terminal;
#[path = "backend_pump_tests/sftp.rs"]
mod sftp;
#[path = "backend_pump_tests/terminal.rs"]
mod terminal;
#[path = "backend_pump_tests/tunnel.rs"]
mod tunnel;

fn sample_host() -> Host {
    Host {
        id: HostId(uuid::Uuid::new_v4()),
        name: "production".to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: vec!["prod".to_owned()],
        address: "example.com".to_owned(),
        port: 22,
        auth: AuthProfile::Agent {
            username: "deploy".to_owned(),
            source: AgentSource::Auto,
            key_hint: Some("id_ed25519".to_owned()),
        },
        network: Default::default(),
        proxies: Vec::new(),
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    }
}

fn sample_tunnel_rule() -> TunnelRule {
    TunnelRule {
        name: "local-db".to_owned(),
        kind: TunnelKind::Local,
        bind_host: "127.0.0.1".to_owned(),
        bind_port: 15432,
        target_host: "10.0.0.5".to_owned(),
        target_port: 5432,
        auto_start: false,
    }
}

#[derive(Debug)]
struct RejectingHostKeyExecutor {
    verification: HostKeyVerification,
}

impl RejectingHostKeyExecutor {
    fn new(verification: HostKeyVerification) -> Self {
        Self { verification }
    }
}

impl BackendExecutor for RejectingHostKeyExecutor {
    fn execute(
        &mut self,
        _command: crate::backend::BackendCommand,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        Err(BackendExecutionError::HostKeyRejected {
            host: "example.com".to_owned(),
            port: 22,
            key_algorithm: KeyAlgorithm::Ed25519,
            fingerprint: "SHA256:new".to_owned(),
            verification: self.verification.clone(),
        })
    }
}

#[test]
fn backend_queue_pump_noops_when_queue_is_empty() {
    let mut state = AppState::default();
    let mut executor = ScriptedBackendExecutor::new();

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(!outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 0);
    assert!(outcome.error.is_none());
}
