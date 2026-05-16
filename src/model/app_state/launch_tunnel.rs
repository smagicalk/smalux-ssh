//! 端口转发和动态隧道启动/停止调度。

use uuid::Uuid;

use crate::backend::{BackendCommand, TunnelStartRequest, TunnelStopRequest};
use crate::model::{HostId, SessionId, SessionStatus, TunnelRule, WorkspacePage};

use super::launch::{connect_command, missing_host, queued_outcome, unix_now_secs};
use super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 启动端口转发或动态隧道，并建立对应的管理标签页。
    pub(super) fn start_tunnel(&mut self, host_id: HostId, rule: TunnelRule) -> AppUpdateOutcome {
        let Some(host) = self.host_by_id(host_id) else {
            return missing_host(host_id);
        };
        let request = match TunnelStartRequest::new(rule.clone()) {
            Ok(request) => request,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(format!("隧道规则无效：{error:?}")),
                    ..AppUpdateOutcome::default()
                };
            }
        };

        let session_id = SessionId(Uuid::new_v4());
        self.sessions.open_tunnel_tab(session_id, host.id, &rule);
        self.sessions
            .set_status(session_id, SessionStatus::Connecting);
        self.ui.workspace.active_page = WorkspacePage::Tunnels;
        self.sessions
            .start_tunnel(&rule, Some(host.id), unix_now_secs());
        self.record_recent_connection(&host);
        self.backend_commands.extend([
            connect_command(session_id, &host),
            BackendCommand::StartTunnel {
                session_id,
                request,
            },
        ]);

        queued_outcome(2)
    }

    /// 停止指定隧道规则。
    pub(super) fn stop_tunnel(
        &mut self,
        session_id: SessionId,
        rule_name: String,
    ) -> AppUpdateOutcome {
        self.sessions.mark_tunnel_stopping(&rule_name);
        self.backend_commands.push(BackendCommand::StopTunnel {
            session_id,
            request: TunnelStopRequest::by_name(rule_name),
        });

        queued_outcome(1)
    }
}
