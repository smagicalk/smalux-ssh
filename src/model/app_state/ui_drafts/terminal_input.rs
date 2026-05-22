//! 终端输入草稿处理。

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
        self.ui.set_terminal_input(session_id, input);
        draft_changed()
    }

    /// 向终端输入草稿追加可见字符。
    pub(in crate::model::app_state) fn append_terminal_input_draft(
        &mut self,
        session_id: SessionId,
        text: String,
    ) -> AppUpdateOutcome {
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
        let before = self.ui.terminal_input_for(session_id).to_owned();
        self.ui.backspace_terminal_input(session_id);
        AppUpdateOutcome {
            state_changed: before != self.ui.terminal_input_for(session_id),
            ..AppUpdateOutcome::default()
        }
    }
}

fn printable_terminal_input(text: &str) -> String {
    text.chars()
        .filter(|ch| !ch.is_control() && !('\u{e000}'..='\u{f8ff}').contains(ch))
        .collect()
}
