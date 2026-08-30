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

/// 单个分屏窗格内的 Tab 会话组 (Editor Group 模型)
#[derive(Clone, Debug)]
pub(crate) struct PaneGroup {
    /// 窗格唯一 ID (如: "pane-1", "pane-2")
    pub(crate) pane_id: String,
    /// 驻留在该窗格内的会话列表 (如: [Session A, Session C])
    pub(crate) tabs: Vec<TerminalSessionInfo>,
    /// 该窗格当前激活选中的会话 ID (如: "sess-A")
    pub(crate) active_tab_id: String,
}

impl PaneGroup {
    /// 创建仅包含初始会话的独立分屏组
    pub(crate) fn new_single(pane_id: String, initial_session: TerminalSessionInfo) -> Self {
        let active_id = initial_session.session_id.clone();
        Self {
            pane_id,
            tabs: vec![initial_session],
            active_tab_id: active_id,
        }
    }

    /// 获取当前窗格处于活跃激活状态的会话元数据
    pub(crate) fn get_active_session(&self) -> Option<&TerminalSessionInfo> {
        self.tabs.iter().find(|t| t.session_id == self.active_tab_id).or_else(|| self.tabs.last())
    }

    /// 将当前窗格的 Tab 序列转换为 Slint UI 数据模型
    pub(crate) fn to_tab_data_list(&self) -> Vec<TabData> {
        self.tabs.iter().map(|s| TabData {
            id: s.session_id.clone().into(),
            title: s.display_title.clone().into(),
            status: s.host_status.clone().into(),
        }).collect()
    }
}

/// 将活跃窗格组与全局会话状态同步至 Slint UI。
///
/// # 参数
/// - `w`: Slint 生成的主窗口引用
/// - `pane_groups`: 当前所有活跃分屏窗格组
/// - `active_pane_id`: 当前处于键盘/鼠标焦点的窗格 ID
/// - `is_split`: 当前是否处于多分屏排布模式
pub(crate) fn sync_active_session_ui(
    w: &AppWindow,
    pane_groups: &[PaneGroup],
    active_pane_id: &str,
    is_split: bool,
) {
    if pane_groups.is_empty() {
        w.set_tabs(slint::ModelRc::default());
        w.set_active_session_tab("".into());
        w.set_has_active_session(false);
        w.set_active_session_name("".into());
        w.set_active_host_address("".into());
        w.set_active_host_ping_ms(0);
        w.set_active_host_status("offline".into());
        w.set_is_split(false);
        w.set_split_count(1);
    } else {
        let active_group = pane_groups
            .iter()
            .find(|g| g.pane_id == active_pane_id)
            .or_else(|| pane_groups.first())
            .unwrap();

        if let Some(active_sess) = active_group.get_active_session() {
            w.set_tabs(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(active_group.to_tab_data_list()))));
            w.set_active_session_tab(active_sess.session_id.clone().into());
            w.set_has_active_session(true);
            w.set_active_session_name(active_sess.display_title.clone().into());
            w.set_active_host_address(active_sess.host_address.clone().into());
            w.set_active_host_ping_ms(active_sess.ping_ms);
            w.set_active_host_status(active_sess.host_status.clone().into());
        }
        w.set_active_pane_id(active_group.pane_id.clone().into());
        w.set_is_split(is_split);
        w.set_split_count(pane_groups.len() as i32);
    }
}


