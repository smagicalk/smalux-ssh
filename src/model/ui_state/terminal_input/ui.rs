//! 终端输入草稿 UI 状态访问。

use super::super::UiState;
use super::TerminalInputDraft;
use crate::model::SessionId;

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
