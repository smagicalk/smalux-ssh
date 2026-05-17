//! SSH 隧道后端请求模型。

use crate::model::{TunnelKind, TunnelRule, TunnelRuleValidationError};

/// 后端启动隧道所需的已校验规则。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelStartRequest {
    pub rule: TunnelRule,
}

impl TunnelStartRequest {
    /// 从隧道规则构造启动请求，并复用模型层校验。
    pub fn new(rule: TunnelRule) -> Result<Self, TunnelRuleValidationError> {
        let rule = rule.normalized();
        rule.validate()?;
        Ok(Self { rule })
    }

    /// 是否需要目标主机和目标端口。
    pub fn requires_target(&self) -> bool {
        !matches!(self.rule.kind, TunnelKind::Dynamic)
    }
}

/// 后端停止隧道请求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelStopRequest {
    pub rule_name: String,
}

impl TunnelStopRequest {
    /// 创建按规则名称停止隧道的请求。
    pub fn by_name(rule_name: impl Into<String>) -> Self {
        Self {
            rule_name: rule_name.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tunnel_rule(kind: TunnelKind) -> TunnelRule {
        TunnelRule {
            name: "proxy".to_owned(),
            kind,
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 1080,
            target_host: "10.0.0.5".to_owned(),
            target_port: 5432,
            auto_start: false,
        }
    }

    #[test]
    fn tunnel_start_request_reuses_rule_validation() {
        let request = TunnelStartRequest::new(tunnel_rule(TunnelKind::Dynamic))
            .expect("有效动态隧道应该可以启动");

        assert!(!request.requires_target());

        let invalid = TunnelRule {
            name: String::new(),
            ..tunnel_rule(TunnelKind::Local)
        };

        assert_eq!(
            TunnelStartRequest::new(invalid),
            Err(TunnelRuleValidationError::EmptyName)
        );
    }

    #[test]
    fn tunnel_start_request_uses_normalized_rule() {
        let rule = TunnelRule {
            name: " proxy ".to_owned(),
            bind_host: " 127.0.0.1 ".to_owned(),
            target_host: " 10.0.0.5 ".to_owned(),
            ..tunnel_rule(TunnelKind::Local)
        };

        let request = TunnelStartRequest::new(rule).expect("修剪后有效的规则应该可以启动");

        assert_eq!(request.rule.name, "proxy");
        assert_eq!(request.rule.bind_host, "127.0.0.1");
        assert_eq!(request.rule.target_host, "10.0.0.5");
    }

    #[test]
    fn tunnel_stop_request_tracks_rule_name() {
        let request = TunnelStopRequest::by_name("proxy");

        assert_eq!(request.rule_name, "proxy");
    }
}
