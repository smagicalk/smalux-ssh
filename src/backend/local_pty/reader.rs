//! 本地 PTY 输出读取器。
//!
//! 只负责把 PTY 字节流转成终端事件，再交给后端事件层。

use std::io::Read;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use crate::model::{DEFAULT_LOCAL_TERMINAL_TITLE, SessionId};
use crate::terminal::{TerminalStreamDecoder, TerminalStreamEvent};

use super::BackendEvent;

pub(crate) fn spawn_reader_thread(
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

pub(crate) fn terminal_event_to_backend(
    session_id: SessionId,
    event: TerminalStreamEvent,
) -> BackendEvent {
    match event {
        TerminalStreamEvent::Output(line) => BackendEvent::Output { session_id, line },
        TerminalStreamEvent::Clear => BackendEvent::ClearTerminal { session_id },
    }
}
