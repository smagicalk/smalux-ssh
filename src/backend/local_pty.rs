//! 本地 PTY 后端执行器。
//!
//! 这个模块只负责本地 shell 进程和 PTY 读写，不读取 UI 状态，也不处理远程 SSH。

use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

use crate::model::{DEFAULT_LOCAL_TERMINAL_TITLE, LOCAL_TERMINAL_SESSION_ID, SessionId};
use crate::terminal::{TerminalStreamDecoder, TerminalStreamEvent};

use super::{
    BackendCommand, BackendEvent, BackendExecutionError, BackendExecutor, LocalCommandFallback,
    LocalCommandFallbackResult, LocalShellProfile,
};

const LOCAL_PTY_FALLBACK_AFTER: Duration = Duration::from_millis(180);
#[cfg(test)]
const LOCAL_PTY_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// 同时承载本地 PTY 和远程后端的组合执行器。
pub struct DesktopBackendExecutor<R> {
    local: LocalPtyBackendExecutor,
    remote: R,
}

impl<R> DesktopBackendExecutor<R> {
    /// 创建桌面端默认后端执行器。
    pub fn new(remote: R) -> Self {
        Self {
            local: LocalPtyBackendExecutor::default(),
            remote,
        }
    }
}

impl<R> BackendExecutor for DesktopBackendExecutor<R>
where
    R: BackendExecutor,
{
    fn execute(
        &mut self,
        command: BackendCommand,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        if command.session_id() == LOCAL_TERMINAL_SESSION_ID {
            self.local.execute(command)
        } else {
            self.remote.execute(command)
        }
    }
}

/// 本地 PTY 后端执行器。
pub struct LocalPtyBackendExecutor {
    sessions: HashMap<SessionId, LocalPtySession>,
    shell: LocalShellProfile,
}

impl Default for LocalPtyBackendExecutor {
    fn default() -> Self {
        Self {
            sessions: HashMap::new(),
            shell: LocalShellProfile::default_for_platform(),
        }
    }
}

impl LocalPtyBackendExecutor {
    /// 当前持有的本地 PTY 会话数量。
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    fn ensure_session(
        &mut self,
        session_id: SessionId,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        if self.sessions.contains_key(&session_id) {
            return Ok(self.drain_output(session_id));
        }

        let mut session = LocalPtySession::spawn(session_id, &self.shell)?;
        let mut events = vec![
            BackendEvent::Connected { session_id },
            BackendEvent::ShellOpened { session_id },
        ];
        events.extend(session.drain_output());
        self.sessions.insert(session_id, session);
        Ok(events)
    }

    fn send_input(
        &mut self,
        session_id: SessionId,
        input: String,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        let startup_events = self.ensure_session(session_id)?;
        let session = self
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| local_pty_error("send input", "local session missing after start"))?;

        let mut events = startup_events;
        session.write_input(&input)?;
        session.remember_fallback(input);
        events.extend(session.drain_output());

        Ok(events)
    }

    fn disconnect(
        &mut self,
        session_id: SessionId,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        self.sessions.remove(&session_id);
        Ok(vec![BackendEvent::Disconnected { session_id }])
    }

    fn drain_output(&mut self, session_id: SessionId) -> Vec<BackendEvent> {
        let Some(session) = self.sessions.get_mut(&session_id) else {
            return Vec::new();
        };

        session.drain_output()
    }
}

impl BackendExecutor for LocalPtyBackendExecutor {
    fn execute(
        &mut self,
        command: BackendCommand,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        match command {
            BackendCommand::OpenShell { session_id, .. } => self.ensure_session(session_id),
            BackendCommand::SendShellInput { session_id, input } => {
                self.send_input(session_id, input)
            }
            BackendCommand::DrainSessionOutput { session_id } => Ok(self.drain_output(session_id)),
            BackendCommand::Disconnect { session_id } => self.disconnect(session_id),
            other => Err(local_pty_error(
                "local pty",
                &format!("unsupported command: {:?}", other.kind()),
            )),
        }
    }
}

struct LocalPtySession {
    session_id: SessionId,
    shell: LocalShellProfile,
    _master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    output: Receiver<BackendEvent>,
    _child: Box<dyn Child + Send + Sync>,
    pending_fallback: Option<PendingFallbackCommand>,
}

