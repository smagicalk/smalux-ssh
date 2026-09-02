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

impl TerminalSessionInfo {
    /// 转换为 Core 层标准化活跃终端上下文快照
    pub(crate) fn to_active_terminal_context(&self) -> smagical_core::ActiveTerminalSessionContext {
        if self.host_id.starts_with("local-") {
            smagical_core::ActiveTerminalSessionContext::local_shell(
                &self.session_id,
                &self.display_title,
            )
        } else {
            let (addr, port) = if let Some((a, p)) = self.host_address.split_once(':') {
                (a.to_string(), p.parse::<u16>().unwrap_or(22))
            } else {
                (self.host_address.clone(), 22)
            };
            smagical_core::ActiveTerminalSessionContext::ssh(
                &self.session_id,
                &self.host_id,
                &self.host_name,
                addr,
                port,
                "root",
                "默认分组",
                vec![],
            )
        }
    }
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
        w.set_active_host_id("".into());
        w.set_active_host_name("".into());
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
            w.set_active_host_id(active_sess.host_id.clone().into());
            w.set_active_host_name(active_sess.host_name.clone().into());
            w.set_active_host_address(active_sess.host_address.clone().into());
            w.set_active_host_ping_ms(active_sess.ping_ms);
            w.set_active_host_status(active_sess.host_status.clone().into());
        }
        w.set_active_pane_id(active_group.pane_id.clone().into());
        w.set_is_split(is_split);
        w.set_split_count(pane_groups.len() as i32);
    }
}

/// 将当前聚焦的终端活跃上下文同步至 CoreState，并自动触发全局 `TerminalFocusChangedEvent` 广播。
pub(crate) fn sync_active_session_to_core(
    pane_groups: &[PaneGroup],
    active_pane_id: &str,
    core_state: &smagical_core::CoreState,
) {
    if pane_groups.is_empty() {
        core_state.set_active_terminal(None);
    } else {
        let active_group = pane_groups
            .iter()
            .find(|g| g.pane_id == active_pane_id)
            .or_else(|| pane_groups.first());

        if let Some(g) = active_group
            && let Some(active_sess) = g.get_active_session()
        {
            core_state.set_active_terminal(Some(active_sess.to_active_terminal_context()));
        } else {
            core_state.set_active_terminal(None);
        }
    }
}


/// 会话退出异步持久化守护与应用退出等待守卫。
#[derive(Clone, Default)]
pub(crate) struct SessionPersistenceGuard {
    pending_handles: std::sync::Arc<std::sync::Mutex<Vec<std::thread::JoinHandle<()>>>>,
}

impl SessionPersistenceGuard {
    /// 派发一个异步后台会话历史与快照持久化任务
    pub(crate) fn spawn<F>(&self, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        if let Ok(handle) = std::thread::Builder::new()
            .name("session-history-flusher".into())
            .spawn(task)
            && let Ok(mut list) = self.pending_handles.lock()
        {
            list.retain(|h| !h.is_finished());
            list.push(handle);
        }

    }

