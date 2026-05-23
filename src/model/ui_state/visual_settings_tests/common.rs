use super::super::{BackgroundProfile, HostId, ThemeProfile};
use uuid::Uuid;

pub(super) fn host_id() -> HostId {
    HostId(Uuid::new_v4())
}

pub(super) fn theme() -> ThemeProfile {
    ThemeProfile {
        name: "Default Dark".to_owned(),
        font_family: "JetBrains Mono".to_owned(),
        font_size: 14.0,
    }
}

pub(super) fn background() -> BackgroundProfile {
    BackgroundProfile {
        enabled: false,
        sources: Vec::new(),
        rotation_interval_secs: 300,
        opacity: 0.18,
        blur: 8.0,
    }
}
