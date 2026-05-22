//! SFTP 取消传输后的浏览器 loading 清理。

use crate::model::{TransferDirection, TransferTask};
use crate::session::SessionManager;

pub(super) fn clear_loading_for_cancelled_transfer(
    sessions: &mut SessionManager,
    task: &TransferTask,
    has_pending_browser_refresh: bool,
) -> bool {
    if matches!(task.direction, TransferDirection::Upload) && !has_pending_browser_refresh {
        sessions.set_sftp_loading_for_session(task.session_id, false)
    } else {
        false
    }
}
