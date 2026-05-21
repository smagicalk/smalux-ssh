//! 隧道停止调度。

use crate::backend::{BackendCommand, TunnelStopRequest};
use crate::model::{SessionId, TunnelStatus};

use super::super::launch::queued_outcome;
use super::super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 停止指定隧道规则。
    pub(in crate::model::app_state) fn stop_tunnel(
        &mut self,
        session_id: SessionId,
        rule_name: String,
    ) -> AppUpdateOutcome {
        let rule_name = rule_name.trim().to_owned();
        if rule_name.is_empty() {
            return AppUpdateOutcome {
                error: Some("隧道名称不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        if self.sessions.tunnel_status(&rule_name).is_none() {
            return AppUpdateOutcome {
                error: Some(format!("隧道 {rule_name} 没有可停止的运行态")),
                ..AppUpdateOutcome::default()
            };
        }
        if !self.tunnel_stop_target_matches_tab(session_id, &rule_name) {
            return AppUpdateOutcome {
                error: Some(format!("隧道 {rule_name} 与会话标签页不匹配")),
                ..AppUpdateOutcome::default()
            };
        }

        match self
            .sessions
            .tunnel_status_for_session(session_id, &rule_name)
        {
            Some(status) if status.is_stoppable() => {}
            Some(TunnelStatus::Stopping) => {
                return AppUpdateOutcome {
                    error: Some(format!("隧道 {rule_name} 正在停止，请等待后端确认")),
                    ..AppUpdateOutcome::default()
                };
            }
            Some(status) if status.is_terminal() => {
                return AppUpdateOutcome {
                    error: Some(format!("隧道 {rule_name} 已停止或失败，没有可停止的运行态")),
                    ..AppUpdateOutcome::default()
                };
            }
            Some(_) => {
                return AppUpdateOutcome {
                    error: Some(format!("隧道 {rule_name} 当前状态不可停止")),
                    ..AppUpdateOutcome::default()
                };
            }
            None => {
                return AppUpdateOutcome {
                    error: Some(format!("隧道 {rule_name} 没有当前会话的运行态")),
                    ..AppUpdateOutcome::default()
                };
            }
        }
        self.sessions.mark_tunnel_stopping(session_id, &rule_name);
        self.backend_commands.push(BackendCommand::StopTunnel {
            session_id,
            request: TunnelStopRequest::by_name(rule_name),
        });

        queued_outcome(1)
    }
}
