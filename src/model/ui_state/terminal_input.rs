//! 终端输入草稿。

use serde::{Deserialize, Serialize};

use super::SessionId;

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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn terminal_input_draft_starts_empty() {
        let draft = TerminalInputDraft::new(SessionId(Uuid::new_v4()));

        assert!(draft.input.is_empty());
    }
}
