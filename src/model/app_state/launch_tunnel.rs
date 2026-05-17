//! 端口转发和动态隧道启动/停止调度。

use uuid::Uuid;

use crate::backend::{BackendCommand, TunnelStartRequest, TunnelStopRequest};
use crate::model::{
    HostId, SessionId, SessionKind, SessionStatus, TunnelRule, TunnelStatus, WorkspacePage,
};

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
        match self.sessions.tunnel_status(&rule_name) {
            Some(TunnelStatus::Starting | TunnelStatus::Running) => {}
            Some(TunnelStatus::Stopping) => {
                return AppUpdateOutcome {
                    error: Some(format!("隧道 {rule_name} 正在停止，请等待后端确认")),
                    ..AppUpdateOutcome::default()
                };
            }
            Some(TunnelStatus::Stopped | TunnelStatus::Failed) => {
                return AppUpdateOutcome {
                    error: Some(format!("隧道 {rule_name} 已停止或失败，没有可停止的运行态")),
                    ..AppUpdateOutcome::default()
                };
            }
            None => {
                return AppUpdateOutcome {
                    error: Some(format!("隧道 {rule_name} 没有可停止的运行态")),
                    ..AppUpdateOutcome::default()
                };
            }
        }

        self.sessions.mark_tunnel_stopping(&rule_name);
        self.backend_commands.push(BackendCommand::StopTunnel {
            session_id,
            request: TunnelStopRequest::by_name(rule_name),
        });

        queued_outcome(1)
    }

    /// 判断同名隧道标签是否已经打开，避免后端同名隧道互相覆盖。
    fn has_open_tunnel_tab(&self, rule_name: &str) -> bool {
        self.sessions.tabs.iter().any(|tab| {
            matches!(
                &tab.kind,
                SessionKind::Tunnel {
                    rule_name: existing_rule_name,
                } if existing_rule_name == rule_name
            )
        })
    }
}
