//! smagicalssh UI crate。
//!
//! 这里依赖 `smagical-core`，负责桌面装配、Slint 界面和主题应用。

#![deny(missing_docs)]

/// Slint 主题资源注册、内置预设和运行时应用接口。
pub mod theme;

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use slint::{ComponentHandle, Model};
use smagical_core::CoreState;
use theme::{apply_theme_by_id, initialize_theme_service};

#[allow(missing_docs, dead_code)]
mod generated {
    slint::include_modules!();
}

pub use generated::{
    AppColorScheme, AppTheme, AppWindow, GroupOptionData, HostItemData, HostTreeNode, TabData,
};

/// 原始树形节点数据结构 (Raw Tree Node)
///
/// 内部核心状态模型，用于完整表达主机管理中所有的分组节点与主机实例节点。
#[derive(Clone, Debug)]
struct RawTreeNode {
    /// 节点的全局唯一 ID (如: "grp-prod"、"host-k8s-w1")
    id: String,
    /// 节点的展示名称 (如: "生产集群 (Production)"、"k8s-control-plane")
    name: String,
    /// 是否为分组节点 (true: 文件夹分组, false: 具体主机资产)
    is_group: bool,
    /// 所属直接父级节点的 ID (顶级根节点为空字符串 "")
    parent_id: String,
    /// 树状层级深度 (0: 顶级根节点, 1: 一级子节点, 2: 二级子节点...)
    level: i32,
    /// 主机 IP 地址或域名 (仅主机节点有效，分组节点为空字符串)
    address: String,
    /// SSH 连接端口 (例如: 22, 6443, 5432)
    port: i32,
    /// 主机在线状态枚举字符串 ("online" 在线, "warning" 告警, "offline" 离线)
    status: String,
    /// ICMP 网络延迟测速结果 (单位: 毫秒，0 表示未测速或离线)
    ping_ms: i32,
    /// 分组下包含的主机/子节点总数量 (仅分组节点有效)
    item_count: i32,
}

/// 获取系统初始化主控树结构数据。
///
/// 预置生产集群、边缘网关、AI算力集群以及容灾备份等多级多层数据。
fn get_initial_master_tree() -> Vec<RawTreeNode> {
    vec![
        // 顶级分组 1: 生产集群
        RawTreeNode { id: "grp-prod".into(), name: "生产集群 (Production)".into(), is_group: true, parent_id: "".into(), level: 0, address: "".into(), port: 0, status: "online".into(), ping_ms: 0, item_count: 5 },
        RawTreeNode { id: "grp-k8s".into(), name: "Kubernetes 集群".into(), is_group: true, parent_id: "grp-prod".into(), level: 1, address: "".into(), port: 0, status: "warning".into(), ping_ms: 0, item_count: 2 },
        RawTreeNode { id: "2".into(), name: "k8s-control-plane".into(), is_group: false, parent_id: "grp-k8s".into(), level: 2, address: "10.0.0.1".into(), port: 6443, status: "warning".into(), ping_ms: 68, item_count: 0 },
        RawTreeNode { id: "host-k8s-w1".into(), name: "k8s-worker-node-01".into(), is_group: false, parent_id: "grp-k8s".into(), level: 2, address: "10.0.0.11".into(), port: 22, status: "online".into(), ping_ms: 24, item_count: 0 },
        RawTreeNode { id: "grp-db".into(), name: "核心数据库集群".into(), is_group: true, parent_id: "grp-prod".into(), level: 1, address: "".into(), port: 0, status: "online".into(), ping_ms: 0, item_count: 2 },
        RawTreeNode { id: "3".into(), name: "db-cluster-primary".into(), is_group: false, parent_id: "grp-db".into(), level: 2, address: "10.0.1.50".into(), port: 5432, status: "online".into(), ping_ms: 18, item_count: 0 },
        RawTreeNode { id: "host-db-s1".into(), name: "db-cluster-standby".into(), is_group: false, parent_id: "grp-db".into(), level: 2, address: "10.0.1.51".into(), port: 5432, status: "online".into(), ping_ms: 20, item_count: 0 },
        RawTreeNode { id: "1".into(), name: "prod-server-01".into(), is_group: false, parent_id: "grp-prod".into(), level: 1, address: "192.168.1.100".into(), port: 22, status: "online".into(), ping_ms: 21, item_count: 0 },

        // 顶级分组 2: 边缘与微服务
        RawTreeNode { id: "grp-edge".into(), name: "边缘网关与缓存".into(), is_group: true, parent_id: "".into(), level: 0, address: "".into(), port: 0, status: "online".into(), ping_ms: 0, item_count: 2 },
        RawTreeNode { id: "5".into(), name: "auth-gateway-edge".into(), is_group: false, parent_id: "grp-edge".into(), level: 1, address: "47.98.12.33".into(), port: 443, status: "online".into(), ping_ms: 35, item_count: 0 },
        RawTreeNode { id: "4".into(), name: "redis-cache-shard-0".into(), is_group: false, parent_id: "grp-edge".into(), level: 1, address: "10.0.2.10".into(), port: 6379, status: "online".into(), ping_ms: 12, item_count: 0 },

        // 顶级分组 3: AI 推理集群
        RawTreeNode { id: "grp-ai".into(), name: "AI 算力集群 (GPU)".into(), is_group: true, parent_id: "".into(), level: 0, address: "".into(), port: 0, status: "online".into(), ping_ms: 0, item_count: 1 },
        RawTreeNode { id: "6".into(), name: "ai-inference-gpu".into(), is_group: false, parent_id: "grp-ai".into(), level: 1, address: "10.0.8.200".into(), port: 22, status: "online".into(), ping_ms: 14, item_count: 0 },

        // 顶级分组 4: 容灾备份与测试
        RawTreeNode { id: "grp-dr".into(), name: "容灾与测试环境".into(), is_group: true, parent_id: "".into(), level: 0, address: "".into(), port: 0, status: "offline".into(), ping_ms: 0, item_count: 2 },
        RawTreeNode { id: "7".into(), name: "backup-node-dr".into(), is_group: false, parent_id: "grp-dr".into(), level: 1, address: "192.168.100.250".into(), port: 22, status: "offline".into(), ping_ms: 0, item_count: 0 },
        RawTreeNode { id: "host-staging".into(), name: "staging-api-test".into(), is_group: false, parent_id: "grp-dr".into(), level: 1, address: "10.0.12.88".into(), port: 22, status: "offline".into(), ping_ms: 0, item_count: 0 },
    ]
}

