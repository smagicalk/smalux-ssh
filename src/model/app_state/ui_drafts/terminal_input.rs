//! 终端输入草稿处理。
//!
//! 终端输入框是 UI 草稿，不直接写入终端缓冲区。用户按发送后才会生成
//! `BackendCommand::SendShellInput`，这样可以支持历史、回显和本地/远程差异化处理。

use crate::model::SessionId;

use super::super::{AppState, AppUpdateOutcome};
use super::draft_changed;

impl AppState {
    /// 更新终端输入草稿。
    pub(in crate::model::app_state) fn update_terminal_input_draft(
        &mut self,
        session_id: SessionId,
        input: String,
    ) -> AppUpdateOutcome {
        // 全量更新用于普通输入框绑定。
        self.ui.set_terminal_input(session_id, input);
        draft_changed()
    }

    /// 向终端输入草稿追加可见字符。
    pub(in crate::model::app_state) fn append_terminal_input_draft(
        &mut self,
        session_id: SessionId,
        text: String,
    ) -> AppUpdateOutcome {
        // 追加更新用于按键事件，过滤控制字符后再写入草稿。
        let filtered = printable_terminal_input(&text);
        if filtered.is_empty() {
            return AppUpdateOutcome::default();
        }

        self.ui.append_terminal_input(session_id, filtered);
        draft_changed()
    }

    /// 删除终端输入草稿的最后一个字符。
    pub(in crate::model::app_state) fn backspace_terminal_input_draft(
        &mut self,
        session_id: SessionId,
    ) -> AppUpdateOutcome {
        // 删除前后做一次比较，只有真实变化才刷新 UI。
        let before = self.ui.terminal_input_for(session_id).to_owned();
        self.ui.backspace_terminal_input(session_id);
        AppUpdateOutcome {
            state_changed: before != self.ui.terminal_input_for(session_id),
            ..AppUpdateOutcome::default()
        }
    }
}

fn printable_terminal_input(text: &str) -> String {
    // 过滤控制字符和私用区按键占位，避免快捷键事件被误写成可见命令文本。
    text.chars()
        .filter(|ch| !ch.is_control() && !('\u{e000}'..='\u{f8ff}').contains(ch))
        .collect()
}
