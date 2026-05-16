//! 展示模型公共辅助函数。

use crate::model::{AppState, Host, HostId};

pub(super) fn background_summary(state: &AppState) -> String {
    let background = state.config.background.normalized();
    format!(
        "{} sources · {:.0}% · blur {:.0}px",
        background.sources.len(),
        background.opacity * 100.0,
        background.blur
    )
}

pub(super) fn group_label(state: &AppState, host: &Host) -> String {
    host.group_id
        .and_then(|group_id| {
            state
                .storage
                .groups
                .iter()
                .find(|group| group.id == group_id)
                .map(|group| group.name.clone())
        })
        .unwrap_or_else(|| "Default".to_owned())
}

pub(super) fn tags_label(host: &Host) -> String {
    if host.tags.is_empty() {
        "untagged".to_owned()
    } else {
        host.tags.join(" / ")
    }
}

pub(super) fn host_name(state: &AppState, host_id: HostId) -> String {
    state
        .storage
        .hosts
        .iter()
        .find(|host| host.id == host_id)
        .map(|host| host.name.clone())
        .unwrap_or_else(|| "Unknown host".to_owned())
}

pub(super) fn bytes_label(size: Option<u64>) -> String {
    let Some(size) = size else {
        return "-".to_owned();
    };

    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let size = size as f64;

    if size >= GIB {
        format!("{:.1} GiB", size / GIB)
    } else if size >= MIB {
        format!("{:.1} MiB", size / MIB)
    } else if size >= KIB {
        format!("{:.1} KiB", size / KIB)
    } else {
        format!("{size:.0} B")
    }
}
