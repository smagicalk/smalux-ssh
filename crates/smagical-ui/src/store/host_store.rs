//! 主机与分组资产集中式 Store (HostStore)。
//!
//! 统一聚合主控树形镜像、平铺卡片、多层展开集合与搜索过滤状态，
//! 并将原本在 UI 线程中的纯 CPU 密集型树节点递归遍历与宽字符计算下沉至 Tokio 线程池并发计算。

use std::collections::HashSet;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use slint::{ComponentHandle, Model};
use smagical_core::event::{EventDispatcher, HostSearchFilteredEvent};

use crate::generated::{AppWindow, HostItemData, HostsBridge};
use crate::store::diff::{compute_card_diff, compute_tree_diff};
use crate::tree_model::{
    build_group_options, build_search_tree_nodes, build_visible_tree_nodes,
    calculate_max_tree_width, RawTreeNode,
};

/// 集中式主机资产管理 Store
#[derive(Clone)]
pub(crate) struct HostStore {
    /// 内存全量主机/分组树形原始节点缓存 (Master Tree)
    pub(crate) master_tree: Arc<RwLock<Vec<RawTreeNode>>>,
    /// 内存全量卡片模式主机列表数据缓存 (Master Cards)
    pub(crate) master_cards: Arc<RwLock<Vec<HostItemData>>>,
    /// 树形视图当前已展开的分组 ID 集合 (Expanded Group IDs)
    pub(crate) expanded_groups: Arc<RwLock<HashSet<String>>>,
    /// 新建/编辑主机弹窗中上级分组树选择器已展开的分组 ID 集合
    pub(crate) selector_expanded_groups: Arc<RwLock<HashSet<String>>>,
    /// 侧边栏主机搜索栏当前输入的过滤关键词 (Search Query)
    pub(crate) search_query: Arc<RwLock<String>>,
    /// 搜索与计算递增版本序列号（防止后台异步任务乱序投递导致输入回退）
    search_version: Arc<AtomicU64>,
}

impl HostStore {
    /// 创建全新的主机集中式 Store
    #[allow(dead_code)]
    pub(crate) fn new(
        master_tree: Vec<RawTreeNode>,
        master_cards: Vec<HostItemData>,
        expanded_groups: HashSet<String>,
        selector_expanded_groups: HashSet<String>,
    ) -> Self {
        Self {
            master_tree: Arc::new(RwLock::new(master_tree)),
            master_cards: Arc::new(RwLock::new(master_cards)),
            expanded_groups: Arc::new(RwLock::new(expanded_groups)),
            selector_expanded_groups: Arc::new(RwLock::new(selector_expanded_groups)),
            search_query: Arc::new(RwLock::new(String::new())),
            search_version: Arc::new(AtomicU64::new(0)),
        }
    }

    /// 从现有共享引用包装为 Store（平滑兼容现有 AppContext）
    pub(crate) fn from_arcs(
        master_tree: Arc<RwLock<Vec<RawTreeNode>>>,
        master_cards: Arc<RwLock<Vec<HostItemData>>>,
        expanded_groups: Arc<RwLock<HashSet<String>>>,
        selector_expanded_groups: Arc<RwLock<HashSet<String>>>,
        search_query: Arc<RwLock<String>>,
    ) -> Self {
        Self {
            master_tree,
            master_cards,
            expanded_groups,
            selector_expanded_groups,
            search_query,
            search_version: Arc::new(AtomicU64::new(0)),
        }
    }

    /// 切换树形分组展开/折叠状态，并返回是否为展开
    pub(crate) fn toggle_group(&self, group_id: &str) -> bool {
        let mut exp = self.expanded_groups.write().unwrap();
        if exp.contains(group_id) {
            exp.remove(group_id);
            false
        } else {
            exp.insert(group_id.to_string());
            true
        }
    }

    /// 切换弹窗选择器分组展开/折叠状态
    pub(crate) fn toggle_selector_group(&self, group_id: &str) -> bool {
        let mut sel = self.selector_expanded_groups.write().unwrap();
        if sel.contains(group_id) {
            sel.remove(group_id);
            false
        } else {
            sel.insert(group_id.to_string());
            true
        }
    }

