//! 凭据工具页入口展示模型。

use crate::app::state::AsDesktopStateView;

use super::i18n::locale_for_state;
use super::tools_credentials_common::{
    credential_kind_label, credential_visible_in_security, key_algorithm_label,
};
use super::tools_types::ToolItemViewModel;

pub(in crate::app::view_model) fn credential_items(
    state: impl AsDesktopStateView,
) -> Vec<ToolItemViewModel> {
    let state = state.as_desktop_state_view();
    let locale = locale_for_state(state);
    state
        .storage
        .credentials
        .iter()
        .filter(|credential| credential_visible_in_security(&credential.kind))
        .map(|credential| ToolItemViewModel {
            title: credential.name.clone(),
            subtitle: credential
                .username
                .clone()
                .unwrap_or_else(|| credential_kind_label(&credential.kind, locale).to_owned()),
            meta: credential
                .fingerprint
                .clone()
                .or_else(|| credential.key_algorithm.as_ref().map(key_algorithm_label))
                .unwrap_or_else(|| credential_kind_label(&credential.kind, locale).to_owned()),
        })
        .collect()
}
