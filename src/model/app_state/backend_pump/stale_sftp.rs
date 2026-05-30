//! 后端泵中过期 SFTP 命令的状态收尾。
//!
//! SFTP 命令的 UI 副作用分三类：目录浏览 loading、目录写操作错误、上传/下载传输进度。
//! 命令过期时要按类型收尾，否则会出现 loading 不消失或传输条永远进行中的问题。

use crate::backend::{BackendCommand, SftpRequest};
use crate::model::SessionId;

use super::super::transfers::{failed_transfer_for_command, transfer_failed_event};
use super::super::{AppState, AppUpdateOutcome};

impl AppState {
    pub(super) fn skip_stale_sftp_command(
        &mut self,
        session_id: SessionId,
        request: &SftpRequest,
        command: &BackendCommand,
    ) -> AppUpdateOutcome {
        match request {
            // 浏览命令过期只需要取消 SFTP 面板的 loading。
            SftpRequest::ListDir { .. } => AppUpdateOutcome {
                state_changed: self
                    .sessions
                    .set_sftp_loading_for_session(session_id, false),
                ..AppUpdateOutcome::default()
            },
            // 创建目录/删除文件没有 transfer id，只能给浏览器区域写入失败原因。
            SftpRequest::RemoveFile { .. } | SftpRequest::CreateDir { .. } => AppUpdateOutcome {
                state_changed: self
                    .sessions
                    .fail_sftp_browser_for_session(session_id, "SFTP 会话已结束，操作未执行"),
                ..AppUpdateOutcome::default()
            },
            // 上传/下载需要更新独立的 transfer 状态。
            _ => self.skip_stale_sftp_transfer_command(session_id, command),
        }
    }

    fn skip_stale_sftp_transfer_command(
        &mut self,
        session_id: SessionId,
        command: &BackendCommand,
    ) -> AppUpdateOutcome {
        let Some(transfer) = failed_transfer_for_command(command) else {
            return AppUpdateOutcome::default();
        };
        // 复用正常进度事件，让 UI 传输列表不用区分“后端失败”和“过期未执行”。
        let event_outcome = self.apply_backend_event(transfer_failed_event(
            transfer,
            "SFTP 会话已结束，传输未执行".to_owned(),
        ));
        // 同时清掉浏览器 loading，防止传输失败后面板还显示忙碌。
        let loading_cleared = self
            .sessions
            .set_sftp_loading_for_session(session_id, false);
        AppUpdateOutcome {
            state_changed: event_outcome.state_changed || loading_cleared,
            applied_backend_events: event_outcome.applied_backend_events,
            ..AppUpdateOutcome::default()
        }
    }
}