    /// 设置搜索查询字符串，并递增版本号用于并发防抖
    pub(crate) fn set_search_query(&self, query: String) -> u64 {
        let mut q = self.search_query.write().unwrap();
        *q = query;
        self.search_version.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// 当前搜索版本号
    #[allow(dead_code)]
    pub(crate) fn current_version(&self) -> u64 {
        self.search_version.load(Ordering::SeqCst)
    }

    /// 【优化点 1 & 4】：后台纯 CPU 密集运算下沉与局部增量刷新
    ///
    /// 将高耗时的全量树遍历、搜索过滤与宽字符计算移至 Tokio 后台计算线程池执行，
    /// 算完后利用局部增量比对引擎 (Diff Engine) 判定差异，按需投递回 Slint 事件循环。
    pub(crate) fn schedule_search_compute(
        &self,
        query: String,
        window_weak: slint::Weak<AppWindow>,
        event_dispatcher: Option<Arc<EventDispatcher>>,
    ) {
        let req_version = self.set_search_query(query.clone());
        let tree_snapshot = self.master_tree.read().unwrap().clone();
        let cards_snapshot = self.master_cards.read().unwrap().clone();
        let expanded_snapshot = self.expanded_groups.read().unwrap().clone();
        let version_atomic = Arc::clone(&self.search_version);

        // 下沉至后台阻塞计算线程池，0 毫秒占用 UI 主线程
        tokio::task::spawn_blocking(move || {
            let q = query.trim().to_lowercase();

            // 1. 在后台线程执行树形过滤或可见性展开
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree_snapshot, &expanded_snapshot)
            } else {
                build_search_tree_nodes(&tree_snapshot, &q)
            };

            // 2. 在后台线程执行宽字符字宽计算
            let content_width = calculate_max_tree_width(&next_nodes);

            // 3. 在后台线程执行卡片平铺列表过滤
            let filtered_cards: Vec<HostItemData> = if q.is_empty() {
                cards_snapshot
            } else {
                cards_snapshot
                    .into_iter()
                    .filter(|h| {
                        h.name.to_lowercase().contains(&q)
                            || h.address.to_lowercase().contains(&q)
                            || h.group.to_lowercase().contains(&q)
                    })
                    .collect()
            };

            // 投递回 UI 线程
            let _ = slint::invoke_from_event_loop(move || {
                // 防抖校验：若期间又有更新的搜索请求到达，放弃落后版本的重绘
                if version_atomic.load(Ordering::SeqCst) != req_version {
                    return;
                }

                if let Some(w) = window_weak.upgrade() {
                    let hb = w.global::<HostsBridge>();

                    // 利用局部增量比对引擎比对树节点
                    let current_tree = hb.get_tree_nodes();
                    let current_count = current_tree.row_count();
                    let mut old_nodes = Vec::with_capacity(current_count);
                    for i in 0..current_count {
                        if let Some(n) = current_tree.row_data(i) {
                            old_nodes.push(n);
                        }
                    }

                    let tree_diffs = compute_tree_diff(&old_nodes, &next_nodes);
                    if !tree_diffs.is_empty() {
                        hb.set_tree_content_width(content_width);
                        hb.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(
                            next_nodes,
                        ))));
                    }

                    // 利用局部增量比对引擎比对卡片列表
                    let current_cards = hb.get_hosts();
                    let current_card_count = current_cards.row_count();
                    let mut old_cards = Vec::with_capacity(current_card_count);
                    for i in 0..current_card_count {
                        if let Some(c) = current_cards.row_data(i) {
                            old_cards.push(c);
                        }
                    }

                    let card_diffs = compute_card_diff(&old_cards, &filtered_cards);
                    if !card_diffs.is_empty() {
                        hb.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(
                            filtered_cards.clone(),
                        ))));
                    }

                    // 派发搜索匹配完成事件
                    if let Some(ref disp) = event_dispatcher {
                        disp.dispatch(&HostSearchFilteredEvent {
                            query: q.clone(),
                            match_count: filtered_cards.len(),
                        });
                    }

                    if !q.is_empty() {
                        tracing::debug!(target: "smagical_ui::search", "后台并发计算完成，匹配到 {} 台主机", filtered_cards.len());
                    }
                }
            });
        });
    }

    /// 【优化点 1 & 4】：后台并发刷新树形结构与选择器选项
    pub(crate) fn schedule_tree_refresh(&self, window_weak: slint::Weak<AppWindow>) {
        let tree_snapshot = self.master_tree.read().unwrap().clone();
        let expanded_snapshot = self.expanded_groups.read().unwrap().clone();
        let selector_snapshot = self.selector_expanded_groups.read().unwrap().clone();
        let q = self.search_query.read().unwrap().clone();

        tokio::task::spawn_blocking(move || {
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree_snapshot, &expanded_snapshot)
            } else {
                build_search_tree_nodes(&tree_snapshot, &q)
            };
            let content_width = calculate_max_tree_width(&next_nodes);
            let next_options = build_group_options(&tree_snapshot, &selector_snapshot);

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(w) = window_weak.upgrade() {
                    let hb = w.global::<HostsBridge>();

                    let current_tree = hb.get_tree_nodes();
                    let current_count = current_tree.row_count();
                    let mut old_nodes = Vec::with_capacity(current_count);
                    for i in 0..current_count {
                        if let Some(n) = current_tree.row_data(i) {
                            old_nodes.push(n);
                        }
                    }

                    let tree_diffs = compute_tree_diff(&old_nodes, &next_nodes);
                    if !tree_diffs.is_empty() {
                        hb.set_tree_content_width(content_width);
                        hb.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(
                            next_nodes,
                        ))));
                    }

                    hb.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(
                        next_options,
                    ))));
                }
            });
        });
    }

    /// 【优化点 4】：单主机状态或延迟原地更新（微秒级局部刷新）
    ///
    /// 仅修改特定节点的 status 与 ping_ms，避免全树重新排版计算
    #[allow(dead_code)]
    pub(crate) fn update_host_status_inplace(
        &self,
        host_id: &str,
        new_status: &str,
        ping_ms: i32,
        window_weak: slint::Weak<AppWindow>,
    ) {
        // 1. 更新内存原始树
        {
            let mut tree = self.master_tree.write().unwrap();
            if let Some(node) = tree.iter_mut().find(|n| n.id == host_id) {
                node.status = new_status.to_string();
                node.ping_ms = ping_ms;
            }
        }

        // 2. 更新内存卡片
        {
            let mut cards = self.master_cards.write().unwrap();
            if let Some(card) = cards.iter_mut().find(|c| c.id == host_id) {
                card.status = new_status.into();
                card.ping_ms = ping_ms;
            }
        }

        // 3. 投递局部更新至界面
        let host_id_owned = host_id.to_string();
        let status_owned = new_status.to_string();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = window_weak.upgrade() {
                let hb = w.global::<HostsBridge>();
                let current_tree = hb.get_tree_nodes();
                let count = current_tree.row_count();
                let mut updated_tree_nodes = Vec::with_capacity(count);

                for i in 0..count {
                    if let Some(mut n) = current_tree.row_data(i) {
                        if n.id == host_id_owned.as_str() {
                            n.status = status_owned.clone().into();
                            n.ping_ms = ping_ms;
                        }
                        updated_tree_nodes.push(n);
                    }
                }

                hb.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(
                    updated_tree_nodes,
                ))));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_mock_store() -> HostStore {
        let tree = vec![
            RawTreeNode {
                id: "grp-dev".into(),
                name: "开发环境".into(),
                is_group: true,
                parent_id: "".into(),
                level: 0,
                address: "".into(),
                port: 0,
                status: "online".into(),
                ping_ms: 0,
                item_count: 1,
                effective_username: None,
            },
            RawTreeNode {
                id: "host-1".into(),
                name: "测试机 1".into(),
                is_group: false,
                parent_id: "grp-dev".into(),
                level: 1,
                address: "192.168.1.100".into(),
                port: 22,
                status: "online".into(),
                ping_ms: 15,
                item_count: 0,
                effective_username: Some("root".into()),
            },
        ];

        let cards = vec![HostItemData {
            id: "host-1".into(),
            name: "测试机 1".into(),
            address: "192.168.1.100".into(),
            port: 22,
            group: "开发环境".into(),
            status: "online".into(),
            ping_ms: 15,
        }];

        HostStore::new(
            tree,
            cards,
            HashSet::from(["grp-dev".to_string()]),
            HashSet::from(["root".to_string()]),
        )
    }

    #[test]
    fn test_host_store_toggle_group() {
        let store = create_mock_store();
        assert!(store.expanded_groups.read().unwrap().contains("grp-dev"));

        let is_expanded = store.toggle_group("grp-dev");
        assert!(!is_expanded);
        assert!(!store.expanded_groups.read().unwrap().contains("grp-dev"));

        let is_expanded = store.toggle_group("grp-dev");
        assert!(is_expanded);
        assert!(store.expanded_groups.read().unwrap().contains("grp-dev"));
    }

    #[test]
    fn test_host_store_toggle_selector_group() {
        let store = create_mock_store();
        assert!(store.selector_expanded_groups.read().unwrap().contains("root"));

        let is_expanded = store.toggle_selector_group("root");
        assert!(!is_expanded);
        assert!(!store.selector_expanded_groups.read().unwrap().contains("root"));
    }

    #[test]
    fn test_host_store_search_versioning() {
        let store = create_mock_store();
        assert_eq!(store.current_version(), 0);

        let v1 = store.set_search_query("test".to_string());
        assert_eq!(v1, 1);
        assert_eq!(store.current_version(), 1);
        assert_eq!(*store.search_query.read().unwrap(), "test");

        let v2 = store.set_search_query("test2".to_string());
        assert_eq!(v2, 2);
        assert_eq!(store.current_version(), 2);
    }

    #[test]
    fn test_host_store_inplace_status_update_memory() {
        let store = create_mock_store();

        // 仅就地修改内存状态
        {
            let mut tree = store.master_tree.write().unwrap();
            let node = tree.iter_mut().find(|n| n.id == "host-1").unwrap();
            node.status = "warning".into();
            node.ping_ms = 220;
        }
        {
            let mut cards = store.master_cards.write().unwrap();
            let card = cards.iter_mut().find(|c| c.id == "host-1").unwrap();
            card.status = "warning".into();
            card.ping_ms = 220;
        }

        let tree = store.master_tree.read().unwrap();
        let host = tree.iter().find(|n| n.id == "host-1").unwrap();
        assert_eq!(host.status, "warning");
        assert_eq!(host.ping_ms, 220);

        let cards = store.master_cards.read().unwrap();
        let card = cards.iter().find(|c| c.id == "host-1").unwrap();
        assert_eq!(card.status, "warning");
        assert_eq!(card.ping_ms, 220);
    }
}
