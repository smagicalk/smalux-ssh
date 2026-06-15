//! SSH 端口转发和隧道模型。

use serde::{Deserialize, Serialize};

use crate::{HostId, SessionId};

/// 端口转发或动态隧道规则。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TunnelRule {
    pub name: String,
    pub kind: TunnelKind,
    pub bind_host: String,
    pub bind_port: u16,
    pub target_host: String,
    pub target_port: u16,
    pub auto_start: bool,
    #[serde(default)]
    pub exit_on_failure: bool,
}

impl TunnelRule {
    /// 返回去除首尾空白后的规则副本，保证运行态、标签页和后端请求使用同一套键值。
    pub fn normalized(&self) -> Self {
        let mut rule = self.clone();
        rule.name = rule.name.trim().to_owned();
        rule.bind_host = rule.bind_host.trim().to_owned();
        rule.target_host = rule.target_host.trim().to_owned();
        rule
    }

    /// 校验隧道规则是否具备启动所需的最小参数。
    pub fn validate(&self) -> Result<(), TunnelRuleValidationError> {
        if self.name.trim().is_empty() {
            return Err(TunnelRuleValidationError::EmptyName);
        }

        if self.bind_host.trim().is_empty() {
            return Err(TunnelRuleValidationError::EmptyBindHost);
        }

        if self.bind_port == 0 {
            return Err(TunnelRuleValidationError::EmptyBindPort);
        }

        if !matches!(self.kind, TunnelKind::Dynamic) {
            if self.target_host.trim().is_empty() {
                return Err(TunnelRuleValidationError::EmptyTargetHost);
            }

            if self.target_port == 0 {
                return Err(TunnelRuleValidationError::EmptyTargetPort);
            }
        }

        Ok(())
    }

    /// 生成适合标签页和列表展示的规则摘要。
    pub fn display_endpoint(&self) -> String {
        match self.kind {
            TunnelKind::Local => format!(
                "L {}:{} -> {}:{}",
                self.bind_host, self.bind_port, self.target_host, self.target_port
            ),
            TunnelKind::Remote => format!(
                "R {}:{} -> {}:{}",
                self.bind_host, self.bind_port, self.target_host, self.target_port
            ),
            TunnelKind::Dynamic => format!("D {}:{}", self.bind_host, self.bind_port),
        }
    }
}

/// 隧道规则校验错误。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TunnelRuleValidationError {
    EmptyName,
    EmptyBindHost,
    EmptyBindPort,
    EmptyTargetHost,
    EmptyTargetPort,
}

/// SSH 隧道方向。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TunnelKind {
    Local,
    Remote,
    Dynamic,
}

/// 单条隧道规则的运行态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TunnelRuntimeState {
    pub session_id: SessionId,
    pub rule_name: String,
    pub host_id: Option<HostId>,
    pub status: TunnelStatus,
    pub started_at_unix_secs: Option<u64>,
    pub last_error: Option<String>,
}

/// 隧道任务生命周期状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TunnelStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
}

impl TunnelStatus {
    /// 判断隧道运行态是否已经结束，终态不再接受迟到后端状态覆盖。
    pub fn is_terminal(&self) -> bool {
        matches!(self, TunnelStatus::Stopped | TunnelStatus::Failed)
    }

    /// 判断隧道是否可以发起停止命令。
    pub fn is_stoppable(&self) -> bool {
        matches!(self, TunnelStatus::Starting | TunnelStatus::Running)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HostId, SessionId};
    use uuid::Uuid;

    #[test]
    fn tunnel_rule_round_trips_through_toml() {
        let rule = TunnelRule {
            name: "local-db".to_owned(),
            kind: TunnelKind::Local,
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 15432,
            target_host: "10.0.0.5".to_owned(),
            target_port: 5432,
            auto_start: true,
            exit_on_failure: false,
        };

        let encoded = toml::to_string(&rule).expect("隧道规则应该可以序列化为 TOML");
        let decoded: TunnelRule =
            toml::from_str(&encoded).expect("隧道规则应该可以从 TOML 反序列化");

        assert_eq!(decoded.name, rule.name);
        assert!(matches!(decoded.kind, TunnelKind::Local));
        assert_eq!(decoded.bind_port, 15432);
        assert_eq!(decoded.target_port, 5432);
        assert!(decoded.auto_start);
    }

