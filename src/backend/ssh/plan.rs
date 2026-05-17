//! SSH 连接执行计划。

use crate::security::{AuthResolver, ResolvedAuth, SecretStore, SecurityError};

use super::super::{BackendExecutionError, ConnectionTarget};

#[cfg(test)]
mod tests;

/// SSH 连接执行器可直接消费的连接计划。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshConnectionPlan {
    pub host: String,
    pub port: u16,
    pub endpoint: String,
    pub auth: SshAuthPlan,
}

impl SshConnectionPlan {
    /// 从后端连接目标和凭据存储创建执行计划。
    pub fn from_target<S: SecretStore>(
        target: &ConnectionTarget,
        store: &S,
    ) -> Result<Self, BackendExecutionError> {
        let resolver = AuthResolver::new(store);
        let auth = resolver
            .resolve(&target.auth)
            .map(SshAuthPlan::from)
            .map_err(|error| credential_error(target, error))?;

        Ok(Self {
            host: target.address.clone(),
            port: target.port,
            endpoint: target.endpoint(),
            auth,
        })
    }

    /// 返回认证用户名。
    pub fn username(&self) -> &str {
        self.auth.username()
    }
}

/// SSH 认证执行计划。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshAuthPlan {
    Password {
        username: String,
        password: String,
    },
    Key {
        username: String,
        private_key: String,
        passphrase: Option<String>,
    },
    Agent {
        username: String,
        key_hint: Option<String>,
    },
    Certificate {
        username: String,
        private_key: String,
        passphrase: Option<String>,
        certificate: String,
    },
}

impl SshAuthPlan {
    /// 返回认证用户名。
    pub fn username(&self) -> &str {
        match self {
            Self::Password { username, .. }
            | Self::Key { username, .. }
            | Self::Agent { username, .. }
            | Self::Certificate { username, .. } => username,
        }
    }

    /// 返回认证方式名称，便于日志和错误展示。
    pub fn method(&self) -> &'static str {
        match self {
            Self::Password { .. } => "password",
            Self::Key { .. } => "key",
            Self::Agent { .. } => "agent",
            Self::Certificate { .. } => "certificate",
        }
    }
}

impl From<ResolvedAuth> for SshAuthPlan {
    fn from(auth: ResolvedAuth) -> Self {
        match auth {
            ResolvedAuth::Password { username, password } => Self::Password { username, password },
            ResolvedAuth::Key {
                username,
                private_key,
                passphrase,
            } => Self::Key {
                username,
                private_key,
                passphrase,
            },
            ResolvedAuth::Agent { username, key_hint } => Self::Agent { username, key_hint },
            ResolvedAuth::Certificate {
                username,
                private_key,
                passphrase,
                certificate,
            } => Self::Certificate {
                username,
                private_key,
                passphrase,
                certificate,
            },
        }
    }
}

fn credential_error(target: &ConnectionTarget, error: SecurityError) -> BackendExecutionError {
    BackendExecutionError::AuthenticationFailed {
        username: target.auth.username().to_owned(),
        reason: error.to_string(),
    }
}
