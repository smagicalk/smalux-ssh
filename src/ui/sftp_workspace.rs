//! SFTP 浏览和传输工作区。
//!
//! 这里把目录浏览、上传、下载、删除和新建目录入口集中到一起，避免散在会话摘要里。

use iced::{
    Element, Length,
    widget::{button, column, row, text, text_input},
};

use crate::model::{AppState, Message, SftpActionDraftField, SftpEntryKind};

/// 渲染 SFTP 工作区。
pub fn view(state: &AppState) -> Element<'_, Message> {
    column![
        text("SFTP Workspace").size(22),
        bookmarks(state),
        browsers(state),
    ]
    .spacing(12)
    .into()
}

fn bookmarks(state: &AppState) -> Element<'_, Message> {
    let mut bookmarks = column![text("Bookmarks").size(18)].spacing(8);

    if state.storage.sftp_bookmarks.is_empty() {
        return bookmarks.push(text("No SFTP bookmarks.")).into();
    }

    for bookmark in &state.storage.sftp_bookmarks {
        bookmarks = bookmarks.push(
            row![
                text(format!(
                    "{} | {} | {}",
                    host_label(state, bookmark.host_id),
                    bookmark.label,
                    bookmark.remote_path
                ))
                .width(Length::Fill),
                button("Open").on_press(Message::OpenSftpBookmark {
                    host_id: bookmark.host_id,
                    remote_path: bookmark.remote_path.clone(),
                }),
                button("Remove").on_press(Message::RemoveSftpBookmark {
                    host_id: bookmark.host_id,
                    remote_path: bookmark.remote_path.clone(),
                }),
            ]
            .spacing(8),
        );
    }

    bookmarks.into()
}

fn browsers(state: &AppState) -> Element<'_, Message> {
    let mut browsers = column![text("Browsers").size(18)].spacing(10);

    if state.sessions.sftp_browsers.is_empty() {
        return browsers.push(text("No SFTP browser opened.")).into();
    }

    for browser in &state.sessions.sftp_browsers {
        browsers = browsers.push(browser_panel(state, browser));
    }

    browsers.into()
}

fn browser_panel<'a>(
    state: &'a AppState,
    browser: &'a crate::model::SftpBrowserState,
) -> Element<'a, Message> {
    let host_id = browser.host_id;
    let title = browser_title(state, host_id, &browser.current_dir);
    let loading_state = if browser.loading { "loading" } else { "ready" };
    let error_state = browser.last_error.as_deref().unwrap_or("ok");
    let parent = parent_remote_dir(&browser.current_dir);

    let mut content = column![
        row![
            text(title).width(Length::Fill),
            button("Bookmark").on_press(Message::SaveSftpBookmark { host_id }),
            button("Refresh").on_press(Message::RefreshSftp { host_id }),
            if browser.current_dir != "/" {
                button("Up").on_press(Message::NavigateSftp {
                    host_id,
                    remote_path: parent,
                })
            } else {
                button("Up")
            },
        ]
        .spacing(8),
        text(format!(
            "dir: {} | entries: {} | {} | {}",
            browser.current_dir,
            browser.entries.len(),
            loading_state,
            error_state
        )),
        browser_actions(state, host_id),
    ]
    .spacing(8);

    for entry in &browser.entries {
        content = content.push(entry_row(state, host_id, entry));
    }

    content.into()
}

fn browser_actions(state: &AppState, host_id: crate::model::HostId) -> Element<'_, Message> {
    let local_path = state.ui.sftp_local_path_for(host_id);
    let remote_name = state.ui.sftp_remote_name_for(host_id);
    let new_dir_name = state.ui.sftp_new_dir_name_for(host_id);

    column![
        row![
            text_input("local path", local_path)
                .on_input(move |value| Message::UpdateSftpActionDraft {
                    host_id,
                    field: SftpActionDraftField::LocalPath,
                    value,
                })
                .width(Length::Fill),
            text_input("remote name", remote_name)
                .on_input(move |value| Message::UpdateSftpActionDraft {
                    host_id,
                    field: SftpActionDraftField::RemoteName,
                    value,
                })
                .width(Length::Fill),
            text_input("new dir", new_dir_name)
                .on_input(move |value| Message::UpdateSftpActionDraft {
                    host_id,
                    field: SftpActionDraftField::NewDirName,
                    value,
                })
                .width(Length::Fill),
        ]
        .spacing(8),
        row![
            button("Upload").on_press(Message::UploadSftp { host_id }),
            button("Mkdir").on_press(Message::CreateSftpDir { host_id }),
        ]
        .spacing(8),
    ]
    .spacing(8)
    .into()
}

