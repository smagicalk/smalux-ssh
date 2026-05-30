//! 快速新增主机草稿错误。

use std::fmt;

/// 快速新增主机表单校验错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuickHostDraftError {
    EmptyAddress,
    EmptyUsername,
    InvalidPort,
    MissingPasswordSecretRef,
    MissingPrivateKeyRef,
    MissingCertificateRef,
    MissingAgentPipePath,
}

impl fmt::Display for QuickHostDraftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAddress => f.write_str("地址不能为空"),
            Self::EmptyUsername => f.write_str("用户名不能为空"),
            Self::InvalidPort => f.write_str("端口必须是 1 到 65535"),
            Self::MissingPasswordSecretRef => f.write_str("密码引用不能为空"),
            Self::MissingPrivateKeyRef => f.write_str("私钥引用不能为空"),
            Self::MissingCertificateRef => f.write_str("证书引用不能为空"),
            Self::MissingAgentPipePath => f.write_str("自定义 agent 管道不能为空"),
        }
    }
}
