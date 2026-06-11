use super::*;
use crate::{ConnectionTarget, PtyRequest};
use smagical_core::{AgentSource, AuthProfile, Host, HostId, SessionId};
use smagical_terminal::TerminalSize;
use uuid::Uuid;

fn session_id() -> SessionId {
    SessionId(Uuid::new_v4())
}

fn host() -> Host {
    Host {
        id: HostId(Uuid::new_v4()),
        name: "production".to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: Vec::new(),
        address: "example.com".to_owned(),
        port: 22,
        auth: AuthProfile::Agent {
            username: "deploy".to_owned(),
            source: AgentSource::Auto,
            key_hint: None,
        },
        proxies: Vec::new(),
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    }
}

#[test]
fn noop_executor_reports_unsupported_command() {
    let mut executor = NoopBackendExecutor;
    let session_id = session_id();

    let error = executor
        .execute(BackendCommand::OpenShell {
            session_id,
            pty: PtyRequest::xterm(TerminalSize::default()),
        })
        .expect_err("占位执行器应该拒绝命令");

    assert_eq!(
        error,
        BackendExecutionError::UnsupportedCommand {
            kind: BackendCommandKind::OpenShell
        }
    );
}

#[test]
fn scripted_executor_returns_matching_events() {
    let mut executor = ScriptedBackendExecutor::new();
    let session_id = session_id();
    executor.push_response(ScriptedBackendResponse::new(
        BackendCommandKind::Connect,
        vec![BackendEvent::Connected { session_id }],
    ));

    let events = executor
        .execute(BackendCommand::Connect {
            session_id,
            target: ConnectionTarget::from_host(&host()),
        })
        .expect("脚本执行器应该返回匹配事件");

    assert_eq!(events, vec![BackendEvent::Connected { session_id }]);
    assert_eq!(executor.executed(), &[BackendCommandKind::Connect]);
    assert_eq!(executor.remaining(), 0);
}

#[test]
fn scripted_executor_consumes_responses_in_fifo_order() {
    let mut executor = ScriptedBackendExecutor::new();
    let session_id = session_id();
    executor.push_response(ScriptedBackendResponse::new(
        BackendCommandKind::Connect,
        vec![BackendEvent::Connected { session_id }],
    ));
    executor.push_response(ScriptedBackendResponse::new(
        BackendCommandKind::Disconnect,
        vec![BackendEvent::Disconnected { session_id }],
    ));

    let connected = executor
        .execute(BackendCommand::Connect {
            session_id,
            target: ConnectionTarget::from_host(&host()),
        })
        .expect("第一条脚本响应应该匹配连接命令");
    let disconnected = executor
        .execute(BackendCommand::Disconnect { session_id })
        .expect("第二条脚本响应应该匹配断开命令");

    assert_eq!(connected, vec![BackendEvent::Connected { session_id }]);
    assert_eq!(
        disconnected,
        vec![BackendEvent::Disconnected { session_id }]
    );
    assert_eq!(
        executor.executed(),
        &[BackendCommandKind::Connect, BackendCommandKind::Disconnect]
    );
    assert_eq!(executor.remaining(), 0);
}

#[test]
fn scripted_executor_rejects_unexpected_command_kind() {
    let mut executor = ScriptedBackendExecutor::new();
    let session_id = session_id();
    executor.push_response(ScriptedBackendResponse::new(
        BackendCommandKind::Disconnect,
        Vec::new(),
    ));

    let error = executor
        .execute(BackendCommand::OpenShell {
            session_id,
            pty: PtyRequest::xterm(TerminalSize::default()),
        })
        .expect_err("命令类型不匹配应该失败");

    assert_eq!(
        error,
        BackendExecutionError::UnexpectedCommand {
            expected: BackendCommandKind::Disconnect,
            actual: BackendCommandKind::OpenShell,
        }
    );
    assert_eq!(executor.executed(), &[]);
    assert_eq!(executor.remaining(), 1);
}

#[test]
fn scripted_executor_reports_missing_response_without_recording_command() {
    let mut executor = ScriptedBackendExecutor::new();
    let session_id = session_id();

    let error = executor
        .execute(BackendCommand::Disconnect { session_id })
        .expect_err("没有脚本响应时应该失败");

    assert_eq!(error, BackendExecutionError::NoScriptedResponse);
    assert_eq!(executor.executed(), &[]);
}

#[test]
fn noop_shared_backend_executor_rejects_commands_through_lock() {
    let executor = noop_shared_backend_executor();
    let session_id = session_id();
    let mut guard = executor.lock().expect("共享执行器锁不应中毒");

    let error = guard
        .execute(BackendCommand::Disconnect { session_id })
        .expect_err("占位共享执行器应该拒绝命令");

    assert_eq!(
        error,
        BackendExecutionError::UnsupportedCommand {
            kind: BackendCommandKind::Disconnect,
        }
    );
}

#[test]
fn backend_execution_errors_keep_action_context() {
    let connection = BackendExecutionError::ConnectionFailed {
        endpoint: "example.com:22".to_owned(),
        reason: "timeout".to_owned(),
    };
    let authentication = BackendExecutionError::AuthenticationFailed {
        username: "deploy".to_owned(),
        reason: "permission denied".to_owned(),
    };
    let channel = BackendExecutionError::ChannelFailed {
        operation: "open shell".to_owned(),
        reason: "session closed".to_owned(),
    };

    assert!(connection.to_string().contains("example.com:22"));
    assert!(authentication.to_string().contains("deploy"));
    assert!(channel.to_string().contains("open shell"));
}
