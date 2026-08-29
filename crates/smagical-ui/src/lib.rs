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
use smagical_debug::{
    calculate_node_width, generate_batch_hosts, get_preset_by_id, BatchGenerateConfig, DebugRawNode,
};
use theme::{apply_theme_by_id, initialize_theme_service};

#[allow(missing_docs, dead_code)]
mod generated {
    slint::include_modules!();
}

pub use generated::{
    AppColorScheme, AppTheme, AppWindow, GroupOptionData, HostItemData, HostTreeNode, LogEntryData,
    TabData,
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

impl From<DebugRawNode> for RawTreeNode {
    fn from(n: DebugRawNode) -> Self {
        Self {
            id: n.id,
            name: n.name,
            is_group: n.is_group,
            parent_id: n.parent_id,
            level: n.level,
            address: n.address,
            port: n.port,
            status: n.status,
            ping_ms: n.ping_ms,
            item_count: n.item_count,
        }
    }
}

/// 解析路径（如 "集群/k8s" 或 "亚太/中国区/杭州"）并在树中逐级确保嵌套分组节点存在
///
/// 返回 (叶子分组 ID, 叶子分组深度层级, 叶子分组展示名称)
fn ensure_raw_group_hierarchy(tree: &mut Vec<RawTreeNode>, path: &str) -> (String, i32, String) {
    let clean_path = path.replace('\\', "/");
    let segments: Vec<&str> = clean_path
        .split('/')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if segments.is_empty() {
        return ("".to_string(), 0, "未分组".to_string());
    }

    let mut current_parent_id = "".to_string();
    let mut current_level = 0;
    let mut last_name = "默认分组".to_string();
    let mut cumulative_slug = String::new();

    for (idx, seg) in segments.iter().enumerate() {
        last_name = seg.to_string();
        if !cumulative_slug.is_empty() {
            cumulative_slug.push('-');
        }
        cumulative_slug.push_str(&seg.to_lowercase().replace(' ', "-"));
        let grp_id = format!("grp-{}", cumulative_slug);

        let existing_idx = tree.iter().position(|n| {
            n.is_group && n.name == *seg && n.parent_id == current_parent_id
        });

        if let Some(pos) = existing_idx {
            current_parent_id = tree[pos].id.clone();
            current_level = tree[pos].level;
        } else {
            tree.push(RawTreeNode {
                id: grp_id.clone(),
                name: seg.to_string(),
                is_group: true,
                parent_id: current_parent_id.clone(),
                level: idx as i32,
                address: "".to_string(),
                port: 0,
                status: "online".to_string(),
                ping_ms: 0,
                item_count: 0,
            });
            current_parent_id = grp_id;
            current_level = idx as i32;
        }
    }

    (current_parent_id, current_level, last_name)
}

/// 获取系统初始化主控树结构数据 (来自 smagical-debug minimal 预设)
fn get_initial_master_tree() -> Vec<RawTreeNode> {
    let (tree, _) = get_preset_by_id("minimal");
    tree.into_iter().map(RawTreeNode::from).collect()
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

/// 全局主控静态主机卡片列表 (测试小数据集时的自隐藏滚动条表现)
const MASTER_HOST_CARDS: &[RawHostCard] = &[
    RawHostCard { id: "1", name: "prod-server-01", address: "192.168.1.100", port: 22, group: "生产集群", status: "online", ping_ms: 21 },
    RawHostCard { id: "2", name: "web-server-02", address: "192.168.1.101", port: 22, group: "生产集群", status: "online", ping_ms: 25 },
    RawHostCard { id: "3", name: "backup-node", address: "192.168.1.200", port: 22, group: "备份节点", status: "offline", ping_ms: 0 },
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

/// 计算可见树形节点列表所需的最大呈现宽度 (像素)
fn calculate_max_tree_width(nodes: &[HostTreeNode]) -> f32 {
    let mut max_w: f32 = 240.0;
    for node in nodes {
        let w = calculate_node_width(node.name.as_str(), node.level);
        if w > max_w {
            max_w = w;
        }
    }
    max_w
}

/// 同步全局 Tracing 实时事件日志到 Slint UI 调试抽屉
fn sync_ui_debug_logs(w: &AppWindow) {
    if let Ok(buf) = smagical_debug::get_global_log_buffer().lock() {
        let entries = buf.get_all();
        let slint_entries: Vec<LogEntryData> = entries
            .into_iter()
            .map(|e| LogEntryData {
                timestamp: e.timestamp.into(),
                level: e.level.into(),
                module: e.module.into(),
                message: e.message.into(),
            })
            .collect();
        w.set_debug_logs(slint::ModelRc::from(Rc::new(slint::VecModel::from(slint_entries))));
    }
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

    tracing::info!(target: "smagical_ui", "Smalux-SSH 桌面应用工作台就绪");
    sync_ui_debug_logs(&window);

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

            tracing::info!(target: "smagical_ui::theme", "切换应用配色主题: {} ({})", name, id_str);
            sync_ui_debug_logs(&w);
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

            tracing::info!(target: "smagical_ui::theme", "{}", if next_dark { "切换至深色模式 (Darcula)" } else { "切换至浅色模式 (GitHub Light)" });
            sync_ui_debug_logs(&w);
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

    // 初始化树形结构折叠状态 (默认展开生产集群)
    let expanded_groups = Rc::new(RefCell::new(HashSet::from([
        "grp-prod".to_string(),
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
    window.set_tree_content_width(calculate_max_tree_width(&initial_nodes));
    window.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(initial_nodes))));

    // 初始渲染卡片列表 (全量 20+ 台主机资产，供卡片模式纵向滚动测试)
    let initial_cards: Vec<HostItemData> = MASTER_HOST_CARDS
        .iter()
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
    window.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(initial_cards))));

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
            let is_expanding = !set.contains(&id_str);
            if set.contains(&id_str) {
                set.remove(&id_str);
            } else {
                set.insert(id_str.clone());
            }
            let tree = master_tree_toggle.borrow();
            let q = search_query_toggle.borrow().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &set)
            } else {
                build_search_tree_nodes(&tree, &q)
            };
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            let gname = tree.iter().find(|n| n.id == id_str).map(|n| n.name.as_str()).unwrap_or(id_str.as_str());
            tracing::debug!(target: "smagical_ui::tree", "{}分组: {}", if is_expanding { "展开" } else { "折叠" }, gname);
            sync_ui_debug_logs(&w);
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
                name: g_name.clone(),
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
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            tracing::info!(target: "smagical_ui::tree", "创建新分组: {} (上级: {})", g_name, if p_id.is_empty() { "根目录" } else { &p_id });
            sync_ui_debug_logs(&w);
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
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
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

            if !q.is_empty() {
                tracing::debug!(target: "smagical_ui::search", "过滤主机资产: '{}'", q);
                sync_ui_debug_logs(&w);
            }
        }
    });

    // 绑定主机打开回调 (从左侧主机列表点击时打开或激活对应的 Tab)
    let window_weak = window.as_weak();
    window.on_open_host(move |id| {
        if let Some(w) = window_weak.upgrade() {
            let tabs = w.get_tabs();
            let id_str = id.to_string();
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
                let title = match id_str.as_str() {
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
            tracing::info!(target: "smagical_ui::session", "打开主机终端会话: {}", id_str);
            sync_ui_debug_logs(&w);
        }
    });

    // 绑定快捷命令发送回调
    let window_weak = window.as_weak();
    window.on_send_snippet(move |cmd| {
        if let Some(w) = window_weak.upgrade() {
            tracing::info!(target: "smagical_ui::cmd", "向终端发送指令片段: {}", cmd);
            sync_ui_debug_logs(&w);
        }
    });

    // =========================================================================
    // 开发者调试控制面板与批量生成事件绑定 (Debug Workbench Handlers)
    // =========================================================================

    // 0.1 批量生成主机资产
    let window_weak = window.as_weak();
    let master_tree_bg = Rc::clone(&master_tree);
    let expanded_bg = Rc::clone(&expanded_groups);
    let selector_bg = Rc::clone(&selector_expanded_groups);
    let search_bg = Rc::clone(&search_query);
    window.on_debug_batch_generate(move |prefix, count_str, ip_prefix, start_ip_str, port_str, group, status_mode, overwrite| {
        if let Some(w) = window_weak.upgrade() {
            let p_str = prefix.to_string();
            let ip_p_str = ip_prefix.to_string();
            let grp_str = group.to_string();
            let st_str = status_mode.to_string();
            let cnt = count_str.as_str().parse::<usize>().unwrap_or(10);
            let start_ip = start_ip_str.as_str().parse::<usize>().unwrap_or(10);
            let port = port_str.as_str().parse::<i32>().unwrap_or(22);

            let config = BatchGenerateConfig {
                name_prefix: if p_str.is_empty() { "node-".to_string() } else { p_str },
                count: if cnt == 0 { 10 } else { cnt },
                start_index: 1,
                ip_prefix: if ip_p_str.is_empty() { "192.168.1.".to_string() } else { ip_p_str },
                start_ip,
                port,
                group_name: if grp_str.is_empty() { "批量集群".to_string() } else { grp_str.clone() },
                status_mode: st_str,
            };

            let (new_tree_raw, new_cards_raw) = generate_batch_hosts(&config);
            let new_tree: Vec<RawTreeNode> = new_tree_raw.into_iter().map(RawTreeNode::from).collect();
            let new_cards: Vec<HostItemData> = new_cards_raw
                .into_iter()
                .map(|c| HostItemData {
                    id: c.id.into(),
                    name: c.name.into(),
                    address: c.address.into(),
                    port: c.port,
                    group: c.group.into(),
                    status: c.status.into(),
                    ping_ms: c.ping_ms,
                })
                .collect();

            if overwrite {
                *master_tree_bg.borrow_mut() = new_tree.clone();
                w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(new_cards))));
            } else {
                let mut current_tree = master_tree_bg.borrow_mut();
                let (leaf_gid, leaf_lvl, _leaf_name) = ensure_raw_group_hierarchy(&mut current_tree, &grp_str);
                
                // 将新生成的 host 节点挂入已存在/新建的叶子分组
                for n in &new_tree {
                    if !n.is_group {
                        let mut host_node = n.clone();
                        host_node.parent_id = leaf_gid.clone();
                        host_node.level = leaf_lvl + 1;
                        current_tree.push(host_node);
                    }
                }

                let hosts = w.get_hosts();
                let mut host_list: Vec<HostItemData> = (0..hosts.row_count()).filter_map(|i| hosts.row_data(i)).collect();
                host_list.extend(new_cards);
                w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(host_list))));
            }

            // 展开新增的分组
            for n in &new_tree {
                if n.is_group {
                    expanded_bg.borrow_mut().insert(n.id.clone());
                    selector_bg.borrow_mut().insert(n.id.clone());
                }
            }

            let tree = master_tree_bg.borrow();
            let opts = build_group_options(&tree, &selector_bg.borrow());
            w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(opts))));

            let q = search_bg.borrow().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &expanded_bg.borrow())
            } else {
                build_search_tree_nodes(&tree, &q)
            };
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            tracing::info!(target: "smagical_debug::batch", "成功批量生成主机: {} 台 (归属: {})", cnt, grp_str);
            sync_ui_debug_logs(&w);
        }
    });

    // 0.2 批量更新主机状态
    let window_weak = window.as_weak();
    let master_tree_bs = Rc::clone(&master_tree);
    let expanded_bs = Rc::clone(&expanded_groups);
    let search_bs = Rc::clone(&search_query);
    window.on_debug_batch_update_status(move |status_mode| {
        if let Some(w) = window_weak.upgrade() {
            let st = status_mode.as_str();
            let mut tree = master_tree_bs.borrow_mut();
            for (i, node) in tree.iter_mut().enumerate() {
                if !node.is_group {
                    let (s, ping) = match st {
                        "all_online" | "online" => ("online", 18),
                        "all_offline" | "offline" => ("offline", 0),
                        "all_warning" | "warning" => ("warning", 160),
                        _ => {
                            if i % 3 == 0 {
                                ("warning", 135)
                            } else if i % 4 == 0 {
                                ("offline", 0)
                            } else {
                                ("online", 20)
                            }
                        }
                    };
                    node.status = s.to_string();
                    node.ping_ms = ping;
                }
            }

            let hosts = w.get_hosts();
            let mut host_list: Vec<HostItemData> = (0..hosts.row_count()).filter_map(|i| hosts.row_data(i)).collect();
            for (i, card) in host_list.iter_mut().enumerate() {
                let (s, ping) = match st {
                    "all_online" | "online" => ("online", 18),
                    "all_offline" | "offline" => ("offline", 0),
                    "all_warning" | "warning" => ("warning", 160),
                    _ => {
                        if i % 3 == 0 {
                            ("warning", 135)
                        } else if i % 4 == 0 {
                            ("offline", 0)
                        } else {
                            ("online", 20)
                        }
                    }
                };
                card.status = s.into();
                card.ping_ms = ping;
            }
            w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(host_list))));

            let q = search_bs.borrow().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &expanded_bs.borrow())
            } else {
                build_search_tree_nodes(&tree, &q)
            };
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            tracing::info!(target: "smagical_debug::batch", "批量变更全量主机状态为: {}", st);
            sync_ui_debug_logs(&w);
        }
    });

    // 0.3 批量更新 SSH 端口
    let window_weak = window.as_weak();
    let master_tree_bp = Rc::clone(&master_tree);
    let expanded_bp = Rc::clone(&expanded_groups);
    let search_bp = Rc::clone(&search_query);
    window.on_debug_batch_update_port(move |new_port_str| {
        if let Some(w) = window_weak.upgrade() {
            let new_port = new_port_str.as_str().parse::<i32>().unwrap_or(22);
            let mut tree = master_tree_bp.borrow_mut();
            for node in tree.iter_mut() {
                if !node.is_group {
                    node.port = new_port;
                }
            }

            let hosts = w.get_hosts();
            let mut host_list: Vec<HostItemData> = (0..hosts.row_count()).filter_map(|i| hosts.row_data(i)).collect();
            for card in host_list.iter_mut() {
                card.port = new_port;
            }
            w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(host_list))));

            let q = search_bp.borrow().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &expanded_bp.borrow())
            } else {
                build_search_tree_nodes(&tree, &q)
            };
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            tracing::info!(target: "smagical_debug::batch", "批量更新全量主机 SSH 端口为: {}", new_port);
            sync_ui_debug_logs(&w);
        }
    });

    // 1. 场景预设一键注入
    let window_weak = window.as_weak();
    let master_tree_dbg = Rc::clone(&master_tree);
    let expanded_dbg = Rc::clone(&expanded_groups);
    let selector_dbg = Rc::clone(&selector_expanded_groups);
    let search_dbg = Rc::clone(&search_query);
    window.on_debug_inject_preset(move |preset_id| {
        if let Some(w) = window_weak.upgrade() {
            let pid = preset_id.as_str();
            let (new_tree_raw, new_cards_raw) = get_preset_by_id(pid);
            let new_tree: Vec<RawTreeNode> = new_tree_raw.into_iter().map(RawTreeNode::from).collect();
            let new_cards: Vec<HostItemData> = new_cards_raw
                .into_iter()
                .map(|c| HostItemData {
                    id: c.id.into(),
                    name: c.name.into(),
                    address: c.address.into(),
                    port: c.port,
                    group: c.group.into(),
                    status: c.status.into(),
                    ping_ms: c.ping_ms,
                })
                .collect();

            *master_tree_dbg.borrow_mut() = new_tree.clone();

            // 重置展开状态（默认展开所有顶级分组）
            let mut new_exp = HashSet::new();
            for n in &new_tree {
                if n.is_group {
                    new_exp.insert(n.id.clone());
                }
            }
            *expanded_dbg.borrow_mut() = new_exp.clone();
            new_exp.insert("root".to_string());
            *selector_dbg.borrow_mut() = new_exp.clone();

            // 刷新弹窗上级分组选项
            let opts = build_group_options(&new_tree, &selector_dbg.borrow());
            w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(opts))));

            // 刷新树形节点与视口计算宽度
            let q = search_dbg.borrow().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&new_tree, &expanded_dbg.borrow())
            } else {
                build_search_tree_nodes(&new_tree, &q)
            };
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            // 刷新卡片列表
            w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(new_cards))));

            tracing::info!(target: "smagical_debug::preset", "成功注入场景预设: {}", pid);
            sync_ui_debug_logs(&w);
        }
    });

    // 2. 快速新增主机 (支持路径嵌套，例如: 集群/k8s)
    let window_weak = window.as_weak();
    let master_tree_qh = Rc::clone(&master_tree);
    let expanded_qh = Rc::clone(&expanded_groups);
    let selector_qh = Rc::clone(&selector_expanded_groups);
    let search_qh = Rc::clone(&search_query);
    let next_hid = Rc::new(RefCell::new(100));
    window.on_debug_quick_add_host(move |name, ip, port_str, group| {
        if let Some(w) = window_weak.upgrade() {
            let h_name = name.trim().to_string();
            let h_ip = ip.trim().to_string();
            let h_grp = group.trim().to_string();
            let port = port_str.as_str().parse::<i32>().unwrap_or(22);
            if h_name.is_empty() { return; }

            let mut counter = next_hid.borrow_mut();
            *counter += 1;
            let new_id = format!("custom-host-{}", *counter);

            let mut tree = master_tree_qh.borrow_mut();

            let (parent_id, level, display_grp) = if !h_grp.is_empty() {
                let (pid, lvl, name) = ensure_raw_group_hierarchy(&mut tree, &h_grp);
                for n in tree.iter() {
                    if n.is_group {
                        expanded_qh.borrow_mut().insert(n.id.clone());
                        selector_qh.borrow_mut().insert(n.id.clone());
                    }
                }
                (pid, lvl + 1, name)
            } else {
                ("".to_string(), 0, "未分组".to_string())
            };

            let node = RawTreeNode {
                id: new_id.clone(),
                name: h_name.clone(),
                is_group: false,
                parent_id,
                level,
                address: h_ip.clone(),
                port,
                status: "online".to_string(),
                ping_ms: 22,
                item_count: 0,
            };

            tree.push(node);

            let opts = build_group_options(&tree, &selector_qh.borrow());
            w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(opts))));

            let q = search_qh.borrow().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &expanded_qh.borrow())
            } else {
                build_search_tree_nodes(&tree, &q)
            };
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            let card = HostItemData {
                id: new_id.into(),
                name: h_name.clone().into(),
                address: h_ip.clone().into(),
                port,
                group: display_grp.into(),
                status: "online".into(),
                ping_ms: 22,
            };
            let hosts = w.get_hosts();
            let mut host_list: Vec<HostItemData> = (0..hosts.row_count()).filter_map(|i| hosts.row_data(i)).collect();
            host_list.push(card);
            w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(host_list))));

            tracing::info!(target: "smagical_debug::data", "快速添加主机: {} ({}:{})", h_name, h_ip, port);
            sync_ui_debug_logs(&w);
        }
    });

    // 3. 快速新增分组 (支持路径嵌套，例如: 集群/k8s)
    let window_weak = window.as_weak();
    let master_tree_qg = Rc::clone(&master_tree);
    let expanded_qg = Rc::clone(&expanded_groups);
    let selector_qg = Rc::clone(&selector_expanded_groups);
    let search_qg = Rc::clone(&search_query);
    window.on_debug_quick_add_group(move |name, _parent| {
        if let Some(w) = window_weak.upgrade() {
            let g_name = name.trim().to_string();
            if g_name.is_empty() { return; }

            let mut tree = master_tree_qg.borrow_mut();
            ensure_raw_group_hierarchy(&mut tree, &g_name);

            // 展开所有分组
            for n in tree.iter() {
                if n.is_group {
                    expanded_qg.borrow_mut().insert(n.id.clone());
                    selector_qg.borrow_mut().insert(n.id.clone());
                }
            }

            let opts = build_group_options(&tree, &selector_qg.borrow());
            w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(opts))));

            let q = search_qg.borrow().clone();
            let next_nodes = if q.is_empty() {
                build_visible_tree_nodes(&tree, &expanded_qg.borrow())
            } else {
                build_search_tree_nodes(&tree, &q)
            };
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            tracing::info!(target: "smagical_debug::data", "快速添加分组层级: {}", g_name);
            sync_ui_debug_logs(&w);
        }
    });

    // 4. 清空全量数据
    let window_weak = window.as_weak();
    let master_tree_clr = Rc::clone(&master_tree);
    window.on_debug_clear_data(move || {
        if let Some(w) = window_weak.upgrade() {
            master_tree_clr.borrow_mut().clear();
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(Vec::new()))));
            w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(Vec::new()))));
            w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(Vec::new()))));
            w.set_tree_content_width(240.0_f32);
            tracing::warn!(target: "smagical_debug::data", "全量主机与分组数据已被清空");
            sync_ui_debug_logs(&w);
        }
    });

    // 5. 恢复默认数据
    let window_weak = window.as_weak();
    let master_tree_rst = Rc::clone(&master_tree);
    let expanded_rst = Rc::clone(&expanded_groups);
    let selector_rst = Rc::clone(&selector_expanded_groups);
    window.on_debug_reset_default_data(move || {
        if let Some(w) = window_weak.upgrade() {
            let (def_tree_raw, def_cards_raw) = get_preset_by_id("minimal");
            let def_tree: Vec<RawTreeNode> = def_tree_raw.into_iter().map(RawTreeNode::from).collect();
            let def_cards: Vec<HostItemData> = def_cards_raw
                .into_iter()
                .map(|c| HostItemData {
                    id: c.id.into(),
                    name: c.name.into(),
                    address: c.address.into(),
                    port: c.port,
                    group: c.group.into(),
                    status: c.status.into(),
                    ping_ms: c.ping_ms,
                })
                .collect();
            *master_tree_rst.borrow_mut() = def_tree.clone();

            *expanded_rst.borrow_mut() = HashSet::from(["grp-prod".to_string()]);
            *selector_rst.borrow_mut() = HashSet::from(["root".to_string(), "grp-prod".to_string()]);

            let opts = build_group_options(&def_tree, &selector_rst.borrow());
            w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(opts))));

            let next_nodes = build_visible_tree_nodes(&def_tree, &expanded_rst.borrow());
            w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
            w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

            w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(def_cards))));

            tracing::info!(target: "smagical_debug::data", "已重置恢复为默认初始数据");
            sync_ui_debug_logs(&w);
        }
    });

    // 6. 清空日志
    let window_weak = window.as_weak();
    window.on_debug_clear_logs(move || {
        if let Some(w) = window_weak.upgrade() {
            if let Ok(mut buf) = smagical_debug::get_global_log_buffer().lock() {
                buf.clear();
            }
            sync_ui_debug_logs(&w);
        }
    });

    // 7. 模拟生成测试日志
    let window_weak = window.as_weak();
    window.on_debug_emit_test_log(move |_level| {
        if let Some(w) = window_weak.upgrade() {
            tracing::info!(target: "smagical_ui::test", "这是测试 INFO 日志消息");
            tracing::warn!(target: "smagical_ui::net", "检测到网络延迟波动: 128ms");
            tracing::error!(target: "smagical_ui::ssh", "连接目标主机 host-prod-01 超时");
            sync_ui_debug_logs(&w);
        }
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
