//! 工作区布局尺寸与树形 UI 状态回调。
//!
//! 这里处理的是纯 UI 状态：列表/树切换、主机/凭据搜索、侧栏宽度和树节点展开。
//! 片段页 CRUD 和运行回调放在 sibling 模块里，避免布局 Adapter 继续承担业务动作。

use std::rc::Rc;

use crate::app::callbacks::{AppWindow, SharedAppState, apply_and_sync};
use crate::model::Message;
use slint::ComponentHandle;

use super::super::parse_optional_group_id;

pub(super) fn bind(window: &AppWindow, state: &SharedAppState) {
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_toggle_host_list_mode(move || {
            // 视图模式需要持久化到 UI 状态，后续可以保存用户偏好。
            apply_and_sync(&weak, &state, Message::ToggleHostListMode);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_toggle_host_tree_group(move |group_id| {
            // 根节点使用空 group id，具体解析规则由 ids helper 统一处理。
            let Some(group_id) = parse_optional_group_id(&group_id) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::ToggleHostTreeGroup { group_id });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_update_host_search(move |query| {
            // 搜索词保存在核心 UI 状态，树和卡片列表共用同一个过滤条件。
            apply_and_sync(
                &weak,
                &state,
                Message::UpdateHostSearchQuery {
                    query: query.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_update_credential_search(move |query| {
            // 密钥页搜索只影响密钥树投影，不改变持久化数据。
            apply_and_sync(
                &weak,
                &state,
                Message::UpdateCredentialSearchQuery {
                    query: query.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_toggle_credential_tree_node(move |node_id| {
            apply_and_sync(
                &weak,
                &state,
                Message::ToggleCredentialTreeNode {
                    node_id: node_id.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_toggle_right_sidebar(move || {
            apply_and_sync(&weak, &state, Message::ToggleRightSidebar);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_update_new_session_search(move |query| {
            // 新建会话弹窗有独立搜索词，避免影响主机首页筛选。
            apply_and_sync(
                &weak,
                &state,
                Message::UpdateNewSessionSearchQuery {
                    query: query.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_resize_hosts_panel(move |width| {
            // 宽度直接进入核心状态，便于以后做设置持久化或窗口恢复。
            apply_and_sync(&weak, &state, Message::ResizeHostsPanel { width });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_resize_activity_panel(move |width| {
            apply_and_sync(&weak, &state, Message::ResizeActivityPanel { width });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_resize_tool_panel(move |width| {
            apply_and_sync(&weak, &state, Message::ResizeToolPanel { width });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_next_background(move || {
            apply_and_sync(&weak, &state, Message::NextBackground);
        });
    }
}
