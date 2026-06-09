//! 主机动作入口回调。
//!
//! 这里只保留跨页面通用动作和连接类动作。主机表单、主机分组、凭据页面的细分回调放在
//! sibling 模块里，避免一个入口文件继续膨胀。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::model::{Message, ToolPanelMode};

use super::host_actions_helpers::copy_text_to_clipboard;
use super::{
    AppWindow, SharedAppState, apply_and_sync_without_drain, apply_messages_and_sync, parse_host_id,
};

pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
    window.on_copy_text_to_clipboard(|text| copy_text_to_clipboard(text.as_str()));

    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_shell(move |host_id| {
            // Slint 不认识 Rust 的 HostId，只传稳定字符串；解析失败说明 UI 状态已过期。
            let Some(host_id) = parse_host_id(&host_id) else {
                return;
            };
            // 连接 shell 后端可能较慢，这里跳过同步 drain，让 worker 先接管命令。
            apply_and_sync_without_drain(&weak, &state, Message::OpenShell { host_id });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_host_sftp(move |host_id| {
            let Some(host_id) = parse_host_id(&host_id) else {
                return;
            };
            // 打开 SFTP 同时切换右侧工具面板，避免 UI 侧做多个状态写入。
            apply_messages_and_sync(
                &weak,
                &state,
                [
                    Message::OpenSftp {
                        host_id,
                        initial_dir: "/".to_owned(),
                    },
                    Message::OpenToolPanel {
                        mode: ToolPanelMode::Sftp,
                    },
                ],
            );
        });
    }

    super::host_actions_quick_host::bind(window, Rc::clone(&state));
    super::host_actions_credentials::bind(window, state);
}