impl LocalPtySession {
    fn spawn(
        session_id: SessionId,
        shell: &LocalShellProfile,
    ) -> Result<Self, BackendExecutionError> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 32,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| local_pty_error("open pty", &error.to_string()))?;

        let mut command = CommandBuilder::new(&shell.program);
        command.args(shell.interactive_args.iter());
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| local_pty_error("spawn shell", &error.to_string()))?;
        drop(pair.slave);

        let writer = pair
            .master
            .take_writer()
            .map_err(|error| local_pty_error("open pty writer", &error.to_string()))?;
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| local_pty_error("open pty reader", &error.to_string()))?;
        let output = spawn_reader_thread(session_id, reader);

        Ok(Self {
            session_id,
            shell: shell.clone(),
            _master: pair.master,
            writer,
            output,
            _child: child,
            pending_fallback: None,
        })
    }

    fn write_input(&mut self, input: &str) -> Result<(), BackendExecutionError> {
        let input = self.shell.normalize_input(input);
        self.writer
            .write_all(input.as_bytes())
            .and_then(|_| self.writer.flush())
            .map_err(|error| local_pty_error("write input", &error.to_string()))
    }

    fn drain_output(&mut self) -> Vec<BackendEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.output.try_recv() {
            events.push(event);
        }
        if !events.is_empty() {
            self.pending_fallback = None;
            return events;
        }

        events.extend(self.drain_fallback_output());
        events
    }

    fn remember_fallback(&mut self, input: String) {
        self.pending_fallback = Some(PendingFallbackCommand::Waiting {
            input,
            created_at: Instant::now(),
        });
    }

    #[cfg(test)]
    fn drain_output_for(&mut self, timeout: Duration) -> Vec<BackendEvent> {
        let deadline = Instant::now() + timeout;
        let mut events = Vec::new();

        while Instant::now() < deadline {
            events.extend(self.drain_output());
            thread::sleep(LOCAL_PTY_POLL_INTERVAL);
        }

        events.extend(self.drain_output());
        events
    }
}

enum PendingFallbackCommand {
    Waiting {
        input: String,
        created_at: Instant,
    },
    Running {
        receiver: Receiver<LocalCommandFallbackResult>,
    },
}

fn spawn_reader_thread(
    session_id: SessionId,
    mut reader: Box<dyn Read + Send>,
) -> Receiver<BackendEvent> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        let mut decoder = TerminalStreamDecoder::new();
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(bytes_read) => {
                    for event in decoder.feed(&buffer[..bytes_read]) {
                        if sender
                            .send(terminal_event_to_backend(session_id, event))
                            .is_err()
                        {
                            return;
                        }
                    }
                }
                Err(error) => {
                    let _ = sender.send(BackendEvent::Output {
                        session_id,
                        line: format!("{DEFAULT_LOCAL_TERMINAL_TITLE} read failed: {error}"),
                    });
                    break;
                }
            }
        }
        for event in decoder.finish() {
            if sender
                .send(terminal_event_to_backend(session_id, event))
                .is_err()
            {
                return;
            }
        }
        let _ = sender.send(BackendEvent::Output {
            session_id,
            line: format!("session {} closed", session_id.0),
        });
    });
    receiver
}

impl LocalPtySession {
    fn drain_fallback_output(&mut self) -> Vec<BackendEvent> {
        let Some(state) = self.pending_fallback.take() else {
            return Vec::new();
        };

        match state {
            PendingFallbackCommand::Waiting { input, created_at } => {
                if created_at.elapsed() < LOCAL_PTY_FALLBACK_AFTER {
                    self.pending_fallback =
                        Some(PendingFallbackCommand::Waiting { input, created_at });
                    Vec::new()
                } else {
                    let receiver =
                        spawn_fallback_worker(self.session_id, input, self.shell.clone());
                    self.pending_fallback = Some(PendingFallbackCommand::Running { receiver });
                    Vec::new()
                }
            }
            PendingFallbackCommand::Running { receiver } => match receiver.try_recv() {
                Ok(result) => result.events,
                Err(TryRecvError::Empty) => {
                    self.pending_fallback = Some(PendingFallbackCommand::Running { receiver });
                    Vec::new()
                }
                Err(TryRecvError::Disconnected) => Vec::new(),
            },
        }
    }
}