/// 构建新建分组弹窗中的上级分组树形选项数据模型 (Group Options Data)。
///
/// # 参数
/// * `tree` - 完整的全量节点树
/// * `expanded` - 当前已展开的分组 ID 集合
///
/// # 返回值
/// 返回拍平后的单选选项列表，包含根节点与所有祖先节点处于展开状态的分组项。
fn build_group_options(tree: &[RawTreeNode], expanded: &HashSet<String>) -> Vec<GroupOptionData> {
    let mut options = Vec::new();

    // 1. 根节点配置 (是否有子分组与是否展开)
    let root_has_children = tree.iter().any(|n| n.is_group && n.parent_id.is_empty());
    let root_is_expanded = expanded.contains("root");

    options.push(GroupOptionData {
        id: "root".into(),
        name: "根目录 (作为顶级分组)".into(),
        level: 0,
        parent_id: "".into(),
        has_children: root_has_children,
        is_expanded: root_is_expanded,
    });

    if !root_is_expanded {
        return options;
    }

    for node in tree {
        if !node.is_group {
            continue;
        }

        // 2. 检查祖先链路是否全部处于展开状态
        let mut is_visible = true;
        let mut current_parent = if node.parent_id.is_empty() { "root" } else { node.parent_id.as_str() };
        while !current_parent.is_empty() {
            if !expanded.contains(current_parent) {
                is_visible = false;
                break;
            }
            if current_parent == "root" {
                break;
            }
            if let Some(p) = tree.iter().find(|n| n.id == current_parent) {
                current_parent = if p.parent_id.is_empty() { "root" } else { p.parent_id.as_str() };
            } else {
                break;
            }
        }

        if is_visible {
            let has_children = tree.iter().any(|n| n.is_group && n.parent_id == node.id);
            let is_expanded = has_children && expanded.contains(&node.id);
            options.push(GroupOptionData {
                id: node.id.clone().into(),
                name: node.name.clone().into(),
                level: node.level + 1,
                parent_id: if node.parent_id.is_empty() { "root".into() } else { node.parent_id.clone().into() },
                has_children,
                is_expanded,
            });
        }
    }
    options
}

