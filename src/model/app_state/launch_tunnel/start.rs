//! 隧道启动调度。

use uuid::Uuid;

use crate::backend::{BackendCommand, TunnelStartRequest};
use crate::core::CoreState;
use crate::model::{HostId, SessionId, SessionStatus, TunnelRule};

use super::super::AppUpdateOutcome;
use super::super::launch::{
    connect_command_with_known_hosts, missing_host, queued_outcome, unix_now_secs,
};

impl CoreState {
    /// 启动端口转发或动态隧道的稳定核心入口。
    #[cfg_attr(not(feature = "desktop"), allow(dead_code))]
    pub(crate) fn start_tunnel_action(
        &mut self,
        host_id: HostId,
        rule: TunnelRule,
    ) -> AppUpdateOutcome {
        self.start_tunnel(host_id, rule)
    }

    /// 启动端口转发或动态隧道，并建立对应的管理标签页。
    pub(in crate::model::app_state) fn start_tunnel(
        &mut self,
        host_id: HostId,
        rule: TunnelRule,
    ) -> AppUpdateOutcome {
        let Some(host) = self.host_by_id(host_id) else {
            return missing_host(host_id);
        };
        let request = match TunnelStartRequest::new(rule) {
            Ok(request) => request,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(format!("隧道规则无效：{error:?}")),
                    ..AppUpdateOutcome::default()
                };
            }
        };
        if self.has_open_tunnel_tab(&request.rule.name) {
            return AppUpdateOutcome {
                error: Some(format!(
                    "隧道 {} 已有打开的标签页，请先关闭旧标签页再重新启动",
                    request.rule.name
                )),
                ..AppUpdateOutcome::default()
            };
        }

        let session_id = SessionId(Uuid::new_v4());
        self.sessions
            .open_tunnel_tab(session_id, host.id, &request.rule);
        self.sessions
            .set_status(session_id, SessionStatus::Connecting);
        self.sessions
            .start_tunnel(session_id, &request.rule, Some(host.id), unix_now_secs());
        self.record_recent_connection(&host);
        let known_hosts = self.storage.known_hosts.clone();
        self.backend_commands.extend([
            connect_command_with_known_hosts(session_id, &host, known_hosts),
            BackendCommand::StartTunnel {
                session_id,
                request,
            },
        ]);

        queued_outcome(2)
    }
}
