//! 终端字节流解析。
//!
//! 本模块属于终端核心层，只理解 VT/ANSI 控制协议，不依赖 UI 或后端执行器。

use alacritty_terminal::vte::ansi::{ClearMode, Handler, Processor};

/// 终端输出字节流解析后的核心事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalStreamEvent {
    Output(String),
    Clear,
}

/// 把 PTY/SSH shell 输出的原始字节解析为终端核心事件。
pub struct TerminalStreamDecoder {
    parser: Processor,
    handler: TerminalStreamHandler,
}

impl TerminalStreamDecoder {
    /// 创建终端流解码器。
    pub fn new() -> Self {
        Self {
            parser: Processor::new(),
            handler: TerminalStreamHandler::default(),
        }
    }

    /// 推入一段原始终端输出，并返回解析出的事件。
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<TerminalStreamEvent> {
        self.parser.advance(&mut self.handler, bytes);
        self.handler.take_events()
    }

    /// 结束读取时刷新尚未换行但已经可见的文本。
    pub fn finish(&mut self) -> Vec<TerminalStreamEvent> {
        self.handler.flush_line();
        self.handler.take_events()
    }
}

impl Default for TerminalStreamDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
struct TerminalStreamHandler {
    current_line: String,
    events: Vec<TerminalStreamEvent>,
}

impl TerminalStreamHandler {
    fn take_events(&mut self) -> Vec<TerminalStreamEvent> {
        std::mem::take(&mut self.events)
    }

    fn flush_line(&mut self) {
        let line = self.current_line.trim_end().to_owned();
        self.current_line.clear();

        if !line.is_empty() {
            self.events.push(TerminalStreamEvent::Output(line));
        }
    }

    fn clear_visible_text(&mut self) {
        self.current_line.clear();
        self.events.push(TerminalStreamEvent::Clear);
    }
}

impl Handler for TerminalStreamHandler {
    fn input(&mut self, c: char) {
        if !c.is_control() {
            self.current_line.push(c);
        }
    }

    fn linefeed(&mut self) {
        self.flush_line();
    }

    fn carriage_return(&mut self) {
        self.flush_line();
    }

    fn backspace(&mut self) {
        self.current_line.pop();
    }

    fn clear_line(&mut self, _mode: alacritty_terminal::vte::ansi::LineClearMode) {
        self.current_line.clear();
    }

    fn clear_screen(&mut self, mode: ClearMode) {
        if matches!(mode, ClearMode::All | ClearMode::Saved) {
            self.clear_visible_text();
        }
    }

    fn reset_state(&mut self) {
        self.current_line.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoder_emits_plain_output_lines() {
        let mut decoder = TerminalStreamDecoder::new();

        let events = decoder.feed(b"hello\r\nworld\n");

        assert_eq!(
            events,
            vec![
                TerminalStreamEvent::Output("hello".to_owned()),
                TerminalStreamEvent::Output("world".to_owned()),
            ]
        );
    }

    #[test]
    fn decoder_maps_ansi_clear_display_to_terminal_event() {
        let mut decoder = TerminalStreamDecoder::new();

        let events = decoder.feed(b"\x1b[2J\x1b[H");

        assert_eq!(events, vec![TerminalStreamEvent::Clear]);
    }

    #[test]
    fn decoder_flushes_unterminated_tail_on_finish() {
        let mut decoder = TerminalStreamDecoder::new();

        assert!(decoder.feed(b"prompt>").is_empty());
        let events = decoder.finish();

        assert_eq!(
            events,
            vec![TerminalStreamEvent::Output("prompt>".to_owned())]
        );
    }
}