fn entry_row<'a>(
    state: &'a AppState,
    host_id: crate::model::HostId,
    entry: &'a crate::model::SftpEntry,
) -> Element<'a, Message> {
    let base = text(format!(
        "{} | {} | {}",
        entry.name,
        entry_kind_label(&entry.kind),
        entry
            .size
            .map(|size| size.to_string())
            .unwrap_or_else(|| "-".to_owned())
    ))
    .width(Length::Fill);

    let mut actions = row![].spacing(8);

    actions = actions.push(button("Select").on_press(Message::SelectSftpEntry {
        host_id,
        remote_path: entry.remote_path.clone(),
    }));

    if entry.is_navigable() {
        actions = actions.push(button("Open").on_press(Message::NavigateSftp {
            host_id,
            remote_path: entry.remote_path.clone(),
        }));
    }

    if matches!(entry.kind, SftpEntryKind::File) {
        let download_remote_path = entry.remote_path.clone();
        let delete_remote_path = entry.remote_path.clone();
        actions = actions
            .push(button("Download").on_press(Message::DownloadSftp {
                host_id,
                remote_path: download_remote_path,
            }))
            .push(button("Delete").on_press(Message::RemoveSftpFile {
                host_id,
                remote_path: delete_remote_path,
            }));
    }

    let selected = browser_selected_hint(state, host_id, &entry.remote_path);

    row![base, text(selected), actions].spacing(8).into()
}

fn entry_kind_label(kind: &SftpEntryKind) -> &'static str {
    match kind {
        SftpEntryKind::File => "file",
        SftpEntryKind::Directory => "dir",
        SftpEntryKind::Symlink => "link",
        SftpEntryKind::Other => "other",
    }
}

fn browser_title(state: &AppState, host_id: crate::model::HostId, current_dir: &str) -> String {
    state
        .sessions
        .tabs
        .iter()
        .rev()
        .find(|tab| {
            tab.host_id == Some(host_id) && matches!(tab.kind, crate::model::SessionKind::Sftp)
        })
        .map(|tab| tab.title.clone())
        .unwrap_or_else(|| format!("SFTP {current_dir}"))
}

fn host_label(state: &AppState, host_id: crate::model::HostId) -> String {
    state
        .storage
        .hosts
        .iter()
        .find(|host| host.id == host_id)
        .map(|host| host.name.clone())
        .unwrap_or_else(|| host_id.0.to_string())
}

fn browser_selected_hint(
    state: &AppState,
    host_id: crate::model::HostId,
    remote_path: &str,
) -> &'static str {
    state
        .sessions
        .sftp_browsers
        .iter()
        .find(|browser| browser.host_id == host_id)
        .and_then(|browser| browser.selected_path.as_deref())
        .filter(|selected| *selected == remote_path)
        .map(|_| "*")
        .unwrap_or("")
}

fn parent_remote_dir(remote_path: &str) -> String {
    let path = remote_path.trim_end_matches('/');
    match path.rfind('/') {
        Some(0) | None => "/".to_owned(),
        Some(index) => path[..index].to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AuthProfile, Host, HostId, SessionId, SessionStatus, SessionTab, SftpBookmark,
        SftpBrowserState, SftpEntry, SftpEntryKind,
    };
    use uuid::Uuid;

    fn host() -> Host {
        Host {
            id: HostId(Uuid::new_v4()),
            name: "production".to_owned(),
            group_id: None,
            tags: Vec::new(),
            address: "example.com".to_owned(),
            port: 22,
            auth: AuthProfile::Agent {
                username: "deploy".to_owned(),
                key_hint: None,
            },
            proxy: None,
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        }
    }

    #[test]
    fn sftp_workspace_view_accepts_default_state() {
        let state = AppState::default();

        let _element = view(&state);
    }

    #[test]
    fn sftp_workspace_view_accepts_populated_state() {
        let mut state = AppState::default();
        let host = host();
        let host_id = host.id;
        let session_id = SessionId(Uuid::new_v4());
        state.storage.upsert_host(host);
        state.storage.upsert_sftp_bookmark(SftpBookmark {
            host_id,
            label: "ops".to_owned(),
            remote_path: "/home/ops".to_owned(),
        });
        state.sessions.tabs.push(SessionTab {
            id: session_id,
            host_id: Some(host_id),
            kind: crate::model::SessionKind::Sftp,
            title: "SFTP /home/ops".to_owned(),
            status: SessionStatus::Connected,
        });
        state.sessions.sftp_browsers.push(SftpBrowserState {
            host_id,
            current_dir: "/home/ops".to_owned(),
            entries: vec![SftpEntry {
                name: "deploy.sh".to_owned(),
                remote_path: "/home/ops/deploy.sh".to_owned(),
                kind: SftpEntryKind::File,
                size: Some(2048),
                modified_at_unix_secs: None,
                permissions: None,
            }],
            selected_path: None,
            loading: false,
            last_error: None,
        });

        let _element = view(&state);
    }
}
