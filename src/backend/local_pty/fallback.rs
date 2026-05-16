//! 本地命令 fallback 状态机。
//!
//! 当 PTY 输出暂时没有新数据时，延迟启动备用本地命令执行，避免阻塞输出轮询。

use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use crate::model::SessionId;

use crate::backend::{
    BackendEvent, LocalCommandFallback, LocalCommandFallbackResult, LocalShellProfile,
};

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
