//! 安全层错误类型。

use smagical_core::SecretRef;

/// 凭据解析和存取错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SecurityError {
    #[error("找不到凭据引用：{0:?}")]
    MissingSecret(SecretRef),
    #[error("系统凭据库错误：{0}")]
    Store(String),
}

impl SecurityError {
    /// 将底层错误包装为可展示的存储错误。
    pub fn store(error: impl std::fmt::Display) -> Self {
        Self::Store(error.to_string())
    }
}
