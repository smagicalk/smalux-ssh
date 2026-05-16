//! SFTP 面板展示模型。

use crate::model::{AppState, HostId, SftpEntryKind};

use super::common::{bytes_label, host_name};

/// SFTP 文件列表展示行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct SftpEntryViewModel {
    pub name: String,
    pub path: String,
    pub kind: &'static str,
    pub size: String,
    pub selected: bool,
}

/// 当前 SFTP 区域展示状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct SftpViewModel {
    pub host_id: String,
    pub title: String,
    pub current_dir: String,
    pub selected_path: String,
    pub loading: bool,
    pub last_error: String,
    pub entries: Vec<SftpEntryViewModel>,
}

pub(super) fn active_sftp(state: &AppState) -> SftpViewModel {
    let active_host_id = state
        .sessions
        .active_tab
        .and_then(|session_id| {
            state
                .sessions
                .tabs
                .iter()
                .find(|tab| tab.id == session_id)
                .and_then(|tab| tab.host_id)
        })
        .or_else(|| {
            state
                .sessions
                .sftp_browsers
                .last()
                .map(|browser| browser.host_id)
        });

    let Some(host_id) = active_host_id else {
        return empty_sftp();
    };

    let Some(browser) = state
        .sessions
        .sftp_browsers
        .iter()
        .find(|browser| browser.host_id == host_id)
        .or_else(|| state.sessions.sftp_browsers.last())
    else {
        return empty_sftp_for_host(state, host_id);
    };

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

fn sftp_kind_label(kind: &SftpEntryKind) -> &'static str {
    match kind {
        SftpEntryKind::File => "File",
        SftpEntryKind::Directory => "Dir",
        SftpEntryKind::Symlink => "Link",
        SftpEntryKind::Other => "Other",
    }
}