/// 构建主界面当前可见的树形视图节点 (Visible Tree Nodes)。
///
/// 遵循级联折叠逻辑：当某个父级分组折叠时，其下所有子节点（无论深度）均被隐藏。
///
/// # 参数
/// * `tree` - 完整的全量节点树
/// * `expanded` - 当前已展开的分组 ID 集合
fn build_visible_tree_nodes(tree: &[RawTreeNode], expanded: &HashSet<String>) -> Vec<HostTreeNode> {
    let mut visible = Vec::new();
    for node in tree {
        let mut is_visible = true;
        let mut current_parent = node.parent_id.as_str();
        while !current_parent.is_empty() {
            if !expanded.contains(current_parent) {
                is_visible = false;
                break;
            }
            if let Some(parent_node) = tree.iter().find(|n| n.id == current_parent) {
                current_parent = parent_node.parent_id.as_str();
            } else {
                break;
            }
        }

        if is_visible {
            let is_expanded = node.is_group && expanded.contains(&node.id);
            visible.push(HostTreeNode {
                id: node.id.clone().into(),
                name: node.name.clone().into(),
                is_group: node.is_group,
                parent_id: node.parent_id.clone().into(),
                level: node.level,
                is_expanded,
                address: node.address.clone().into(),
                port: node.port,
                status: node.status.clone().into(),
                ping_ms: node.ping_ms,
                item_count: node.item_count,
            });
        }
    }
    visible
}

/// 静态主机卡片视图数据定义 (Raw Host Card)
struct RawHostCard {
    /// 主机 ID
    id: &'static str,
    /// 主机名称
    name: &'static str,
    /// IP 地址或主机名
    address: &'static str,
    /// 连接端口
    port: i32,
    /// 所属分组标签
    group: &'static str,
    /// 在线状态
    status: &'static str,
    /// 延迟毫秒数
    ping_ms: i32,
}

/// 全局主控静态主机卡片列表
const MASTER_HOST_CARDS: &[RawHostCard] = &[
    RawHostCard { id: "1", name: "prod-server-01", address: "192.168.1.100", port: 22, group: "生产集群", status: "online", ping_ms: 21 },
    RawHostCard { id: "2", name: "k8s-control-plane", address: "10.0.0.1", port: 6443, group: "K8s集群", status: "warning", ping_ms: 68 },
    RawHostCard { id: "3", name: "db-cluster-primary", address: "10.0.1.50", port: 5432, group: "数据库", status: "online", ping_ms: 18 },
    RawHostCard { id: "4", name: "redis-cache-shard-0", address: "10.0.2.10", port: 6379, group: "缓存集群", status: "online", ping_ms: 12 },
    RawHostCard { id: "5", name: "auth-gateway-edge", address: "47.98.12.33", port: 443, group: "边缘网关", status: "online", ping_ms: 35 },
    RawHostCard { id: "6", name: "ai-inference-gpu", address: "10.0.8.200", port: 22, group: "AI算力", status: "online", ping_ms: 14 },
    RawHostCard { id: "7", name: "backup-node-dr", address: "192.168.100.250", port: 22, group: "容灾备份", status: "offline", ping_ms: 0 },
];

/// 根据搜索关键字构建匹配的树形结构节点 (Search Tree Nodes)。
///
/// 核心特性：
/// 1. 匹配目标节点及其所有祖先节点，保持树形层级链路完整；
/// 2. 搜索状态下，所有涉及的中间分组自动强制展开 (`is_expanded: true`)；
/// 3. 若匹配到分组名称，则自动展示其直属所有子节点。
fn build_search_tree_nodes(tree: &[RawTreeNode], query: &str) -> Vec<HostTreeNode> {
    let q = query.to_lowercase();
    let mut matching_or_needed_ids = HashSet::new();

    // 1. 找出所有匹配的主机或匹配的分组
    for node in tree {
        let is_match = node.name.to_lowercase().contains(&q)
            || node.address.to_lowercase().contains(&q);
        if is_match {
            matching_or_needed_ids.insert(node.id.clone());
            // 如果是分组匹配，它的所有直接子项也展现
            if node.is_group {
                for child in tree {
                    if child.parent_id == node.id {
                        matching_or_needed_ids.insert(child.id.clone());
                    }
                }
            }
            // 将所有祖先 ID 加入集合以确保树形链路完整
            let mut cur_parent = node.parent_id.as_str();
            while !cur_parent.is_empty() {
                matching_or_needed_ids.insert(cur_parent.to_string());
                if let Some(p) = tree.iter().find(|n| n.id == cur_parent) {
                    cur_parent = p.parent_id.as_str();
                } else {
                    break;
                }
            }
        }
    }

    // 2. 按自然顺序生成可见节点，搜索模式下所有中间分组均默认展开
    let mut result = Vec::new();
    for node in tree {
        if matching_or_needed_ids.contains(&node.id) {
            result.push(HostTreeNode {
                id: node.id.clone().into(),
                name: node.name.clone().into(),
                is_group: node.is_group,
                parent_id: node.parent_id.clone().into(),
                level: node.level,
                is_expanded: true,
                address: node.address.clone().into(),
                port: node.port,
                status: node.status.clone().into(),
                ping_ms: node.ping_ms,
                item_count: node.item_count,
            });
        }
    }
    result
}

