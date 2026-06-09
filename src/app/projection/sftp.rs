//! Slint SFTP 面板写入。

use crate::app::AppWindow;
use crate::app::view_model::AppViewModel;

pub(super) fn sync_sftp_model(window: &AppWindow, model: &AppViewModel) {
    let sftp = &model.terminal_workspace.sftp;
    window.set_sftp_host_id(sftp.host_id.as_str().into());
    window.set_sftp_title(sftp.title.as_str().into());
    window.set_sftp_current_dir(sftp.current_dir.as_str().into());
    window.set_sftp_selected_path(sftp.selected_path.as_str().into());
    window.set_sftp_loading(sftp.loading);
    window.set_sftp_error(sftp.last_error.as_str().into());
}
