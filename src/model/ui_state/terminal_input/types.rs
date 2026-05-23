//! 终端输入草稿类型。

use serde::{Deserialize, Serialize};

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