/// 创建并运行桌面应用主窗口。
pub fn run() -> anyhow::Result<()> {
    let mut core = CoreState::new();
    core.seed_example_host();

    let window = AppWindow::new()?;
    let themes = Rc::new(initialize_theme_service(None)?);

    // 默认应用 Darcula 主题
    apply_theme_by_id(&window, &themes, "builtin.ui.darcula")?;
    window.set_current_theme_name("Darcula".into());

    // 绑定主题切换回调
    let window_weak = window.as_weak();
    let themes_clone = Rc::clone(&themes);
    window.on_switch_theme(move |theme_id| {
        if let Some(w) = window_weak.upgrade() {
            let id_str = theme_id.as_str();
            let _ = apply_theme_by_id(&w, &themes_clone, id_str);
            let name = if id_str.contains("darcula") {
                "Darcula"
            } else if id_str.contains("monokai") {
                "Monokai"
            } else if id_str.contains("onedark") || id_str.contains("one-dark") {
                "One Dark"
            } else if id_str.contains("solarized-light") {
                "Solarized Light"
            } else if id_str.contains("solarized") {
                "Solarized"
            } else if id_str.contains("github-light") || id_str.contains("light") {
                "GitHub Light"
            } else if id_str.contains("github-dark") {
                "GitHub Dark"
            } else {
                "System"
            };
            w.set_current_theme_name(name.into());

            // 自动同步深浅色状态
            let is_light = id_str.contains("light") || id_str.contains("dawn") || id_str.contains("latte");
            w.set_is_dark_mode(!is_light);
        }
    });

    // 绑定深色 / 浅色模式一键切换回调
    let window_weak = window.as_weak();
    let themes_clone = Rc::clone(&themes);
    window.on_toggle_color_mode(move || {
        if let Some(w) = window_weak.upgrade() {
            let is_dark = w.get_is_dark_mode();
            let next_dark = !is_dark;
            w.set_is_dark_mode(next_dark);

            if next_dark {
                let _ = apply_theme_by_id(&w, &themes_clone, "builtin.ui.darcula");
                w.set_current_theme_name("Darcula".into());
            } else {
                let _ = apply_theme_by_id(&w, &themes_clone, "builtin.ui.github-light");
                w.set_current_theme_name("GitHub Light".into());
            }
        }
    });

    // 绑定关闭 Tab 回调 (实时从列表中移除该会话，并智能切换至邻近 Tab)
    let window_weak = window.as_weak();
    window.on_close_tab(move |id| {
        if let Some(w) = window_weak.upgrade() {
            let tabs = w.get_tabs();
            let mut new_tabs: Vec<TabData> = Vec::new();
            let id_str = id.as_str();
            let active_id = w.get_active_session_tab();
            let mut next_active = active_id.clone();
            let count = tabs.row_count();

            for idx in 0..count {
                if let Some(tab) = tabs.row_data(idx) {
                    if tab.id != id_str {
                        new_tabs.push(tab);
                    } else if tab.id == active_id {
                        // 如果关闭的是当前激活的 Tab，则智能切到前一个或后一个
                        if idx > 0 {
                            if let Some(prev) = tabs.row_data(idx - 1) {
                                next_active = prev.id;
                            }
                        } else if idx + 1 < count {
                            if let Some(next) = tabs.row_data(idx + 1) {
                                next_active = next.id;
                            }
                        }
                    }
                }
            }

            if new_tabs.is_empty() {
                w.set_has_active_session(false);
            } else {
                w.set_active_session_tab(next_active);
                let model = std::rc::Rc::new(slint::VecModel::from(new_tabs));
                w.set_tabs(slint::ModelRc::from(model));
            }
        }
    });

    // 初始化主控树形结构与分组生成器
    let master_tree = Rc::new(RefCell::new(get_initial_master_tree()));
    let next_group_id = Rc::new(RefCell::new(100));

    // 初始化树形结构折叠状态 (默认展开生产集群、K8s、数据库、边缘与AI)
    let expanded_groups = Rc::new(RefCell::new(HashSet::from([
        "grp-prod".to_string(),
        "grp-k8s".to_string(),
        "grp-db".to_string(),
        "grp-edge".to_string(),
        "grp-ai".to_string(),
    ])));

    let search_query = Rc::new(RefCell::new(String::new()));

    // 初始化上级分组选择器折叠状态 (默认展开根目录与生产集群)
    let selector_expanded_groups = Rc::new(RefCell::new(HashSet::from([
        "root".to_string(),
        "grp-prod".to_string(),
    ])));

    // 初始渲染上级分组选项数据
    let initial_options = build_group_options(&master_tree.borrow(), &selector_expanded_groups.borrow());
    window.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(initial_options))));

    // 初始渲染树形节点
    let initial_nodes = build_visible_tree_nodes(&master_tree.borrow(), &expanded_groups.borrow());
    window.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(initial_nodes))));

    // 绑定上级分组选择器折叠 / 展开回调 (支持弹窗内自由收缩/展开子节点)
    let window_weak = window.as_weak();
    let master_tree_toggle_opt = Rc::clone(&master_tree);
    let selector_expanded_clone = Rc::clone(&selector_expanded_groups);
    window.on_toggle_group_option(move |id| {
        if let Some(w) = window_weak.upgrade() {
            let mut set = selector_expanded_clone.borrow_mut();
            let id_str = id.to_string();
            if set.contains(&id_str) {
                set.remove(&id_str);
            } else {
                set.insert(id_str);
            }
            let tree = master_tree_toggle_opt.borrow();
            let next_options = build_group_options(&tree, &set);
            w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_options))));
        }
    });

    // 绑定树形结构分组折叠 / 展开回调
    let window_weak = window.as_weak();
    let master_tree_toggle = Rc::clone(&master_tree);
    let expanded_clone = Rc::clone(&expanded_groups);
    let search_query_toggle = Rc::clone(&search_query);
    window.on_toggle_tree_group(move |id| {
        if let Some(w) = window_weak.upgrade() {
            let mut set = expanded_clone.borrow_mut();
            let id_str = id.to_string();
            if set.contains(&id_str) {
                set.remove(&id_str);
            } else {
                set.insert(id_str);
            }
            let tree = master_tree_toggle.borrow();
            let q = search_query_toggle.borrow().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &set)
            } else {
                build_search_tree_nodes(&tree, &q)
            };
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));
        }
    });

    // 绑定新建分组回调 (支持树状层级指定上级与即时展开)
    let window_weak = window.as_weak();
    let master_tree_create = Rc::clone(&master_tree);
    let expanded_create = Rc::clone(&expanded_groups);
    let selector_expanded_create = Rc::clone(&selector_expanded_groups);
    let search_query_create = Rc::clone(&search_query);
    let next_gid_create = Rc::clone(&next_group_id);
    window.on_create_group(move |parent_id, name| {
        if let Some(w) = window_weak.upgrade() {
            let p_id = parent_id.to_string();
            let g_name = name.trim().to_string();
            if g_name.is_empty() {
                return;
            }

            let mut tree = master_tree_create.borrow_mut();
            let mut gid_counter = next_gid_create.borrow_mut();
            *gid_counter += 1;
            let new_id = format!("grp-custom-{}", *gid_counter);

            let (target_parent_id, level) = if p_id == "root" || p_id.is_empty() {
                ("".to_string(), 0)
            } else {
                let parent_level = tree.iter().find(|n| n.id == p_id).map(|n| n.level).unwrap_or(0);
                (p_id.clone(), parent_level + 1)
            };

            let new_group_node = RawTreeNode {
                id: new_id.clone(),
                name: g_name,
                is_group: true,
                parent_id: target_parent_id.clone(),
                level,
                address: "".to_string(),
                port: 0,
                status: "online".to_string(),
                ping_ms: 0,
                item_count: 0,
            };

            // 智能定位插入位置：插入到同父节点的子项末尾，或追加到分组后
            let mut insert_pos = tree.len();
            if !target_parent_id.is_empty() {
                let mut last_child_idx = None;
                for (idx, node) in tree.iter().enumerate() {
                    if node.id == target_parent_id || node.parent_id == target_parent_id {
                        last_child_idx = Some(idx);
                    }
                }
                if let Some(idx) = last_child_idx {
                    insert_pos = idx + 1;
                }
                // 确保父节点处于展开状态，以便立刻看见新建的分组
                expanded_create.borrow_mut().insert(target_parent_id.clone());
                selector_expanded_create.borrow_mut().insert(target_parent_id);
            }
            // 新创建的分组自身默认展开
            expanded_create.borrow_mut().insert(new_id);

            tree.insert(insert_pos, new_group_node);

            // 刷新弹窗中的上级分组列表选项
            let next_options = build_group_options(&tree, &selector_expanded_create.borrow());
            w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_options))));

            // 刷新主界面树形结构
            let q = search_query_create.borrow().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &expanded_create.borrow())
            } else {
                build_search_tree_nodes(&tree, &q)
            };
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));
        }
    });

    // 绑定主机实时搜索过滤回调 (双向联动树形视图与卡片列表)
    let window_weak = window.as_weak();
    let master_tree_filter = Rc::clone(&master_tree);
    let expanded_clone = Rc::clone(&expanded_groups);
    let search_query_filter = Rc::clone(&search_query);
    window.on_filter_hosts(move |query| {
        if let Some(w) = window_weak.upgrade() {
            let q = query.trim().to_lowercase();
            *search_query_filter.borrow_mut() = q.clone();

            // 1. 动态过滤树形节点
            let tree = master_tree_filter.borrow();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &expanded_clone.borrow())
            } else {
                build_search_tree_nodes(&tree, &q)
            };
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            // 2. 动态过滤卡片列表
            let filtered_cards: Vec<HostItemData> = MASTER_HOST_CARDS
                .iter()
                .filter(|h| {
                    if q.is_empty() {
                        true
                    } else {
                        h.name.to_lowercase().contains(&q)
                            || h.address.to_lowercase().contains(&q)
                            || h.group.to_lowercase().contains(&q)
                    }
                })
                .map(|h| HostItemData {
                    id: h.id.into(),
                    name: h.name.into(),
                    address: h.address.into(),
                    port: h.port,
                    group: h.group.into(),
                    status: h.status.into(),
                    ping_ms: h.ping_ms,
                })
                .collect();
            w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(filtered_cards))));
        }
    });

    // 绑定主机打开回调 (从左侧主机列表点击时打开或激活对应的 Tab)
    let window_weak = window.as_weak();
    window.on_open_host(move |id| {
        if let Some(w) = window_weak.upgrade() {
            let tabs = w.get_tabs();
            let id_str = id.as_str();
            let mut found = false;
            for idx in 0..tabs.row_count() {
                if let Some(tab) = tabs.row_data(idx) {
                    if tab.id == id_str {
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                let mut list: Vec<TabData> = (0..tabs.row_count()).filter_map(|i| tabs.row_data(i)).collect();
                let title = match id_str {
                    "host-prod-01" => "prod-server-01",
                    "host-k8s-master" => "k8s-control-plane",
                    "host-db-pg" => "db-cluster-primary",
                    "host-redis" => "redis-cache-shard-0",
                    _ => "new-terminal-session",
                };
                list.push(TabData {
                    id: id.clone(),
                    title: title.into(),
                    status: "online".into(),
                });
                w.set_has_active_session(true);
                let model = std::rc::Rc::new(slint::VecModel::from(list));
                w.set_tabs(slint::ModelRc::from(model));
            }
            w.set_active_session_tab(id);
        }
    });

    // 绑定快捷命令发送回调
    window.on_send_snippet(|cmd| {
        println!("向当前终端发送指令: {}", cmd);
    });

    // 绑定无边框窗口控制回调
    window.on_close_window(|| {
        let _ = slint::quit_event_loop();
    });

    let window_weak = window.as_weak();
    window.on_minimize_window(move || {
        if let Some(w) = window_weak.upgrade() {
            w.window().set_minimized(true);
        }
    });

    let window_weak = window.as_weak();
    window.on_maximize_window(move || {
        if let Some(w) = window_weak.upgrade() {
            let is_max = w.window().is_maximized();
            w.window().set_maximized(!is_max);
            w.set_is_window_maximized(!is_max);
        }
    });

    window.run()?;
    Ok(())
}
