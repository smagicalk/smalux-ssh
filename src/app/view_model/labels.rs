//! 展示层固定文案映射。

use crate::model::{
    AuthProfile, BuiltInTheme, HostListMode, SessionKind, SessionStatus, WorkspacePage,
};

pub(super) fn page_label(page: WorkspacePage) -> &'static str {
    match page {
        WorkspacePage::Hosts => "Hosts",
        WorkspacePage::Terminal => "Terminal",
        WorkspacePage::Sftp => "SFTP",
        WorkspacePage::Tunnels => "Tunnels",
        WorkspacePage::Snippets => "Snippets",
        WorkspacePage::History => "History",
        WorkspacePage::Security => "Security",
        WorkspacePage::Settings => "Settings",
    }
}

pub(super) fn theme_label(theme: BuiltInTheme) -> &'static str {
    match theme {
        BuiltInTheme::ProfessionalDark => "Professional Dark",
        BuiltInTheme::OceanDark => "Ocean Dark",
        BuiltInTheme::ForestDark => "Forest Dark",
    }
}

pub(super) fn host_list_mode_label(mode: HostListMode) -> &'static str {
    match mode {
        HostListMode::List => "List",
        HostListMode::Card => "Card",
    }
}

pub(super) fn auth_label(auth: &AuthProfile) -> &'static str {
    match auth {
        AuthProfile::Password { .. } => "Password",
        AuthProfile::Key { .. } => "Key",
        AuthProfile::Agent { .. } => "Agent",
        AuthProfile::Certificate { .. } => "Cert",
    }
}

pub(super) fn session_kind_label(kind: &SessionKind) -> &'static str {
    match kind {
        SessionKind::LocalShell => "Local",
        SessionKind::Shell => "Shell",
        SessionKind::RemoteCommand { .. } => "Command",
        SessionKind::Sftp => "SFTP",
        SessionKind::Tunnel { .. } => "Tunnel",
    }
}

pub(super) fn session_status_label(status: &SessionStatus) -> &'static str {
    match status {
        SessionStatus::Created => "Created",
        SessionStatus::Connecting => "Connecting",
        SessionStatus::Authenticating => "Authenticating",
        SessionStatus::Connected => "Connected",
        SessionStatus::RunningCommand => "Running",
        SessionStatus::Reconnecting => "Reconnecting",
        SessionStatus::Disconnected => "Disconnected",
        SessionStatus::Failed { .. } => "Failed",
    }
}