fn spawn_fallback_worker(
    session_id: SessionId,
    input: String,
    shell: LocalShellProfile,
) -> Receiver<LocalCommandFallbackResult> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = LocalCommandFallback::execute_with_profile(session_id, &input, &shell);
        let _ = sender.send(result);
    });
    receiver
}

pub(crate) fn terminal_event_to_backend(
    session_id: SessionId,
    event: TerminalStreamEvent,
) -> BackendEvent {
    match event {
        TerminalStreamEvent::Output(line) => BackendEvent::Output { session_id, line },
        TerminalStreamEvent::Clear => BackendEvent::ClearTerminal { session_id },
    }
}

fn local_pty_error(operation: &str, reason: &str) -> BackendExecutionError {
    BackendExecutionError::ChannelFailed {
        operation: operation.to_owned(),
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::PtyRequest;
    use crate::terminal::TerminalSize;

    #[test]
    #[ignore = "requires interactive Windows ConPTY output in the test runner"]
    fn local_pty_starts_shell_and_accepts_input() {
        let mut executor = LocalPtyBackendExecutor::default();
        let session_id = LOCAL_TERMINAL_SESSION_ID;

        let open = executor
            .execute(BackendCommand::OpenShell {
                session_id,
                pty: PtyRequest::xterm(TerminalSize::default()),
            })
            .expect("local pty should open");
        let open_drain = executor
            .sessions
            .get_mut(&session_id)
            .expect("local session should exist")
            .drain_output_for(Duration::from_millis(750));
        let child_status = executor
            .sessions
            .get_mut(&session_id)
            .expect("local session should exist")
            ._child
            .try_wait()
            .expect("child status should be readable");
        let output = executor
            .execute(BackendCommand::SendShellInput {
                session_id,
                input: local_echo_command("smagicalssh-pty"),
            })
            .expect("local pty should accept input");
        std::thread::sleep(LOCAL_PTY_FALLBACK_AFTER + Duration::from_millis(50));
        let delayed_start = executor
            .execute(BackendCommand::DrainSessionOutput { session_id })
            .expect("local pty should drain delayed output");
        let delayed_output = executor
            .sessions
            .get_mut(&session_id)
            .expect("local session should exist")
            .drain_output_for(Duration::from_secs(3));

        assert_eq!(executor.session_count(), 1);
        assert!(open.iter().any(|event| matches!(
            event,
            BackendEvent::ShellOpened { session_id: id } if *id == session_id
        )));
        assert!(
            child_status.is_none(),
            "local shell exited before input; open={open:?}, open_drain={open_drain:?}, status={child_status:?}"
        );
        assert!(
            output
                .iter()
                .chain(delayed_start.iter())
                .chain(delayed_output.iter())
                .any(|event| matches!(
                    event,
                    BackendEvent::Output { line, .. } if line.contains("smagicalssh-pty")
                )),
            "local pty output should contain echo marker after drain: open={open:?}, open_drain={open_drain:?}, output={output:?}, delayed_start={delayed_start:?}, delayed={delayed_output:?}"
        );
    }

    fn local_echo_command(text: &str) -> String {
        if cfg!(windows) {
            format!("Write-Output '{text}'\n")
        } else {
            format!("printf '{text}\\n'\n")
        }
    }

    #[test]
    #[ignore = "requires interactive Windows ConPTY output in the test runner"]
    fn due_fallback_starts_worker_without_blocking_drain() {
        let mut session = LocalPtySession::spawn(
            LOCAL_TERMINAL_SESSION_ID,
            &LocalShellProfile::default_for_platform(),
        )
        .expect("local pty should open");
        session.remember_fallback(local_echo_command("smagicalssh-fallback-worker"));

        std::thread::sleep(LOCAL_PTY_FALLBACK_AFTER + Duration::from_millis(20));
        let started_at = Instant::now();
        let first_drain = session.drain_output();

        assert!(
            started_at.elapsed() < Duration::from_millis(120),
            "starting fallback worker should not wait for command output"
        );
        assert!(first_drain.is_empty());

        let output = session.drain_output_for(Duration::from_secs(3));

        assert!(output.iter().any(|event| matches!(
            event,
            BackendEvent::Output { line, .. } if line.contains("smagicalssh-fallback-worker")
        )));
    }
}
