//! 展示层固定文案映射。

use crate::model::{
    AuthProfile, BuiltInTheme, HostListMode, SessionKind, SessionStatus, ToolPanelMode,
    WorkspacePage,
};

use super::i18n::{Locale, tr};

pub(super) fn page_label(page: WorkspacePage, locale: Locale) -> &'static str {
    match page {
        WorkspacePage::Hosts => tr(locale, "page.hosts"),
        WorkspacePage::Terminal => tr(locale, "page.terminal"),
        WorkspacePage::Sftp => tr(locale, "page.sftp"),
        WorkspacePage::Tunnels => tr(locale, "page.tunnels"),
        WorkspacePage::Snippets => tr(locale, "page.snippets"),
        WorkspacePage::History => tr(locale, "page.history"),
        WorkspacePage::Security => tr(locale, "page.security"),
        WorkspacePage::Settings => tr(locale, "page.settings"),
    }
}

pub(super) fn page_key(page: WorkspacePage) -> &'static str {
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

pub(super) fn theme_label(theme: BuiltInTheme, locale: Locale) -> &'static str {
    match theme {
        BuiltInTheme::ProfessionalDark => tr(locale, "theme.professional_dark"),
        BuiltInTheme::CatppuccinMocha => tr(locale, "theme.catppuccin_mocha"),
        BuiltInTheme::NordDark => tr(locale, "theme.nord_dark"),
        BuiltInTheme::Dracula => tr(locale, "theme.dracula"),
        BuiltInTheme::SolarizedDark => tr(locale, "theme.solarized_dark"),
        BuiltInTheme::OceanDark => tr(locale, "theme.ocean_dark"),
        BuiltInTheme::ForestDark => tr(locale, "theme.forest_dark"),
    }
}

pub(super) fn host_list_mode_label(mode: HostListMode, locale: Locale) -> &'static str {
    match mode {
        HostListMode::Tree => tr(locale, "mode.host_tree"),
        HostListMode::Card => tr(locale, "mode.host_card"),
    }
}

pub(super) fn host_list_mode_key(mode: HostListMode) -> &'static str {
    match mode {
        HostListMode::Tree => "Tree",
        HostListMode::Card => "Card",
    }
}

pub(super) fn tool_panel_mode_label(mode: ToolPanelMode, locale: Locale) -> &'static str {
    match mode {
        ToolPanelMode::Closed => tr(locale, "mode.tool_closed"),
        ToolPanelMode::Sftp => tr(locale, "mode.tool_sftp"),
        ToolPanelMode::Snippets => tr(locale, "mode.tool_snippets"),
        ToolPanelMode::History => tr(locale, "mode.tool_history"),
        ToolPanelMode::Tunnels => tr(locale, "mode.tool_tunnels"),
        ToolPanelMode::KnownHosts => tr(locale, "mode.tool_known_hosts"),
    }
}

pub(super) fn tool_panel_mode_key(mode: ToolPanelMode) -> &'static str {
    match mode {
        ToolPanelMode::Closed => "Closed",
        ToolPanelMode::Sftp => "SFTP",
        ToolPanelMode::Snippets => "Snippets",
        ToolPanelMode::History => "History",
        ToolPanelMode::Tunnels => "Tunnels",
        ToolPanelMode::KnownHosts => "KnownHosts",
    }
}

pub(super) fn auth_label(auth: &AuthProfile, locale: Locale) -> &'static str {
    match auth {
        AuthProfile::Password { .. } => tr(locale, "auth.password"),
        AuthProfile::Key { .. } => tr(locale, "auth.key"),
        AuthProfile::Agent { .. } => tr(locale, "auth.agent"),
        AuthProfile::Certificate { .. } => tr(locale, "auth.certificate"),
    }
}

pub(super) fn session_kind_label(kind: &SessionKind, locale: Locale) -> &'static str {
    match kind {
        SessionKind::LocalShell => tr(locale, "session.kind.local"),
        SessionKind::Shell => tr(locale, "session.kind.shell"),
        SessionKind::RemoteCommand { .. } => tr(locale, "session.kind.command"),
        SessionKind::Sftp => tr(locale, "session.kind.sftp"),
        SessionKind::Tunnel { .. } => tr(locale, "session.kind.tunnel"),
    }
}

pub(super) fn session_status_label(status: &SessionStatus, locale: Locale) -> &'static str {
    match status {
        SessionStatus::Created => tr(locale, "session.status.created"),
        SessionStatus::Connecting => tr(locale, "session.status.connecting"),
        SessionStatus::Authenticating => tr(locale, "session.status.authenticating"),
        SessionStatus::Connected => tr(locale, "session.status.connected"),
        SessionStatus::RunningCommand => tr(locale, "session.status.running"),
        SessionStatus::Reconnecting => tr(locale, "session.status.reconnecting"),
        SessionStatus::Disconnected => tr(locale, "session.status.disconnected"),
        SessionStatus::Failed { .. } => tr(locale, "session.status.failed"),
    }
}
