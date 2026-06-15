//! SFTP 传输取消调度。

use crate::core::CoreState;
use crate::model::TransferId;

use super::super::AppUpdateOutcome;
use cancel_commands::{has_pending_sftp_browser_refresh, remove_queued_sftp_transfer_command};
use cancel_loading::clear_loading_for_cancelled_transfer;
use cancel_lookup::{TransferLookup, unique_transfer_task};

#[path = "cancel_commands.rs"]
mod cancel_commands;
#[path = "cancel_loading.rs"]
mod cancel_loading;
#[path = "cancel_lookup.rs"]
mod cancel_lookup;

impl CoreState {
    /// 取消尚未交给后端执行器的 SFTP 传输。
    pub(in crate::model::app_state) fn cancel_sftp_transfer(
        &mut self,
        transfer_id: TransferId,
    ) -> AppUpdateOutcome {
        let task = match unique_transfer_task(&self.sessions.transfers, transfer_id) {
            TransferLookup::Found(task) => task,
            TransferLookup::Missing => {
                return AppUpdateOutcome {
                    error: Some(format!("找不到 SFTP 传输任务：{}", transfer_id.0)),
                    ..AppUpdateOutcome::default()
                };
            }
            TransferLookup::Ambiguous => {
                return AppUpdateOutcome {
                    error: Some(format!("SFTP 传输任务不唯一，无法取消：{}", transfer_id.0)),
                    ..AppUpdateOutcome::default()
                };
            }
        };

        if !task.status.is_queued() {
            return AppUpdateOutcome {
                error: Some("只能取消尚未开始的 SFTP 传输".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let removed_commands = remove_queued_sftp_transfer_command(
            &mut self.backend_commands,
            task.session_id,
            transfer_id,
        );
        if removed_commands == 0 {
            return AppUpdateOutcome {
                error: Some("SFTP 传输已经开始，无法从队列取消".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let has_pending_browser_refresh = has_pending_sftp_browser_refresh(
            &self.sessions,
            &self.backend_commands,
            task.host_id,
            task.session_id,
        );
        let transfer_cancelled = self
            .sessions
            .cancel_queued_transfer(task.session_id, transfer_id);
        let loading_cleared = clear_loading_for_cancelled_transfer(
            &mut self.sessions,
            &task,
            has_pending_browser_refresh,
        );

        AppUpdateOutcome {
            state_changed: transfer_cancelled || loading_cleared || removed_commands > 0,
            ..AppUpdateOutcome::default()
        }
    }
}
