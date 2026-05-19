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
        for event in read_events_from_stream(session_id, &mut reader) {
            if sender.send(event).is_err() {
                return;
            }
        }
    });
    receiver
}

fn read_events_from_stream(session_id: SessionId, reader: &mut dyn Read) -> Vec<BackendEvent> {
    let mut events = Vec::new();
    let mut buffer = [0_u8; 4096];
    let mut decoder = TerminalStreamDecoder::new();

    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(bytes_read) => {
                events.extend(
                    decoder
                        .feed(&buffer[..bytes_read])
                        .into_iter()
                        .map(|event| terminal_event_to_backend(session_id, event)),
                );
            }
            Err(error) => {
                events.push(BackendEvent::Output {
                    session_id,
                    line: format!("{DEFAULT_LOCAL_TERMINAL_TITLE} read failed: {error}"),
                });
                break;
            }
        }
    }

    events.extend(
        decoder
            .finish()
            .into_iter()
            .map(|event| terminal_event_to_backend(session_id, event)),
    );
    events.push(BackendEvent::Output {
        session_id,
        line: format!("session {} closed", session_id.0),
    });
    events
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

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Error, ErrorKind, Read};

    use uuid::Uuid;

    use super::*;

    fn session_id() -> SessionId {
        SessionId(Uuid::new_v4())
    }

    #[test]
    fn read_events_from_stream_decodes_output_and_close() {
        let session_id = session_id();
        let mut reader = Cursor::new(b"hello\r\nprompt>".to_vec());

        let events = read_events_from_stream(session_id, &mut reader);

        assert_eq!(
            events,
            vec![
                BackendEvent::Output {
                    session_id,
                    line: "hello".to_owned(),
                },
                BackendEvent::Output {
                    session_id,
                    line: "prompt>".to_owned(),
                },
                BackendEvent::Output {
                    session_id,
                    line: format!("session {} closed", session_id.0),
                },
            ]
        );
    }

    #[test]
    fn read_events_from_stream_decodes_clear_screen() {
        let session_id = session_id();
        let mut reader = Cursor::new(b"\x1b[2J\x1b[H".to_vec());

        let events = read_events_from_stream(session_id, &mut reader);

        assert_eq!(
            events,
            vec![
                BackendEvent::ClearTerminal { session_id },
                BackendEvent::Output {
                    session_id,
                    line: format!("session {} closed", session_id.0),
                },
            ]
        );
    }

    #[test]
    fn read_events_from_stream_reports_read_errors_and_close() {
        let session_id = session_id();
        let mut reader = FailingReader;

        let events = read_events_from_stream(session_id, &mut reader);

        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            BackendEvent::Output { session_id: id, line }
                if *id == session_id
                    && line.contains(DEFAULT_LOCAL_TERMINAL_TITLE)
                    && line.contains("injected read failure")
        ));
        assert_eq!(
            events[1],
            BackendEvent::Output {
                session_id,
                line: format!("session {} closed", session_id.0),
            }
        );
    }

    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
            Err(Error::new(ErrorKind::Other, "injected read failure"))
        }
    }
}