    #[test]
    fn tunnel_rule_validation_accepts_dynamic_without_target() {
        let rule = TunnelRule {
            name: "dynamic-proxy".to_owned(),
            kind: TunnelKind::Dynamic,
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 1080,
            target_host: String::new(),
            target_port: 0,
            auto_start: false,
            exit_on_failure: false,
        };

        assert_eq!(rule.validate(), Ok(()));
        assert_eq!(rule.display_endpoint(), "D 127.0.0.1:1080");
    }

    #[test]
    fn tunnel_rule_validation_rejects_missing_required_fields() {
        let empty_name = TunnelRule {
            name: String::new(),
            kind: TunnelKind::Local,
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 15432,
            target_host: "10.0.0.5".to_owned(),
            target_port: 5432,
            auto_start: false,
            exit_on_failure: false,
        };
        let empty_target = TunnelRule {
            name: "bad-local".to_owned(),
            kind: TunnelKind::Local,
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 15432,
            target_host: String::new(),
            target_port: 5432,
            auto_start: false,
            exit_on_failure: false,
        };

        assert_eq!(
            empty_name.validate(),
            Err(TunnelRuleValidationError::EmptyName)
        );
        assert_eq!(
            empty_target.validate(),
            Err(TunnelRuleValidationError::EmptyTargetHost)
        );
    }

    #[test]
    fn tunnel_rule_normalization_trims_identity_and_endpoints() {
        let rule = TunnelRule {
            name: " local-db ".to_owned(),
            kind: TunnelKind::Local,
            bind_host: " 127.0.0.1 ".to_owned(),
            bind_port: 15432,
            target_host: " 10.0.0.5 ".to_owned(),
            target_port: 5432,
            auto_start: false,
            exit_on_failure: false,
        };

        let normalized = rule.normalized();

        assert_eq!(normalized.name, "local-db");
        assert_eq!(normalized.bind_host, "127.0.0.1");
        assert_eq!(normalized.target_host, "10.0.0.5");
        assert_eq!(
            normalized.display_endpoint(),
            "L 127.0.0.1:15432 -> 10.0.0.5:5432"
        );
    }

    #[test]
    fn tunnel_runtime_state_round_trips_through_toml() {
        let state = TunnelRuntimeState {
            session_id: SessionId(Uuid::new_v4()),
            rule_name: "local-db".to_owned(),
            host_id: Some(HostId(Uuid::new_v4())),
            status: TunnelStatus::Running,
            started_at_unix_secs: Some(1_700_000_000),
            last_error: None,
        };

        let encoded = toml::to_string(&state).expect("隧道运行态应该可以序列化为 TOML");
        let decoded: TunnelRuntimeState =
            toml::from_str(&encoded).expect("隧道运行态应该可以从 TOML 反序列化");

        assert_eq!(decoded, state);
    }

    #[test]
    fn tunnel_status_lifecycle_helpers_are_centralized() {
        assert!(TunnelStatus::Stopped.is_terminal());
        assert!(!TunnelStatus::Starting.is_terminal());
        assert!(!TunnelStatus::Running.is_terminal());
        assert!(!TunnelStatus::Stopping.is_terminal());
        assert!(TunnelStatus::Failed.is_terminal());

        assert!(!TunnelStatus::Stopped.is_stoppable());
        assert!(TunnelStatus::Starting.is_stoppable());
        assert!(TunnelStatus::Running.is_stoppable());
        assert!(!TunnelStatus::Stopping.is_stoppable());
        assert!(!TunnelStatus::Failed.is_stoppable());
    }
}
