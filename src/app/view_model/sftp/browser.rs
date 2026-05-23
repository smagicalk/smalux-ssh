//! SFTP 浏览器展示模型构建。

use crate::model::{AppState, HostId, SftpBrowserState};

use super::super::common::{bytes_label, host_name};
use super::labels::sftp_kind_label;
use super::types::{SftpEntryViewModel, SftpViewModel};

pub(in crate::app::view_model) fn active_sftp(state: &AppState) -> SftpViewModel {
    if let Some(host_id) = active_host_id(state) {
        return state
            .sessions
            .sftp_browsers
            .iter()
            .find(|browser| browser.host_id == host_id)
            .map(|browser| browser_view_model(state, browser))
            .unwrap_or_else(|| empty_sftp_for_host(state, host_id));
    }

    let Some(browser) = state.sessions.sftp_browsers.last() else {
        return empty_sftp();
    };

    browser_view_model(state, browser)
}

fn active_host_id(state: &AppState) -> Option<HostId> {
    state.sessions.active_tab.and_then(|session_id| {
        state
            .sessions
            .tabs
            .iter()
            .find(|tab| tab.id == session_id)
            .and_then(|tab| tab.host_id)
    })
}

fn browser_view_model(state: &AppState, browser: &SftpBrowserState) -> SftpViewModel {
    let selected_path = browser.selected_path.clone().unwrap_or_default();
    SftpViewModel {
        host_id: browser.host_id.0.to_string(),
        title: format!("SFTP · {}", host_name(state, browser.host_id)),
        current_dir: browser.current_dir.clone(),
        selected_path: selected_path.clone(),
        loading: browser.loading,
        last_error: browser.last_error.clone().unwrap_or_default(),
        entries: browser
            .entries
            .iter()
            .map(|entry| SftpEntryViewModel {
                name: entry.name.clone(),
                path: entry.remote_path.clone(),
                kind: sftp_kind_label(&entry.kind),
                size: bytes_label(entry.size),
                selected: entry.remote_path == selected_path,
            })
            .collect(),
    }
}

fn empty_sftp() -> SftpViewModel {
    SftpViewModel {
        host_id: String::new(),
        title: "SFTP Workspace".to_owned(),
        current_dir: "/".to_owned(),
        selected_path: String::new(),
        loading: false,
        last_error: String::new(),
        entries: Vec::new(),
    }
}

fn empty_sftp_for_host(state: &AppState, host_id: HostId) -> SftpViewModel {
    SftpViewModel {
        host_id: host_id.0.to_string(),
        title: format!("SFTP · {}", host_name(state, host_id)),
        ..empty_sftp()
    }
}
