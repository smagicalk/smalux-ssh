//! 工作区布局与侧边栏回调。
//!
//! 工作区回调覆盖布局开关、工具面板、主题切换和 Known Hosts 操作。这里仍然只做事件
//! 转发；布局尺寸、面板状态和安全记录都由核心状态保存。

use std::rc::Rc;

use slint::{ComponentHandle, Model};
use uuid::Uuid;

use crate::model::{ForwardId, HostId, JumpChainId, Message, ProxyId};

use crate::app::HostRow;
use crate::app::state::AsDesktopStateView;
use crate::model::JumpProfile;

use super::{AppWindow, SharedAppState, apply_and_sync, apply_and_sync_success, parse_session_id};

#[path = "workspace/known_hosts.rs"]
mod known_hosts;
#[path = "workspace/layout.rs"]
mod layout;
#[path = "workspace/snippet_actions.rs"]
mod snippet_actions;
#[path = "workspace/snippet_helpers.rs"]
mod snippet_helpers;
#[path = "workspace/tool_panel.rs"]
mod tool_panel;

pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
    // 布局相关回调单独拆分，避免主工作区绑定文件继续膨胀。
    layout::bind(window, &state);
    snippet_actions::bind(window, &state);

    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_local_terminal(move || {
            // 本地终端和远程 shell 共享核心 session/terminal 模型。
            apply_and_sync(&weak, &state, Message::OpenLocalTerminal);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_tool_panel(move |mode| {
            // 工具面板 key 的解析集中在 tool_panel 子模块，保持这里只组织绑定。
            tool_panel::open_tool_panel(&weak, &state, &mode);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_close_tool_panel(move || {
            apply_and_sync(&weak, &state, Message::CloseToolPanel);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_next_theme(move || {
            apply_and_sync(&weak, &state, Message::NextTheme);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_trust_known_host(move |host, port| {
            // Known Hosts 端口来自 UI 数值，仍需要子模块校验范围和构造消息。
            let Some(message) = known_hosts::trust_known_host_message(&host, port) else {
                return;
            };
            apply_and_sync(&weak, &state, message);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_remove_known_host(move |host, port| {
            let Some(message) = known_hosts::remove_known_host_message(&host, port) else {
                return;
            };
            apply_and_sync(&weak, &state, message);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_stop_network_tunnel(move |session_id, rule_name| {
            let Some(session_id) = parse_session_id(&session_id) else {
                return;
            };
            apply_and_sync(
                &weak,
                &state,
                Message::StopTunnel {
                    session_id,
                    rule_name: rule_name.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_save_network_proxy(
            move |asset_id,
                  name,
                  proxy_kind,
                  host,
                  port,
                  tags,
                  auth_kind,
                  auth_username,
                  auth_password_ref,
                  remote_dns| {
                let Some(proxy_id) = parse_optional_proxy_id(asset_id.as_str()) else {
                    return report_network_parse_error(&weak, &state);
                };
                apply_and_sync_success(
                    &weak,
                    &state,
                    Message::SaveProxyAsset {
                        proxy_id,
                        name: name.to_string(),
                        proxy_kind: proxy_kind.to_string(),
                        host: host.to_string(),
                        port: port.to_string(),
                        tags: tags.to_string(),
                        auth_kind: auth_kind.to_string(),
                        auth_username: auth_username.to_string(),
                        auth_password_ref: auth_password_ref.to_string(),
                        remote_dns,
                    },
                )
            },
        );
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_save_network_jump_chain(move |asset_id, name, host_ids| {
            let Some(chain_id) = parse_optional_jump_chain_id(asset_id.as_str()) else {
                return report_network_parse_error(&weak, &state);
            };
            let Some(steps) = parse_jump_steps_text(host_ids.as_str()) else {
                return report_network_parse_error(&weak, &state);
            };
            apply_and_sync_success(
                &weak,
                &state,
                Message::SaveJumpChainAsset {
                    chain_id,
                    name: name.to_string(),
                    steps,
                },
            )
        });
    }
    window.on_network_jump_host_selected(|host_ids, host_id| {
        let Some(host_id) = parse_host_id_text(host_id.as_str()) else {
            return false;
        };
        parse_jump_steps_text(host_ids.as_str())
            .is_some_and(|steps| steps.iter().any(|step| step.host_id == host_id))
    });
    window.on_network_jump_host_order(|host_ids, host_id| {
        let Some(host_id) = parse_host_id_text(host_id.as_str()) else {
            return 0;
        };
        jump_host_order(host_ids.as_str(), host_id)
    });
    window.on_network_jump_host_matches(|query, host| host_row_matches_query(&query, &host));
    {
        let weak = window.as_weak();
        window.on_has_network_jump_host_match(move |query| {
            let Some(window) = weak.upgrade() else {
                return false;
            };
            let hosts = window.get_hosts();
            (0..hosts.row_count()).any(|index| {
                hosts
                    .row_data(index)
                    .is_some_and(|host| host_row_matches_query(&query, &host))
            })
        });
    }
    window.on_selected_network_jump_host_count(|host_ids| jump_step_count(host_ids.as_str()));
    window.on_clear_network_jump_hosts(|| "[]".into());
    window.on_toggle_network_jump_host(|host_ids, host_id| {
        let Some(host_id) = parse_host_id_text(host_id.as_str()) else {
            return host_ids;
        };
        toggle_jump_step_text(host_ids.as_str(), host_id).into()
    });
    window.on_update_network_jump_step_host(|step_text, step_index, host_id| {
        let Some(host_id) = parse_host_id_text(host_id.as_str()) else {
            return step_text;
        };
        update_jump_step_host_text(step_text.as_str(), step_index as usize, host_id).into()
    });
    window.on_update_network_jump_step_username(|step_text, step_index, username| {
        update_jump_step_username_text(step_text.as_str(), step_index as usize, username.as_str())
            .into()
    });
    window.on_update_network_jump_step_port(|step_text, step_index, port| {
        update_jump_step_port_text(step_text.as_str(), step_index as usize, port.as_str()).into()
    });
    window.on_update_network_jump_step_alias(|step_text, step_index, alias| {
        update_jump_step_alias_text(step_text.as_str(), step_index as usize, alias.as_str()).into()
    });
    window.on_move_jump_step_up(|step_text, step_index| {
        move_jump_step_text(step_text.as_str(), step_index as usize, true).into()
    });
    window.on_move_jump_step_down(|step_text, step_index| {
        move_jump_step_text(step_text.as_str(), step_index as usize, false).into()
    });
    window.on_network_jump_step_host_id(|step_text, step_index| {
        jump_step_host_id_text(step_text.as_str(), step_index as usize).into()
    });
    window.on_network_jump_step_username(|step_text, step_index| {
        jump_step_username_text(step_text.as_str(), step_index as usize).into()
    });
    window.on_network_jump_step_port(|step_text, step_index| {
        jump_step_port_text(step_text.as_str(), step_index as usize).into()
    });
    window.on_network_jump_step_alias(|step_text, step_index| {
        jump_step_alias_text(step_text.as_str(), step_index as usize).into()
    });
    {
        let weak = window.as_weak();
        window.on_network_jump_step_label(move |step_text, step_index| {
            let Some(window) = weak.upgrade() else {
                return "".into();
            };
            jump_step_label_text(step_text.as_str(), step_index as usize, &window).into()
        });
        let weak = window.as_weak();
        window.on_network_jump_step_endpoint(move |step_text, step_index| {
            let Some(window) = weak.upgrade() else {
                return "".into();
            };
            jump_step_endpoint_text(step_text.as_str(), step_index as usize, &window).into()
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_save_network_forward(
            move |asset_id,
                  name,
                  kind,
                  bind_host,
                  bind_port,
                  target_host,
                  target_port,
                  tags,
                  auto_start,
                  exit_on_failure| {
                let Some(forward_id) = parse_optional_forward_id(asset_id.as_str()) else {
                    return report_network_parse_error(&weak, &state);
                };
                apply_and_sync_success(
                    &weak,
                    &state,
                    Message::SaveForwardAsset {
                        forward_id,
                        name: name.to_string(),
                        kind: kind.to_string(),
                        bind_host: bind_host.to_string(),
                        bind_port: bind_port.to_string(),
                        target_host: target_host.to_string(),
                        target_port: target_port.to_string(),
                        tags: tags.to_string(),
                        auto_start,
                        exit_on_failure,
                    },
                )
            },
        );
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_remove_network_asset(move |kind_key, asset_id| {
            let Some(message) =
                parse_remove_network_asset_message(kind_key.as_str(), asset_id.as_str())
            else {
                return report_network_parse_error(&weak, &state);
            };
            apply_and_sync_success(&weak, &state, message)
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_dismiss_ui_error(move || {
            apply_and_sync(&weak, &state, Message::DismissUiError);
        });
    }
}

fn parse_optional_proxy_id(id: &str) -> Option<Option<ProxyId>> {
    parse_optional_uuid(id).map(|id| id.map(ProxyId))
}

fn parse_optional_jump_chain_id(id: &str) -> Option<Option<JumpChainId>> {
    parse_optional_uuid(id).map(|id| id.map(JumpChainId))
}

fn parse_optional_forward_id(id: &str) -> Option<Option<ForwardId>> {
    parse_optional_uuid(id).map(|id| id.map(ForwardId))
}

fn parse_optional_uuid(id: &str) -> Option<Option<Uuid>> {
    let id = id.trim();
    if id.is_empty() {
        Some(None)
    } else {
        Uuid::parse_str(id).ok().map(Some)
    }
}

fn parse_host_ids_text(text: &str) -> Option<Vec<HostId>> {
    let mut host_ids = Vec::new();
    for token in text.split(is_host_id_separator).map(str::trim) {
        if token.is_empty() {
            continue;
        }
        host_ids.push(parse_host_id_text(token)?);
    }
    Some(host_ids)
}

fn parse_jump_steps_text(text: &str) -> Option<Vec<crate::model::JumpProfile>> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Some(Vec::new());
    }
    if trimmed.starts_with('[') {
        serde_json::from_str::<Vec<JumpStepDraft>>(trimmed)
            .ok()
            .map(|steps| {
                steps
                    .into_iter()
                    .map(Into::into)
                    .filter(|step: &JumpProfile| step.host_id.0 != Uuid::nil())
                    .collect()
            })
    } else {
        let host_ids = parse_host_ids_text(trimmed)?;
        Some(
            host_ids
                .into_iter()
                .map(|host_id| JumpProfile {
                    host_id,
                    username_override: None,
                    port_override: None,
                    alias: None,
                })
                .collect(),
        )
    }
}

fn encode_jump_steps_text(steps: &[JumpProfile]) -> String {
    let drafts = steps
        .iter()
        .cloned()
        .map(JumpStepDraft::from)
        .collect::<Vec<_>>();
    serde_json::to_string(&drafts).unwrap_or_else(|_| "[]".to_owned())
}

fn toggle_jump_step_text(text: &str, host_id: HostId) -> String {
    let mut steps = parse_jump_steps_text(text).unwrap_or_default();
    if let Some(index) = steps
        .iter()
        .position(|existing| existing.host_id == host_id)
    {
        steps.remove(index);
    } else {
        steps.push(JumpProfile {
            host_id,
            username_override: None,
            port_override: None,
            alias: None,
        });
    }
    encode_jump_steps_text(&steps)
}

fn update_jump_step_host_text(text: &str, step_index: usize, host_id: HostId) -> String {
    let mut steps = parse_jump_steps_text(text).unwrap_or_default();
    let Some(step) = steps.get_mut(step_index) else {
        return text.to_owned();
    };
    step.host_id = host_id;
    encode_jump_steps_text(&steps)
}

fn update_jump_step_username_text(text: &str, step_index: usize, username: &str) -> String {
    let mut steps = parse_jump_steps_text(text).unwrap_or_default();
    let Some(step) = steps.get_mut(step_index) else {
        return text.to_owned();
    };
    let username = username.trim();
    step.username_override = (!username.is_empty()).then(|| username.to_owned());
    encode_jump_steps_text(&steps)
}

fn update_jump_step_port_text(text: &str, step_index: usize, port: &str) -> String {
    let mut steps = parse_jump_steps_text(text).unwrap_or_default();
    let Some(step) = steps.get_mut(step_index) else {
        return text.to_owned();
    };
    step.port_override = port.trim().parse::<u16>().ok().filter(|port| *port > 0);
    encode_jump_steps_text(&steps)
}

fn update_jump_step_alias_text(text: &str, step_index: usize, alias: &str) -> String {
    let mut steps = parse_jump_steps_text(text).unwrap_or_default();
    let Some(step) = steps.get_mut(step_index) else {
        return text.to_owned();
    };
    let alias = alias.trim();
    step.alias = (!alias.is_empty()).then(|| alias.to_owned());
    encode_jump_steps_text(&steps)
}

fn move_jump_step_text(text: &str, step_index: usize, move_up: bool) -> String {
    let mut steps = parse_jump_steps_text(text).unwrap_or_default();
    if step_index >= steps.len() {
        return text.to_owned();
    }
    if move_up {
        if step_index == 0 {
            return text.to_owned();
        }
        steps.swap(step_index, step_index - 1);
    } else {
        if step_index + 1 >= steps.len() {
            return text.to_owned();
        }
        steps.swap(step_index, step_index + 1);
    }
    encode_jump_steps_text(&steps)
}

fn jump_step_host_id_text(text: &str, step_index: usize) -> String {
    parse_jump_steps_text(text)
        .and_then(|steps| steps.get(step_index).map(|step| step.host_id.0.to_string()))
        .unwrap_or_default()
}

fn jump_step_username_text(text: &str, step_index: usize) -> String {
    parse_jump_steps_text(text)
        .and_then(|steps| {
            steps
                .get(step_index)
                .and_then(|step| step.username_override.clone())
        })
        .unwrap_or_default()
}

fn jump_step_port_text(text: &str, step_index: usize) -> String {
    parse_jump_steps_text(text)
        .and_then(|steps| {
            steps
                .get(step_index)
                .and_then(|step| step.port_override.map(|port| port.to_string()))
        })
        .unwrap_or_default()
}

fn jump_step_alias_text(text: &str, step_index: usize) -> String {
    parse_jump_steps_text(text)
        .and_then(|steps| steps.get(step_index).and_then(|step| step.alias.clone()))
        .unwrap_or_default()
}

fn jump_step_label_text(text: &str, step_index: usize, window: &AppWindow) -> String {
    let Some(step) = parse_jump_steps_text(text).and_then(|steps| steps.get(step_index).cloned())
    else {
        return String::new();
    };
    let hosts = window.get_hosts();
    let host_name = (0..hosts.row_count())
        .find_map(|index| hosts.row_data(index))
        .filter(|host| host.id.as_str() == step.host_id.0.to_string())
        .map(|host| host.name.to_string())
        .unwrap_or_else(|| step.host_id.0.to_string());
    step.alias.unwrap_or(host_name)
}

fn jump_step_endpoint_text(text: &str, step_index: usize, window: &AppWindow) -> String {
    let Some(step) = parse_jump_steps_text(text).and_then(|steps| steps.get(step_index).cloned())
    else {
        return String::new();
    };
    let hosts = window.get_hosts();
    (0..hosts.row_count())
        .find_map(|index| hosts.row_data(index))
        .filter(|host| host.id.as_str() == step.host_id.0.to_string())
        .map(|host| host.endpoint.to_string())
        .unwrap_or_default()
}

fn jump_host_order(text: &str, host_id: HostId) -> i32 {
    parse_jump_steps_text(text)
        .and_then(|steps| {
            steps
                .iter()
                .position(|existing| existing.host_id == host_id)
                .map(|index| index as i32 + 1)
        })
        .unwrap_or(0)
}

fn jump_step_count(text: &str) -> i32 {
    parse_jump_steps_text(text)
        .map(|steps| steps.len() as i32)
        .unwrap_or(0)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct JumpStepDraft {
    host_id: String,
    username_override: Option<String>,
    port_override: Option<u16>,
    alias: Option<String>,
}

impl From<JumpStepDraft> for JumpProfile {
    fn from(value: JumpStepDraft) -> Self {
        JumpProfile {
            host_id: HostId(Uuid::parse_str(value.host_id.trim()).unwrap_or_else(|_| Uuid::nil())),
            username_override: value.username_override,
            port_override: value.port_override,
            alias: value.alias,
        }
    }
}

impl From<JumpProfile> for JumpStepDraft {
    fn from(value: JumpProfile) -> Self {
        Self {
            host_id: value.host_id.0.to_string(),
            username_override: value.username_override,
            port_override: value.port_override,
            alias: value.alias,
        }
    }
}

fn parse_host_id_text(text: &str) -> Option<HostId> {
    Uuid::parse_str(text.trim()).ok().map(HostId)
}

fn host_row_matches_query(query: &str, host: &HostRow) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty()
        || host.name.to_lowercase().contains(&query)
        || host.endpoint.to_lowercase().contains(&query)
        || host.auth.to_lowercase().contains(&query)
        || host.group.to_lowercase().contains(&query)
        || host.tags.to_lowercase().contains(&query)
        || host.status.to_lowercase().contains(&query)
}

fn is_host_id_separator(ch: char) -> bool {
    ch == ',' || ch == ';' || ch == '，' || ch == '；' || ch.is_whitespace()
}

fn parse_remove_network_asset_message(kind_key: &str, asset_id: &str) -> Option<Message> {
    let id = Uuid::parse_str(asset_id.trim()).ok()?;
    match kind_key {
        "ProxyAsset" => Some(Message::RemoveProxyAsset {
            proxy_id: ProxyId(id),
        }),
        "JumpChainAsset" => Some(Message::RemoveJumpChainAsset {
            chain_id: JumpChainId(id),
        }),
        "ForwardAsset" => Some(Message::RemoveForwardAsset {
            forward_id: ForwardId(id),
        }),
        _ => None,
    }
}

fn report_network_parse_error(weak: &slint::Weak<AppWindow>, state: &SharedAppState) -> bool {
    let Some(window) = weak.upgrade() else {
        return false;
    };
    let message = {
        let state_ref = state.borrow();
        crate::app::view_model::tr_for_state(
            state_ref.as_desktop_state_view(),
            "proxy.invalid_resource_id",
        )
        .to_owned()
    };
    state.borrow_mut().ui.set_last_error(message);
    super::sync_window(&window, state.borrow().as_desktop_state_view());
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host_id(value: u128) -> HostId {
        HostId(Uuid::from_u128(value))
    }

    #[test]
    fn parse_host_ids_text_accepts_common_separators() {
        let first = host_id(1);
        let second = host_id(2);
        let text = format!("{}，{}\n{}", first.0, second.0, first.0);

        assert_eq!(parse_host_ids_text(&text), Some(vec![first, second, first]));
    }

    #[test]
    fn host_row_matches_query_checks_visible_host_fields() {
        let host = HostRow {
            id: "host-id".into(),
            name: "Production API".into(),
            endpoint: "prod.example.com:22".into(),
            icon_key: "server".into(),
            auth: "Agent".into(),
            group: "Backend".into(),
            group_id: "group-id".into(),
            group_header: "Backend".into(),
            group_header_id: "group-id".into(),
            tags: "prod api".into(),
            network_summary: "1 代理".into(),
            status_key: "Created".into(),
            status: "Ready".into(),
            accent_index: 0,
        };

        assert!(host_row_matches_query("", &host));
        assert!(host_row_matches_query("api", &host));
        assert!(host_row_matches_query("example.com", &host));
        assert!(host_row_matches_query("backend", &host));
        assert!(!host_row_matches_query("staging", &host));
    }

    #[test]
    fn toggle_jump_step_text_recovers_from_invalid_existing_text() {
        let host = host_id(7);

        assert_eq!(
            toggle_jump_step_text("not-a-uuid", host),
            format!(
                "[{{\"host_id\":\"{}\",\"username_override\":null,\"port_override\":null,\"alias\":null}}]",
                host.0
            )
        );
    }

    #[test]
    fn jump_step_helpers_track_selected_hosts() {
        let first = host_id(1);
        let second = host_id(2);

        let appended = toggle_jump_step_text("[]", first);
        let appended = toggle_jump_step_text(&appended, second);

        assert_eq!(jump_host_order(&appended, first), 1);
        assert_eq!(jump_host_order(&appended, second), 2);
        assert_eq!(jump_step_count(&appended), 2);
    }
}
