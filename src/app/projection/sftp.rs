//! Slint SFTP 面板写入。

use crate::app::AppWindow;
use crate::app::view_model::AppViewModel;

pub(super) fn sync_sftp_model(window: &AppWindow, model: &AppViewModel) {
    window.set_sftp_host_id(model.sftp.host_id.as_str().into());
    window.set_sftp_title(model.sftp.title.as_str().into());
    window.set_sftp_current_dir(model.sftp.current_dir.as_str().into());
    window.set_sftp_selected_path(model.sftp.selected_path.as_str().into());
    window.set_sftp_loading(model.sftp.loading);
    window.set_sftp_error(model.sftp.last_error.as_str().into());
}
