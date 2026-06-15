//! Known Hosts 工具页展示模型。

use crate::app::state::AsDesktopStateView;

use super::i18n::{locale_for_state, tr};
use super::tools_types::KnownHostViewModel;

pub(in crate::app::view_model) fn known_host_items(
    state: impl AsDesktopStateView,
) -> Vec<KnownHostViewModel> {
    let state = state.as_desktop_state_view();
    let locale = locale_for_state(state);
    state
        .storage
        .known_hosts
        .iter()
        .map(|entry| KnownHostViewModel {
            host: entry.host.clone(),
            port: entry.port,
            fingerprint: entry.fingerprint.clone(),
            status_key: if entry.trusted { "trusted" } else { "pending" }.to_owned(),
            status: if entry.trusted {
                tr(locale, "tool.known_host_trusted")
            } else {
                tr(locale, "tool.known_host_pending")
            }
            .to_owned(),
        })
        .collect()
}
