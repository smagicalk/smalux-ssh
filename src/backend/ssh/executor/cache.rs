//! SSH executor 运行态缓存管理。

use std::collections::HashMap;

use smagical_ssh_client_core::{is_channel_failure, is_sftp_failure};

use crate::backend::{BackendEvent, BackendExecutionError};
use crate::model::SessionId;

use super::super::RemoteTunnel;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct CachedSessionSubresources<TShell, TSftp> {
    pub(super) shell: Option<TShell>,
    pub(super) sftp: Option<TSftp>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct CachedSessionResources<TShell, TSftp, TConnection> {
    pub(super) shell: Option<TShell>,
    pub(super) sftp: Option<TSftp>,
    pub(super) connection: Option<TConnection>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct CachedSessionRuntimeResources<TShell, TSftp, TConnection, TTunnel> {
    pub(super) cached_resources: CachedSessionResources<TShell, TSftp, TConnection>,
    pub(super) tunnels: Vec<TTunnel>,
}

/// 一次性取出会话拥有的所有后端运行态，调用方随后负责关闭或停止。
pub(super) fn take_cached_session_runtime_resources<TShell, TSftp, TConnection, TTunnel>(
    shells: &mut HashMap<SessionId, TShell>,
    sftps: &mut HashMap<SessionId, TSftp>,
    connections: &mut HashMap<SessionId, TConnection>,
    tunnels: &mut HashMap<String, TTunnel>,
    session_id: SessionId,
) -> CachedSessionRuntimeResources<TShell, TSftp, TConnection, TTunnel>
where
    TTunnel: TunnelOwner,
{
    CachedSessionRuntimeResources {
        cached_resources: take_cached_session_resources(shells, sftps, connections, session_id),
        tunnels: take_tunnels_for_session(tunnels, session_id),
    }
}

pub(super) fn take_cached_session_subresources<TShell, TSftp>(
    shells: &mut HashMap<SessionId, TShell>,
    sftps: &mut HashMap<SessionId, TSftp>,
    session_id: SessionId,
) -> CachedSessionSubresources<TShell, TSftp> {
    CachedSessionSubresources {
        shell: shells.remove(&session_id),
        sftp: sftps.remove(&session_id),
    }
}

pub(super) fn take_cached_session_resources<TShell, TSftp, TConnection>(
    shells: &mut HashMap<SessionId, TShell>,
    sftps: &mut HashMap<SessionId, TSftp>,
    connections: &mut HashMap<SessionId, TConnection>,
    session_id: SessionId,
) -> CachedSessionResources<TShell, TSftp, TConnection> {
    CachedSessionResources {
        shell: shells.remove(&session_id),
        sftp: sftps.remove(&session_id),
        connection: connections.remove(&session_id),
    }
}

pub(super) fn replace_cached_shell<TShell>(
    shells: &mut HashMap<SessionId, TShell>,
    session_id: SessionId,
    shell: TShell,
) -> Option<TShell> {
    shells.insert(session_id, shell)
}

pub(super) fn replace_cached_sftp<TSftp>(
    sftps: &mut HashMap<SessionId, TSftp>,
    session_id: SessionId,
    sftp: TSftp,
) -> Option<TSftp> {
    sftps.insert(session_id, sftp)
}

pub(super) fn remove_tunnel_for_session_rule<TTunnel>(
    tunnels: &mut HashMap<String, TTunnel>,
    session_id: SessionId,
    rule_name: &str,
) -> Option<TTunnel>
where
    TTunnel: TunnelOwner,
{
    if !tunnels
        .get(rule_name)
        .is_some_and(|tunnel| tunnel.session_id() == session_id)
    {
        return None;
    }

    tunnels.remove(rule_name)
}

pub(super) trait TunnelOwner {
    fn session_id(&self) -> SessionId;
}

pub(super) trait StoppableTunnel {
    fn stop(&self);
}

impl TunnelOwner for RemoteTunnel {
    fn session_id(&self) -> SessionId {
        self.session_id()
    }
}

impl StoppableTunnel for RemoteTunnel {
    fn stop(&self) {
        RemoteTunnel::stop(self);
    }
}

pub(super) fn replace_tunnel_stopping_previous<TTunnel>(
    tunnels: &mut HashMap<String, TTunnel>,
    tunnel: TTunnel,
) where
    TTunnel: RuleNamedTunnel + StoppableTunnel,
{
    if let Some(previous) = tunnels.insert(tunnel.rule_name().to_owned(), tunnel) {
        previous.stop();
    }
}

pub(super) fn take_tunnels_for_session<TTunnel>(
    tunnels: &mut HashMap<String, TTunnel>,
    session_id: SessionId,
) -> Vec<TTunnel>
where
    TTunnel: TunnelOwner,
{
    let rule_names = tunnels
        .iter()
        .filter_map(|(rule_name, tunnel)| {
            (tunnel.session_id() == session_id).then(|| rule_name.clone())
        })
        .collect::<Vec<_>>();

    rule_names
        .into_iter()
        .filter_map(|rule_name| tunnels.remove(&rule_name))
        .collect()
}

pub(super) fn stop_detached_tunnels<TTunnel>(
    session_id: SessionId,
    tunnels: Vec<TTunnel>,
    operation: &'static str,
) where
    TTunnel: RuleNamedTunnel + StoppableTunnel,
{
    for tunnel in tunnels {
        let rule_name = tunnel.rule_name().to_owned();
        tunnel.stop();
        tracing::warn!(
            session_id = %session_id.0,
            operation,
            rule_name,
            "stopped detached SSH tunnel"
        );
    }
}

pub(super) trait RuleNamedTunnel {
    fn rule_name(&self) -> &str;
}

impl RuleNamedTunnel for RemoteTunnel {
    fn rule_name(&self) -> &str {
        RemoteTunnel::rule_name(self)
    }
}

pub(super) fn drop_cached_shell_after_failed_input<T>(
    shells: &mut HashMap<SessionId, T>,
    session_id: SessionId,
    result: &Result<(), BackendExecutionError>,
) -> bool {
    if !shell_input_result_requires_session_drop(result) {
        return false;
    }

    shells.remove(&session_id).is_some()
}

pub(super) fn shell_input_result_requires_session_drop(
    result: &Result<(), BackendExecutionError>,
) -> bool {
    result.as_ref().is_err_and(is_channel_failure)
}

pub(super) fn drop_cached_sftp_after_failed_request<T>(
    sftps: &mut HashMap<SessionId, T>,
    session_id: SessionId,
    result: &Result<Vec<BackendEvent>, BackendExecutionError>,
) -> bool {
    if !sftp_result_requires_session_drop(result) {
        return false;
    }

    sftps.remove(&session_id).is_some()
}

pub(super) fn sftp_result_requires_session_drop(
    result: &Result<Vec<BackendEvent>, BackendExecutionError>,
) -> bool {
    result.as_ref().is_err_and(is_sftp_failure)
}

pub(super) fn remote_shell_events_require_cache_drop(
    session_id: SessionId,
    events: &[BackendEvent],
) -> bool {
    events
        .iter()
        .any(|event| remote_shell_event_requires_cache_drop(session_id, event))
}

fn remote_shell_event_requires_cache_drop(session_id: SessionId, event: &BackendEvent) -> bool {
    if event.session_id() != session_id {
        return false;
    }

    event.is_terminal()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::BackendEvent;
    use smagical_ssh_client_core::{channel_reason_error, sftp_error};
    use uuid::Uuid;

    fn session_id() -> SessionId {
        SessionId(Uuid::new_v4())
    }

    #[test]
    fn taking_cached_session_resources_removes_only_target_session() {
        let target_session_id = session_id();
        let other_session_id = session_id();
        let mut shells = HashMap::from([
            (target_session_id, "target-shell"),
            (other_session_id, "other-shell"),
        ]);
        let mut sftps = HashMap::from([
            (target_session_id, "target-sftp"),
            (other_session_id, "other-sftp"),
        ]);
        let mut connections = HashMap::from([
            (target_session_id, "target-connection"),
            (other_session_id, "other-connection"),
        ]);

        let resources = take_cached_session_resources(
            &mut shells,
            &mut sftps,
            &mut connections,
            target_session_id,
        );

        assert_eq!(
            resources,
            CachedSessionResources {
                shell: Some("target-shell"),
                sftp: Some("target-sftp"),
                connection: Some("target-connection"),
            }
        );
        assert!(!shells.contains_key(&target_session_id));
        assert!(!sftps.contains_key(&target_session_id));
        assert!(!connections.contains_key(&target_session_id));
        assert_eq!(shells.get(&other_session_id), Some(&"other-shell"));
        assert_eq!(sftps.get(&other_session_id), Some(&"other-sftp"));
        assert_eq!(
            connections.get(&other_session_id),
            Some(&"other-connection")
        );
    }

    #[test]
    fn taking_cached_session_resources_is_idempotent_for_missing_session() {
        let missing_session_id = session_id();
        let other_session_id = session_id();
        let mut shells = HashMap::from([(other_session_id, "other-shell")]);
        let mut sftps = HashMap::from([(other_session_id, "other-sftp")]);
        let mut connections = HashMap::from([(other_session_id, "other-connection")]);

        let resources = take_cached_session_resources(
            &mut shells,
            &mut sftps,
            &mut connections,
            missing_session_id,
        );

        assert_eq!(
            resources,
            CachedSessionResources {
                shell: None::<&str>,
                sftp: None::<&str>,
                connection: None::<&str>,
            }
        );
        assert_eq!(shells.get(&other_session_id), Some(&"other-shell"));
        assert_eq!(sftps.get(&other_session_id), Some(&"other-sftp"));
        assert_eq!(
            connections.get(&other_session_id),
            Some(&"other-connection")
        );
    }

    #[test]
    fn taking_cached_session_resources_detaches_all_target_resources_before_close() {
        let session_id = session_id();
        let mut shells = HashMap::from([(session_id, "shell")]);
        let mut sftps = HashMap::from([(session_id, "sftp")]);
        let mut connections = HashMap::from([(session_id, "connection")]);

        let resources =
            take_cached_session_resources(&mut shells, &mut sftps, &mut connections, session_id);

        assert_eq!(
            resources,
            CachedSessionResources {
                shell: Some("shell"),
                sftp: Some("sftp"),
                connection: Some("connection"),
            }
        );
        assert!(shells.is_empty());
        assert!(sftps.is_empty());
        assert!(connections.is_empty());
    }

    #[test]
    fn taking_cached_session_runtime_resources_detaches_owned_tunnels() {
        let target_session_id = session_id();
        let other_session_id = session_id();
        let mut shells = HashMap::from([(target_session_id, "target-shell")]);
        let mut sftps = HashMap::from([(target_session_id, "target-sftp")]);
        let mut connections = HashMap::from([(target_session_id, "target-connection")]);
        let mut tunnels = HashMap::from([
            (
                "proxy".to_owned(),
                TestTunnel {
                    session_id: target_session_id,
                    rule_name: "proxy".to_owned(),
                    stopped: false,
                },
            ),
            (
                "metrics".to_owned(),
                TestTunnel {
                    session_id: other_session_id,
                    rule_name: "metrics".to_owned(),
                    stopped: false,
                },
            ),
        ]);

        let resources = take_cached_session_runtime_resources(
            &mut shells,
            &mut sftps,
            &mut connections,
            &mut tunnels,
            target_session_id,
        );

        assert_eq!(
            resources.cached_resources,
            CachedSessionResources {
                shell: Some("target-shell"),
                sftp: Some("target-sftp"),
                connection: Some("target-connection"),
            }
        );
        assert_eq!(resources.tunnels.len(), 1);
        assert_eq!(resources.tunnels[0].rule_name, "proxy");
        assert!(shells.is_empty());
        assert!(sftps.is_empty());
        assert!(connections.is_empty());
        assert!(!tunnels.contains_key("proxy"));
        assert_eq!(
            tunnels.get("metrics"),
            Some(&TestTunnel {
                session_id: other_session_id,
                rule_name: "metrics".to_owned(),
                stopped: false,
            })
        );
    }

    #[test]
    fn taking_cached_session_runtime_resources_is_idempotent_for_missing_session() {
        let missing_session_id = session_id();
        let other_session_id = session_id();
        let mut shells = HashMap::from([(other_session_id, "other-shell")]);
        let mut sftps = HashMap::from([(other_session_id, "other-sftp")]);
        let mut connections = HashMap::from([(other_session_id, "other-connection")]);
        let mut tunnels = HashMap::from([(
            "metrics".to_owned(),
            TestTunnel {
                session_id: other_session_id,
                rule_name: "metrics".to_owned(),
                stopped: false,
            },
        )]);

        let resources = take_cached_session_runtime_resources(
            &mut shells,
            &mut sftps,
            &mut connections,
            &mut tunnels,
            missing_session_id,
        );

        assert_eq!(
            resources.cached_resources,
            CachedSessionResources {
                shell: None::<&str>,
                sftp: None::<&str>,
                connection: None::<&str>,
            }
        );
        assert!(resources.tunnels.is_empty());
        assert_eq!(shells.get(&other_session_id), Some(&"other-shell"));
        assert_eq!(sftps.get(&other_session_id), Some(&"other-sftp"));
        assert_eq!(
            connections.get(&other_session_id),
            Some(&"other-connection")
        );
        assert_eq!(
            tunnels.get("metrics"),
            Some(&TestTunnel {
                session_id: other_session_id,
                rule_name: "metrics".to_owned(),
                stopped: false,
            })
        );
    }

    #[test]
    fn taking_cached_session_subresources_removes_only_target_session() {
        let target_session_id = session_id();
        let other_session_id = session_id();
        let mut shells = HashMap::from([
            (target_session_id, "target-shell"),
            (other_session_id, "other-shell"),
        ]);
        let mut sftps = HashMap::from([
            (target_session_id, "target-sftp"),
            (other_session_id, "other-sftp"),
        ]);

        let resources =
            take_cached_session_subresources(&mut shells, &mut sftps, target_session_id);

        assert_eq!(
            resources,
            CachedSessionSubresources {
                shell: Some("target-shell"),
                sftp: Some("target-sftp"),
            }
        );
        assert!(!shells.contains_key(&target_session_id));
        assert!(!sftps.contains_key(&target_session_id));
        assert_eq!(shells.get(&other_session_id), Some(&"other-shell"));
        assert_eq!(sftps.get(&other_session_id), Some(&"other-sftp"));
    }

    #[test]
    fn taking_cached_session_subresources_is_idempotent_for_missing_session() {
        let missing_session_id = session_id();
        let other_session_id = session_id();
        let mut shells = HashMap::from([(other_session_id, "other-shell")]);
        let mut sftps = HashMap::from([(other_session_id, "other-sftp")]);

        let resources =
            take_cached_session_subresources(&mut shells, &mut sftps, missing_session_id);

        assert_eq!(
            resources,
            CachedSessionSubresources {
                shell: None::<&str>,
                sftp: None::<&str>,
            }
        );
        assert_eq!(shells.get(&other_session_id), Some(&"other-shell"));
        assert_eq!(sftps.get(&other_session_id), Some(&"other-sftp"));
    }

    #[test]
    fn replacing_cached_shell_returns_previous_shell_for_same_session() {
        let session_id = session_id();
        let mut shells = HashMap::from([(session_id, "old-shell")]);

        let previous = replace_cached_shell(&mut shells, session_id, "new-shell");

        assert_eq!(previous, Some("old-shell"));
        assert_eq!(shells.get(&session_id), Some(&"new-shell"));
    }

    #[test]
    fn replacing_cached_shell_keeps_other_sessions() {
        let target_session_id = session_id();
        let other_session_id = session_id();
        let mut shells = HashMap::from([(other_session_id, "other-shell")]);

        let previous = replace_cached_shell(&mut shells, target_session_id, "target-shell");

        assert_eq!(previous, None);
        assert_eq!(shells.get(&target_session_id), Some(&"target-shell"));
        assert_eq!(shells.get(&other_session_id), Some(&"other-shell"));
    }

    #[test]
    fn replacing_cached_sftp_returns_previous_sftp_for_same_session() {
        let session_id = session_id();
        let mut sftps = HashMap::from([(session_id, "old-sftp")]);

        let previous = replace_cached_sftp(&mut sftps, session_id, "new-sftp");

        assert_eq!(previous, Some("old-sftp"));
        assert_eq!(sftps.get(&session_id), Some(&"new-sftp"));
    }

    #[test]
    fn replacing_cached_sftp_keeps_other_sessions() {
        let target_session_id = session_id();
        let other_session_id = session_id();
        let mut sftps = HashMap::from([(other_session_id, "other-sftp")]);

        let previous = replace_cached_sftp(&mut sftps, target_session_id, "target-sftp");

        assert_eq!(previous, None);
        assert_eq!(sftps.get(&target_session_id), Some(&"target-sftp"));
        assert_eq!(sftps.get(&other_session_id), Some(&"other-sftp"));
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct OwnedTunnel {
        session_id: SessionId,
    }

    impl TunnelOwner for OwnedTunnel {
        fn session_id(&self) -> SessionId {
            self.session_id
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestTunnel {
        session_id: SessionId,
        rule_name: String,
        stopped: bool,
    }

    impl TunnelOwner for TestTunnel {
        fn session_id(&self) -> SessionId {
            self.session_id
        }
    }

    impl RuleNamedTunnel for TestTunnel {
        fn rule_name(&self) -> &str {
            &self.rule_name
        }
    }

    impl StoppableTunnel for TestTunnel {
        fn stop(&self) {
            STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow_mut().push(self.rule_name.clone()));
        }
    }

    thread_local! {
        static STOPPED_TEST_TUNNEL_NAMES: std::cell::RefCell<Vec<String>> =
            const { std::cell::RefCell::new(Vec::new()) };
    }

    #[test]
    fn removing_tunnel_requires_matching_session_and_rule() {
        let owner_session_id = session_id();
        let stale_session_id = session_id();
        let other_session_id = session_id();
        let mut tunnels = HashMap::from([
            (
                "proxy".to_owned(),
                OwnedTunnel {
                    session_id: owner_session_id,
                },
            ),
            (
                "metrics".to_owned(),
                OwnedTunnel {
                    session_id: other_session_id,
                },
            ),
        ]);

        let stale = remove_tunnel_for_session_rule(&mut tunnels, stale_session_id, "proxy");
        let missing = remove_tunnel_for_session_rule(&mut tunnels, owner_session_id, "missing");
        let removed = remove_tunnel_for_session_rule(&mut tunnels, owner_session_id, "proxy");

        assert_eq!(stale, None);
        assert_eq!(missing, None);
        assert_eq!(
            removed,
            Some(OwnedTunnel {
                session_id: owner_session_id,
            })
        );
        assert!(!tunnels.contains_key("proxy"));
        assert_eq!(
            tunnels.get("metrics"),
            Some(&OwnedTunnel {
                session_id: other_session_id,
            })
        );
    }

    #[test]
    fn replacing_tunnel_stops_previous_same_rule() {
        let old_session_id = session_id();
        let new_session_id = session_id();
        let mut tunnels = HashMap::from([(
            "proxy".to_owned(),
            TestTunnel {
                session_id: old_session_id,
                rule_name: "proxy".to_owned(),
                stopped: false,
            },
        )]);
        STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow_mut().clear());

        replace_tunnel_stopping_previous(
            &mut tunnels,
            TestTunnel {
                session_id: new_session_id,
                rule_name: "proxy".to_owned(),
                stopped: false,
            },
        );

        assert_eq!(
            STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow().clone()),
            ["proxy"]
        );
        assert_eq!(
            tunnels.get("proxy"),
            Some(&TestTunnel {
                session_id: new_session_id,
                rule_name: "proxy".to_owned(),
                stopped: false,
            })
        );
    }

    #[test]
    fn replacing_tunnel_keeps_unrelated_rules_running() {
        let existing_session_id = session_id();
        let new_session_id = session_id();
        let mut tunnels = HashMap::from([(
            "metrics".to_owned(),
            TestTunnel {
                session_id: existing_session_id,
                rule_name: "metrics".to_owned(),
                stopped: false,
            },
        )]);
        STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow_mut().clear());

        replace_tunnel_stopping_previous(
            &mut tunnels,
            TestTunnel {
                session_id: new_session_id,
                rule_name: "proxy".to_owned(),
                stopped: false,
            },
        );

        assert!(STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow().is_empty()));
        assert!(tunnels.contains_key("metrics"));
        assert_eq!(
            tunnels.get("proxy"),
            Some(&TestTunnel {
                session_id: new_session_id,
                rule_name: "proxy".to_owned(),
                stopped: false,
            })
        );
    }

    #[test]
    fn taking_tunnels_for_session_removes_only_owned_tunnels() {
        let owner_session_id = session_id();
        let other_session_id = session_id();
        let mut tunnels = HashMap::from([
            (
                "proxy".to_owned(),
                TestTunnel {
                    session_id: owner_session_id,
                    rule_name: "proxy".to_owned(),
                    stopped: false,
                },
            ),
            (
                "db".to_owned(),
                TestTunnel {
                    session_id: owner_session_id,
                    rule_name: "db".to_owned(),
                    stopped: false,
                },
            ),
            (
                "metrics".to_owned(),
                TestTunnel {
                    session_id: other_session_id,
                    rule_name: "metrics".to_owned(),
                    stopped: false,
                },
            ),
        ]);

        let removed = take_tunnels_for_session(&mut tunnels, owner_session_id);

        assert_eq!(removed.len(), 2);
        assert!(removed.iter().any(|tunnel| tunnel.rule_name == "proxy"));
        assert!(removed.iter().any(|tunnel| tunnel.rule_name == "db"));
        assert!(!tunnels.contains_key("proxy"));
        assert!(!tunnels.contains_key("db"));
        assert_eq!(
            tunnels.get("metrics"),
            Some(&TestTunnel {
                session_id: other_session_id,
                rule_name: "metrics".to_owned(),
                stopped: false,
            })
        );
    }

    #[test]
    fn taking_tunnels_for_missing_session_keeps_all_tunnels() {
        let missing_session_id = session_id();
        let other_session_id = session_id();
        let mut tunnels = HashMap::from([(
            "metrics".to_owned(),
            TestTunnel {
                session_id: other_session_id,
                rule_name: "metrics".to_owned(),
                stopped: false,
            },
        )]);

        let removed = take_tunnels_for_session(&mut tunnels, missing_session_id);

        assert!(removed.is_empty());
        assert_eq!(
            tunnels.get("metrics"),
            Some(&TestTunnel {
                session_id: other_session_id,
                rule_name: "metrics".to_owned(),
                stopped: false,
            })
        );
    }

    #[test]
    fn stopping_detached_tunnels_stops_each_removed_tunnel() {
        let session_id = session_id();
        let tunnels = vec![
            TestTunnel {
                session_id,
                rule_name: "proxy".to_owned(),
                stopped: false,
            },
            TestTunnel {
                session_id,
                rule_name: "db".to_owned(),
                stopped: false,
            },
        ];
        STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow_mut().clear());

        stop_detached_tunnels(session_id, tunnels, "test");

        let mut stopped = STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow().clone());
        stopped.sort();
        assert_eq!(stopped, ["db", "proxy"]);
    }

    #[test]
    fn shell_input_failure_drops_only_failed_cached_shell() {
        let failed_session_id = session_id();
        let other_session_id = session_id();
        let result: Result<(), BackendExecutionError> =
            Err(channel_reason_error("shell input", "channel closed"));
        let mut cached_shells = HashMap::from([
            (failed_session_id, "failed-shell"),
            (other_session_id, "other-shell"),
        ]);

        let dropped =
            drop_cached_shell_after_failed_input(&mut cached_shells, failed_session_id, &result);

        assert!(dropped);
        assert!(!cached_shells.contains_key(&failed_session_id));
        assert_eq!(cached_shells.get(&other_session_id), Some(&"other-shell"));
    }

    #[test]
    fn shell_input_cache_survives_success_and_non_channel_failures() {
        let success_session_id = session_id();
        let sftp_failure_session_id = session_id();
        let success: Result<(), BackendExecutionError> = Ok(());
        let sftp_failure: Result<(), BackendExecutionError> =
            Err(sftp_error("list dir", "permission denied"));
        let mut cached_shells = HashMap::from([
            (success_session_id, "success-shell"),
            (sftp_failure_session_id, "sftp-failure-shell"),
        ]);

        let dropped_after_success =
            drop_cached_shell_after_failed_input(&mut cached_shells, success_session_id, &success);
        let dropped_after_sftp_failure = drop_cached_shell_after_failed_input(
            &mut cached_shells,
            sftp_failure_session_id,
            &sftp_failure,
        );

        assert!(!dropped_after_success);
        assert!(!dropped_after_sftp_failure);
        assert_eq!(
            cached_shells.get(&success_session_id),
            Some(&"success-shell")
        );
        assert_eq!(
            cached_shells.get(&sftp_failure_session_id),
            Some(&"sftp-failure-shell")
        );
    }

    #[test]
    fn shell_input_drop_gate_is_strict_about_channel_failures_only() {
        let channel_failure: Result<(), BackendExecutionError> =
            Err(channel_reason_error("shell input", "channel closed"));
        let sftp_failure: Result<(), BackendExecutionError> =
            Err(sftp_error("list dir", "permission denied"));
        let success: Result<(), BackendExecutionError> = Ok(());

        assert!(shell_input_result_requires_session_drop(&channel_failure));
        assert!(!shell_input_result_requires_session_drop(&sftp_failure));
        assert!(!shell_input_result_requires_session_drop(&success));
    }

    #[test]
    fn remote_shell_cache_drop_follows_shell_terminal_events() {
        let shell_session_id = session_id();
        let other_session_id = session_id();

        assert!(!remote_shell_events_require_cache_drop(
            shell_session_id,
            &[
                BackendEvent::Output {
                    session_id: shell_session_id,
                    line: "still running".to_owned(),
                },
                BackendEvent::SftpFailed {
                    session_id: shell_session_id,
                    reason: "unrelated sftp failure".to_owned(),
                },
            ],
        ));
        assert!(!remote_shell_events_require_cache_drop(
            shell_session_id,
            &[BackendEvent::Disconnected {
                session_id: other_session_id,
            }],
        ));
        assert!(remote_shell_events_require_cache_drop(
            shell_session_id,
            &[BackendEvent::CommandExited {
                session_id: shell_session_id,
                exit_code: Some(0),
            }],
        ));
        assert!(remote_shell_events_require_cache_drop(
            shell_session_id,
            &[BackendEvent::Failed {
                session_id: shell_session_id,
                reason: "channel failed".to_owned(),
            }],
        ));
        assert!(remote_shell_events_require_cache_drop(
            shell_session_id,
            &[BackendEvent::Disconnected {
                session_id: shell_session_id,
            }],
        ));
    }

    #[test]
    fn sftp_failure_drops_only_failed_cached_session() {
        let failed_session_id = session_id();
        let other_session_id = session_id();
        let result: Result<Vec<BackendEvent>, BackendExecutionError> =
            Err(sftp_error("list dir", "permission denied"));
        let mut cached_sftps = HashMap::from([
            (failed_session_id, "failed-session"),
            (other_session_id, "other-session"),
        ]);

        let dropped =
            drop_cached_sftp_after_failed_request(&mut cached_sftps, failed_session_id, &result);

        assert!(dropped);
        assert!(!cached_sftps.contains_key(&failed_session_id));
        assert_eq!(cached_sftps.get(&other_session_id), Some(&"other-session"));
    }

    #[test]
    fn sftp_cache_survives_success_and_non_sftp_failures() {
        let success_session_id = session_id();
        let channel_failure_session_id = session_id();
        let success: Result<Vec<BackendEvent>, BackendExecutionError> = Ok(Vec::new());
        let channel_failure: Result<Vec<BackendEvent>, BackendExecutionError> =
            Err(channel_reason_error("read", "channel closed"));
        let mut cached_sftps = HashMap::from([
            (success_session_id, "success-session"),
            (channel_failure_session_id, "channel-failure-session"),
        ]);

        let dropped_after_success =
            drop_cached_sftp_after_failed_request(&mut cached_sftps, success_session_id, &success);
        let dropped_after_channel_failure = drop_cached_sftp_after_failed_request(
            &mut cached_sftps,
            channel_failure_session_id,
            &channel_failure,
        );

        assert!(!dropped_after_success);
        assert!(!dropped_after_channel_failure);
        assert_eq!(
            cached_sftps.get(&success_session_id),
            Some(&"success-session")
        );
        assert_eq!(
            cached_sftps.get(&channel_failure_session_id),
            Some(&"channel-failure-session")
        );
    }

    #[test]
    fn sftp_drop_gate_is_strict_about_sftp_failures_only() {
        let sftp_failure: Result<Vec<BackendEvent>, BackendExecutionError> =
            Err(sftp_error("list dir", "permission denied"));
        let channel_failure: Result<Vec<BackendEvent>, BackendExecutionError> =
            Err(channel_reason_error("read", "channel closed"));
        let success: Result<Vec<BackendEvent>, BackendExecutionError> = Ok(Vec::new());

        assert!(sftp_result_requires_session_drop(&sftp_failure));
        assert!(!sftp_result_requires_session_drop(&channel_failure));
        assert!(!sftp_result_requires_session_drop(&success));
    }
}
