//! 终端输入草稿。

use serde::{Deserialize, Serialize};

use super::UiState;
use crate::model::SessionId;

/// 按会话保存的终端输入草稿。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalInputDraft {
    pub session_id: SessionId,
    pub input: String,
}

impl TerminalInputDraft {
    /// 为指定会话创建空输入草稿。
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            input: String::new(),
        }
    }
}

impl UiState {
    /// 返回指定终端会话的输入草稿；没有草稿时返回空字符串。
    pub fn terminal_input_for(&self, session_id: SessionId) -> &str {
        self.terminal_input_drafts
            .iter()
            .find(|draft| draft.session_id == session_id)
            .map(|draft| draft.input.as_str())
            .unwrap_or("")
    }

    /// 更新指定终端会话的输入草稿。
    pub fn set_terminal_input(&mut self, session_id: SessionId, input: impl Into<String>) {
        self.ensure_terminal_input_draft(session_id).input = input.into();
    }

    /// 向指定终端会话的输入草稿追加文本。
    pub fn append_terminal_input(&mut self, session_id: SessionId, text: impl AsRef<str>) {
        self.ensure_terminal_input_draft(session_id)
            .input
            .push_str(text.as_ref());
    }

    /// 删除指定终端会话输入草稿的最后一个字符。
    pub fn backspace_terminal_input(&mut self, session_id: SessionId) {
        if let Some(draft) = self
            .terminal_input_drafts
            .iter_mut()
            .find(|draft| draft.session_id == session_id)
        {
            draft.input.pop();
        }
    }

    /// 清空指定终端会话的输入草稿。
    pub fn clear_terminal_input(&mut self, session_id: SessionId) {
        self.terminal_input_drafts
            .retain(|draft| draft.session_id != session_id);
    }

    fn ensure_terminal_input_draft(&mut self, session_id: SessionId) -> &mut TerminalInputDraft {
        if let Some(index) = self
            .terminal_input_drafts
            .iter()
            .position(|draft| draft.session_id == session_id)
        {
            return &mut self.terminal_input_drafts[index];
        }

        self.terminal_input_drafts
            .push(TerminalInputDraft::new(session_id));
        self.terminal_input_drafts
            .last_mut()
            .expect("刚插入的终端输入草稿应该存在")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn terminal_input_draft_starts_empty() {
        let draft = TerminalInputDraft::new(SessionId(Uuid::new_v4()));

        assert!(draft.input.is_empty());
    }

    #[test]
    fn ui_state_terminal_input_messages_update_draft_only() {
        let mut ui = UiState::default();
        let first = SessionId(Uuid::new_v4());
        let second = SessionId(Uuid::new_v4());

        ui.set_terminal_input(first, "ls");
        ui.set_terminal_input(second, "pwd");
        ui.append_terminal_input(first, " -la");

        assert_eq!(ui.terminal_input_for(first), "ls -la");
        assert_eq!(ui.terminal_input_for(second), "pwd");
        assert_eq!(ui.terminal_input_for(SessionId(Uuid::new_v4())), "");

        ui.backspace_terminal_input(first);
        assert_eq!(ui.terminal_input_for(first), "ls -l");

        ui.clear_terminal_input(first);
        assert_eq!(ui.terminal_input_for(first), "");
        assert_eq!(ui.terminal_input_for(second), "pwd");
    }
}
