//! 单个本地 PTY 会话。
//!
//! 负责 PTY 句柄、输入写入、输出汇总和 fallback 状态。

use std::io::Write;
use std::sync::mpsc::Receiver;

#[cfg(test)]
use std::thread;
#[cfg(test)]
use std::time::{Duration, Instant};

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

use smagical_backend_core::{BackendEvent, BackendExecutionError, LocalShellProfile};
use smagical_core::SessionId;

use super::fallback::{
    PendingFallbackCommand, drain_pending_fallback, pending_fallback_from_input,
};
use super::local_pty_error;
use super::reader::spawn_reader_thread;

#[cfg(test)]
const LOCAL_PTY_POLL_INTERVAL: Duration = Duration::from_millis(10);

pub(super) struct LocalPtySession {
    session_id: SessionId,
    shell: LocalShellProfile,
    _master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    output: Receiver<BackendEvent>,
    _child: Box<dyn Child + Send + Sync>,
    pending_fallback: Option<PendingFallbackCommand>,
}

impl LocalPtySession {
    pub(super) fn spawn(
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

    pub(super) fn write_input(&mut self, input: &str) -> Result<(), BackendExecutionError> {
        let input = self.shell.normalize_input(input);
        self.writer
            .write_all(input.as_bytes())
            .and_then(|_| self.writer.flush())
            .map_err(|error| local_pty_error("write input", &error.to_string()))
    }

    pub(super) fn drain_output(&mut self) -> Vec<BackendEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.output.try_recv() {
            events.push(event);
        }
        if !events.is_empty() {
            self.pending_fallback = None;
            return events;
        }

        events.extend(drain_pending_fallback(
            self.session_id,
            &self.shell,
            &mut self.pending_fallback,
        ));
        events
    }

    pub(super) fn remember_fallback(&mut self, input: String) {
        self.pending_fallback = Some(pending_fallback_from_input(input));
    }

    #[cfg(test)]
    pub(super) fn child_is_running(&mut self) -> bool {
        self._child
            .try_wait()
            .expect("child status should be readable")
            .is_none()
    }

    #[cfg(test)]
    pub(super) fn drain_output_for(&mut self, timeout: Duration) -> Vec<BackendEvent> {
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
