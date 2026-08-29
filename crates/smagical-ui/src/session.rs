//! 终端会话管理与 Slint UI 同步。

use crate::generated::{AppWindow, TabData};


/// 活跃终端会话运行时信息。
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct TerminalSessionInfo {
    /// 会话唯一 ID (如: "sess-1")
    pub(crate) session_id: String,
    /// 对应主机或本地 Shell 的唯一 ID
    pub(crate) host_id: String,
    /// 主机或 Shell 的显示名称
    pub(crate) host_name: String,
    /// 连接地址 (远程: "host:port", 本地: "Local (bash)")
    pub(crate) host_address: String,
    /// 主机在线状态字符串
    pub(crate) host_status: String,
    /// 网络延迟 (ms)，本地 Shell 为 0
    pub(crate) ping_ms: i32,
    /// Tab 栏展示标题 (支持多开编号)
    pub(crate) display_title: String,
}

/// 将活跃会话列表与激活 Tab 状态同步至 Slint UI。
pub(crate) fn sync_active_session_ui(
    w: &AppWindow,
    sessions: &[TerminalSessionInfo],
    active_session_id: &str,
) {
    if sessions.is_empty() {
        w.set_tabs(slint::ModelRc::default());
        w.set_active_session_tab("".into());
        w.set_has_active_session(false);
        w.set_active_session_name("".into());
        w.set_active_host_address("".into());
        w.set_active_host_ping_ms(0);
        w.set_active_host_status("offline".into());
    } else {
        let active_sess = sessions
            .iter()
            .find(|s| s.session_id == active_session_id)
            .or_else(|| sessions.last())
            .unwrap();

        let tab_data: Vec<TabData> = sessions
            .iter()
            .map(|s| TabData {
                id: s.session_id.clone().into(),
                title: s.display_title.clone().into(),
                status: s.host_status.clone().into(),
            })
            .collect();

        w.set_tabs(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(tab_data))));
        w.set_active_session_tab(active_sess.session_id.clone().into());
        w.set_has_active_session(true);
        w.set_active_session_name(active_sess.display_title.clone().into());
        w.set_active_host_address(active_sess.host_address.clone().into());
        w.set_active_host_ping_ms(active_sess.ping_ms);
        w.set_active_host_status(active_sess.host_status.clone().into());
    }
}
