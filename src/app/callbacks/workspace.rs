//! 工作区布局与侧边栏回调。
//!
//! 工作区回调覆盖布局开关、工具面板、主题切换和 Known Hosts 操作。这里仍然只做事件
//! 转发；布局尺寸、面板状态和安全记录都由核心状态保存。

use std::rc::Rc;

use slint::ComponentHandle;
use uuid::Uuid;

use crate::model::{ForwardId, HostId, JumpChainId, Message, ProxyId};

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
        window.on_save_network_proxy(move |asset_id, name, proxy_kind, host, port, tags| {
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
                },
            )
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_save_network_jump_chain(move |asset_id, name, host_ids| {
            let Some(chain_id) = parse_optional_jump_chain_id(asset_id.as_str()) else {
                return report_network_parse_error(&weak, &state);
            };
            let Some(host_ids) = parse_host_ids_text(host_ids.as_str()) else {
                return report_network_parse_error(&weak, &state);
            };
            apply_and_sync_success(
                &weak,
                &state,
                Message::SaveJumpChainAsset {
                    chain_id,
                    name: name.to_string(),
                    host_ids,
                },
            )
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
                  auto_start| {
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
                report_network_parse_error(&weak, &state);
                return;
            };
            apply_and_sync(&weak, &state, message);
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
        host_ids.push(HostId(Uuid::parse_str(token).ok()?));
    }
    Some(host_ids)
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
        crate::app::view_model::tr_for_state(&state_ref, "proxy.invalid_resource_id").to_owned()
    };
    state.borrow_mut().ui.set_last_error(message);
    super::sync_window(&window, &state.borrow());
    false
}
