//! 隧道规则的内存索引操作。

use smagical_core::TunnelRule;

use super::StorageManager;

impl StorageManager {
    /// 保存或更新隧道规则。
    pub fn upsert_tunnel_rule(&mut self, rule: TunnelRule) {
        let rule = rule.normalized();
        if let Some(existing) = self
            .tunnel_rules
            .iter_mut()
            .find(|existing| existing.name == rule.name)
        {
            *existing = rule;
        } else {
            self.tunnel_rules.push(rule);
        }
    }

    /// 按名称查找隧道规则。
    pub fn tunnel_rule_by_name(&self, name: &str) -> Option<&TunnelRule> {
        self.tunnel_rules.iter().find(|rule| rule.name == name)
    }

    /// 删除指定名称的隧道规则。
    pub fn remove_tunnel_rule(&mut self, name: &str) -> bool {
        let before = self.tunnel_rules.len();
        self.tunnel_rules.retain(|rule| rule.name != name);
        before != self.tunnel_rules.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smagical_core::{TunnelKind, TunnelRule};

    fn sample_tunnel_rule() -> TunnelRule {
        TunnelRule {
            name: "dynamic-proxy".to_owned(),
            kind: TunnelKind::Dynamic,
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 1080,
            target_host: "ignored-for-dynamic".to_owned(),
            target_port: 0,
            auto_start: false,
            exit_on_failure: false,
        }
    }

    #[test]
    fn tunnel_rules_can_be_found_and_removed_by_name() {
        let mut storage = StorageManager::default();

        storage.upsert_tunnel_rule(sample_tunnel_rule());

        assert!(storage.tunnel_rule_by_name("dynamic-proxy").is_some());
        assert!(storage.remove_tunnel_rule("dynamic-proxy"));
        assert!(storage.tunnel_rule_by_name("dynamic-proxy").is_none());
        assert!(!storage.remove_tunnel_rule("dynamic-proxy"));
    }

    #[test]
    fn upsert_tunnel_rule_replaces_same_name() {
        let mut storage = StorageManager::default();
        let mut updated = sample_tunnel_rule();
        updated.bind_port = 1081;

        storage.upsert_tunnel_rule(sample_tunnel_rule());
        storage.upsert_tunnel_rule(updated);

        assert_eq!(storage.tunnel_rule_count(), 1);
        assert_eq!(
            storage
                .tunnel_rule_by_name("dynamic-proxy")
                .map(|rule| rule.bind_port),
            Some(1081)
        );
    }

    #[test]
    fn upsert_tunnel_rule_normalizes_rule_identity() {
        let mut storage = StorageManager::default();
        let mut spaced = sample_tunnel_rule();
        spaced.name = " dynamic-proxy ".to_owned();
        spaced.bind_host = " 127.0.0.1 ".to_owned();
        spaced.target_host = " ignored-for-dynamic ".to_owned();
        spaced.bind_port = 1081;

        storage.upsert_tunnel_rule(sample_tunnel_rule());
        storage.upsert_tunnel_rule(spaced);

        assert_eq!(storage.tunnel_rule_count(), 1);
        let stored = storage
            .tunnel_rule_by_name("dynamic-proxy")
            .expect("规范化后的名称应该可以查到规则");
        assert_eq!(stored.bind_host, "127.0.0.1");
        assert_eq!(stored.target_host, "ignored-for-dynamic");
        assert_eq!(stored.bind_port, 1081);
    }
}
