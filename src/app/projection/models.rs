//! Slint 列表模型转换。

use slint::{ModelRc, SharedString, VecModel};

use crate::app::view_model::{
    ActivityViewModel, CommandPaletteItemViewModel, HostViewModel, KnownHostViewModel,
    SessionTabViewModel, SftpEntryViewModel, ToolItemViewModel,
};
use crate::app::{
    ActivityRow, CommandPaletteRow, HostRow, KnownHostRow, SessionTabRow, SftpEntryRow, ToolItemRow,
};

pub(super) fn host_model(items: &[HostViewModel]) -> ModelRc<HostRow> {
    let rows = items
        .iter()
        .map(|host| HostRow {
            id: host.id.as_str().into(),
            name: host.name.as_str().into(),
            endpoint: host.endpoint.as_str().into(),
            auth: host.auth.into(),
            group: host.group.as_str().into(),
            tags: host.tags.as_str().into(),
            status: host.status.into(),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

pub(super) fn tab_model(items: &[SessionTabViewModel]) -> ModelRc<SessionTabRow> {
    let rows = items
        .iter()
        .map(|tab| SessionTabRow {
            id: tab.id.as_str().into(),
            title: tab.title.as_str().into(),
            kind: tab.kind.into(),
            status: tab.status.into(),
            active: tab.active,
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

pub(super) fn activity_model(items: &[ActivityViewModel]) -> ModelRc<ActivityRow> {
    let rows = items
        .iter()
        .map(|item| ActivityRow {
            label: item.label.into(),
            value: item.value.as_str().into(),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

pub(super) fn command_palette_model(
    items: &[CommandPaletteItemViewModel],
) -> ModelRc<CommandPaletteRow> {
    let rows = items
        .iter()
        .map(|item| CommandPaletteRow {
            id: item.id.as_str().into(),
            title: item.title.as_str().into(),
            subtitle: item.subtitle.as_str().into(),
            kind: item.kind.into(),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

pub(super) fn sftp_entry_model(items: &[SftpEntryViewModel]) -> ModelRc<SftpEntryRow> {
    let rows = items
        .iter()
        .map(|item| SftpEntryRow {
            name: item.name.as_str().into(),
            path: item.path.as_str().into(),
            kind: item.kind.into(),
            size: item.size.as_str().into(),
            selected: item.selected,
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

pub(super) fn tool_item_model(items: &[ToolItemViewModel]) -> ModelRc<ToolItemRow> {
    let rows = items
        .iter()
        .map(|item| ToolItemRow {
            title: item.title.as_str().into(),
            subtitle: item.subtitle.as_str().into(),
            meta: item.meta.as_str().into(),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

pub(super) fn known_host_model(items: &[KnownHostViewModel]) -> ModelRc<KnownHostRow> {
    let rows = items
        .iter()
        .map(|item| KnownHostRow {
            host: item.host.as_str().into(),
            port: i32::from(item.port),
            fingerprint: item.fingerprint.as_str().into(),
            status: item.status.as_str().into(),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

pub(super) fn string_model(items: &[String]) -> ModelRc<SharedString> {
    let rows = items
        .iter()
        .map(|item| SharedString::from(item.as_str()))
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}
