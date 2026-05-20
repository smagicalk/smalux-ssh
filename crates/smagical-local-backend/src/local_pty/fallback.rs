//! 本地命令 fallback 状态机。
//!
//! 当 PTY 输出暂时没有新数据时，延迟启动备用本地命令执行，避免阻塞输出轮询。

use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use smagical_backend_core::{BackendEvent, LocalShellProfile};
use smagical_core::SessionId;

use crate::{LocalCommandFallback, LocalCommandFallbackResult};

pub(super) const LOCAL_PTY_FALLBACK_AFTER: Duration = Duration::from_millis(180);

pub(super) enum PendingFallbackCommand {
    Waiting {
        input: String,
        created_at: Instant,
    },
    Running {
        receiver: Receiver<LocalCommandFallbackResult>,
    },
}

pub(super) fn drain_pending_fallback(
    session_id: SessionId,
    shell: &LocalShellProfile,
    state: &mut Option<PendingFallbackCommand>,
) -> Vec<BackendEvent> {
    let Some(pending) = state.take() else {
        return Vec::new();
    };

    match pending {
        PendingFallbackCommand::Waiting { input, created_at } => {
            if created_at.elapsed() < LOCAL_PTY_FALLBACK_AFTER {
                *state = Some(PendingFallbackCommand::Waiting { input, created_at });
                Vec::new()
            } else {
                let receiver = spawn_fallback_worker(session_id, input, shell.clone());
                *state = Some(PendingFallbackCommand::Running { receiver });
                Vec::new()
            }
        }
        PendingFallbackCommand::Running { receiver } => match receiver.try_recv() {
            Ok(result) => result.events,
            Err(TryRecvError::Empty) => {
                *state = Some(PendingFallbackCommand::Running { receiver });
                Vec::new()
            }
            Err(TryRecvError::Disconnected) => Vec::new(),
        },
    }
}

pub(super) fn pending_fallback_from_input(input: String) -> PendingFallbackCommand {
    PendingFallbackCommand::Waiting {
        input,
        created_at: Instant::now(),
    }
}

fn spawn_fallback_worker(
    session_id: SessionId,
    input: String,
    shell: LocalShellProfile,
) -> Receiver<LocalCommandFallbackResult> {
    let (sender, receiver) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let result = LocalCommandFallback::execute_with_profile(session_id, &input, &shell);
        let _ = sender.send(result);
    });
    receiver
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use uuid::Uuid;

    use smagical_backend_core::LocalShellKind;

    use super::*;

    fn session_id() -> SessionId {
        SessionId(Uuid::new_v4())
    }

    fn shell_profile() -> LocalShellProfile {
        LocalShellProfile {
            kind: LocalShellKind::Posix,
            program: "sh".to_owned(),
            interactive_args: Vec::new(),
            prompt: "$",
        }
    }

    #[test]
    fn waiting_fallback_before_due_keeps_state() {
        let mut state = Some(PendingFallbackCommand::Waiting {
            input: "echo delayed".to_owned(),
            created_at: Instant::now(),
        });

        let events = drain_pending_fallback(session_id(), &shell_profile(), &mut state);

        assert!(events.is_empty());
        assert!(matches!(
            state,
            Some(PendingFallbackCommand::Waiting { ref input, .. }) if input == "echo delayed"
        ));
    }

    #[test]
    fn running_fallback_without_result_keeps_receiver() {
        let (_sender, receiver) = mpsc::channel();
        let mut state = Some(PendingFallbackCommand::Running { receiver });

        let events = drain_pending_fallback(session_id(), &shell_profile(), &mut state);

        assert!(events.is_empty());
        assert!(matches!(
            state,
            Some(PendingFallbackCommand::Running { .. })
        ));
    }

    #[test]
    fn running_fallback_returns_ready_events_and_clears_state() {
        let session_id = session_id();
        let (sender, receiver) = mpsc::channel();
        sender
            .send(LocalCommandFallbackResult {
                events: vec![BackendEvent::Output {
                    session_id,
                    line: "fallback output".to_owned(),
                }],
                exit_code: Some(0),
            })
            .unwrap();
        let mut state = Some(PendingFallbackCommand::Running { receiver });

        let events = drain_pending_fallback(session_id, &shell_profile(), &mut state);

        assert_eq!(
            events,
            vec![BackendEvent::Output {
                session_id,
                line: "fallback output".to_owned(),
            }]
        );
        assert!(state.is_none());
    }

    #[test]
    fn disconnected_fallback_clears_state_without_events() {
        let (sender, receiver) = mpsc::channel();
        drop(sender);
        let mut state = Some(PendingFallbackCommand::Running { receiver });

        let events = drain_pending_fallback(session_id(), &shell_profile(), &mut state);

        assert!(events.is_empty());
        assert!(state.is_none());
    }
}