    /// 应用即将退出时：阻塞等待所有未完成的后台持久化落盘任务（带最大超时保护）
    pub(crate) fn flush_and_wait(&self, timeout: std::time::Duration) {
        let start = std::time::Instant::now();
        let handles = if let Ok(mut list) = self.pending_handles.lock() {
            std::mem::take(&mut *list)
        } else {
            Vec::new()
        };

        for h in handles {
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                tracing::warn!(target: "smagical_ui::session", "会话后台持久化等待超时");
                break;
            }
            let _ = h.join();
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::terminal::split_tree::{SplitNode, SplitOrientation};

    fn make_test_session(id: &str, title: &str) -> TerminalSessionInfo {
        TerminalSessionInfo {
            session_id: id.to_string(),
            host_id: format!("host-{}", id),
            host_name: title.to_string(),
            host_address: "127.0.0.1:22".to_string(),
            host_status: "online".to_string(),
            ping_ms: 10,
            display_title: title.to_string(),
        }
    }

    #[test]
    fn test_same_pane_tab_reorder() {
        let s1 = make_test_session("s1", "Terminal 1");
        let s2 = make_test_session("s2", "Terminal 2");
        let s3 = make_test_session("s3", "Terminal 3");

        let mut group = PaneGroup {
            pane_id: "pane-1".to_string(),
            tabs: vec![s1.clone(), s2.clone(), s3.clone()],
            active_tab_id: "s1".to_string(),
        };

        // 将 s1 从 index 0 拖动到 index 2
        let moved = group.tabs.remove(0);
        let target_pos = 2usize.min(group.tabs.len());
        group.tabs.insert(target_pos, moved.clone());
        group.active_tab_id = moved.session_id.clone();

        assert_eq!(group.tabs.len(), 3);
        assert_eq!(group.tabs[0].session_id, "s2");
        assert_eq!(group.tabs[1].session_id, "s3");
        assert_eq!(group.tabs[2].session_id, "s1");
        assert_eq!(group.active_tab_id, "s1");
    }

    #[test]
    fn test_cross_pane_tab_move_with_remaining_tabs() {
        let s1 = make_test_session("s1", "Terminal 1");
        let s2 = make_test_session("s2", "Terminal 2");
        let s3 = make_test_session("s3", "Terminal 3");

        let mut groups = [
            PaneGroup {
                pane_id: "pane-1".to_string(),
                tabs: vec![s1.clone(), s2.clone()],
                active_tab_id: "s1".to_string(),
            },
            PaneGroup {
                pane_id: "pane-2".to_string(),
                tabs: vec![s3.clone()],
                active_tab_id: "s3".to_string(),
            },
        ];

        // 跨分屏将 s1 从 pane-1 移动到 pane-2
        let moved = groups[0].tabs.remove(0);
        assert!(!groups[0].tabs.is_empty());
        // 更新源窗格激活 Tab 为剩余的 s2
        groups[0].active_tab_id = groups[0].tabs[0].session_id.clone();

        // 插入目标窗格 pane-2 的首位
        groups[1].tabs.insert(0, moved.clone());
        groups[1].active_tab_id = moved.session_id.clone();

        assert_eq!(groups[0].tabs.len(), 1);
        assert_eq!(groups[0].tabs[0].session_id, "s2");
        assert_eq!(groups[0].active_tab_id, "s2");

        assert_eq!(groups[1].tabs.len(), 2);
        assert_eq!(groups[1].tabs[0].session_id, "s1");
        assert_eq!(groups[1].tabs[1].session_id, "s3");
        assert_eq!(groups[1].active_tab_id, "s1");
    }

    #[test]
    fn test_cross_pane_tab_move_last_tab_closes_source_pane() {
        let s1 = make_test_session("s1", "Terminal 1");
        let s2 = make_test_session("s2", "Terminal 2");

        let mut groups = vec![
            PaneGroup {
                pane_id: "pane-1".to_string(),
                tabs: vec![s1.clone()],
                active_tab_id: "s1".to_string(),
            },
            PaneGroup {
                pane_id: "pane-2".to_string(),
                tabs: vec![s2.clone()],
                active_tab_id: "s2".to_string(),
            },
        ];

        let mut split_tree = {
            let mut tree = SplitNode::new_single("pane-1".to_string(), "Pane 1".to_string());
            tree.split_pane("pane-1", "pane-2".to_string(), "Pane 2".to_string(), SplitOrientation::Vertical);
            Some(tree)
        };

        assert_eq!(split_tree.as_ref().unwrap().leaf_count(), 2);

        // 将 pane-1 的唯一一个 Tab s1 迁出到 pane-2
        let moved = groups[0].tabs.remove(0);
        let src_is_empty = groups[0].tabs.is_empty();
        assert!(src_is_empty);

        // 插入 pane-2
        groups[1].tabs.push(moved.clone());
        groups[1].active_tab_id = moved.session_id.clone();

        // 移除空的 pane-1 窗格并关闭分屏
        if src_is_empty {
            groups.remove(0);
            if let Some(tree) = split_tree.as_mut() {
                let closed = tree.close_pane("pane-1");
                assert!(closed);
                if tree.leaf_count() <= 1 {
                    split_tree = None;
                }
            }
        }

        // 验证：剩余窗格数为 1，分屏树已自动回缩为 None (退出分屏模式)，Tab 完整迁移至 pane-2
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].pane_id, "pane-2");
        assert_eq!(groups[0].tabs.len(), 2);
        assert_eq!(groups[0].tabs[0].session_id, "s2");
        assert_eq!(groups[0].tabs[1].session_id, "s1");
        assert_eq!(groups[0].active_tab_id, "s1");
        assert!(split_tree.is_none(), "分屏二叉树应在只剩 1 个窗格时自动回缩并退出分屏模式");
    }
}




