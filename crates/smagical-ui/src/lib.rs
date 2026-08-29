//! smagicalssh UI crate。
//!
//! 这里依赖 `smagical-core`，负责桌面装配、Slint 界面和主题应用。

#![deny(missing_docs)]

/// 本地终端环境探测模块。
pub mod local_shells;

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
    AppColorScheme, AppTheme, AppWindow, GroupOptionData, HostItemData, HostTreeNode,
    LocalShellItemData, LogEntryData, TabData,
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

/// 移动与调序树形节点（主机或分组）
///
/// 支持四种落点模式：
/// - "inside": 移入目标分组内部作为其子节点
/// - "before": 插在目标节点上方（成为目标节点的同级前序节点）
/// - "after": 插在目标节点下方（成为目标节点的同级后序节点）
/// - "root": 移至顶级根目录
fn move_and_reorder_raw_node(
    tree: &mut Vec<RawTreeNode>,
    source_id: &str,
    target_id: &str,
    drop_position: &str,
) -> Result<(String /* source_name */, String /* target_name */), String> {
    let source_idx = tree
        .iter()
        .position(|n| n.id == source_id)
        .ok_or_else(|| "未找到源节点".to_string())?;

    let is_source_group = tree[source_idx].is_group;
    let source_name = tree[source_idx].name.clone();
    let old_level = tree[source_idx].level;

    // 自身拖拽且无需调序保护
    if source_id == target_id {
        if drop_position == "inside" {
            return Err("不能将节点移入自身内部".to_string());
        }
        return Ok((source_name.clone(), source_name));
    }

    // 收集源节点的所有后裔节点 ID（如果源节点是分组）
    let mut source_descendant_ids = HashSet::new();
    if is_source_group {
        let mut queue = vec![source_id.to_string()];
        while let Some(parent) = queue.pop() {
            for n in tree.iter() {
                if n.parent_id == parent {
                    source_descendant_ids.insert(n.id.clone());
                    if n.is_group {
                        queue.push(n.id.clone());
                    }
                }
            }
        }
    }

    // 循环引用检测：严禁将父节点移动/插入到其任意子孙节点中
    if source_descendant_ids.contains(target_id) {
        return Err("不能将父分组移动至其子孙节点中 (循环引用)".to_string());
    }

    // 计算目标 Parent ID 和目标层级 Level
    let (new_parent_id, new_level, target_name) = if drop_position == "root" || target_id == "root" || target_id.is_empty() {
        ("".to_string(), 0, "顶级根目录".to_string())
    } else {
        let target_node = tree
            .iter()
            .find(|n| n.id == target_id)
            .ok_or_else(|| "未找到目标节点".to_string())?;

        if drop_position == "inside" {
            if !target_node.is_group {
                return Err("只能移入文件夹分组内部".to_string());
            }
            (target_node.id.clone(), target_node.level + 1, target_node.name.clone())
        } else {
            // "before" 或 "after": 与目标节点同级
            (target_node.parent_id.clone(), target_node.level, target_node.name.clone())
        }
    };

    let level_delta = new_level - old_level;

    // 提取源子树所有节点 (源节点及其所有后代)
    let mut is_subtree_set = HashSet::new();
    is_subtree_set.insert(source_id.to_string());
    for id in &source_descendant_ids {
        is_subtree_set.insert(id.clone());
    }

    let mut subtree_nodes = Vec::new();
    let mut remaining_tree = Vec::new();

    for mut node in tree.drain(..) {
        if is_subtree_set.contains(&node.id) {
            if node.id == source_id {
                node.parent_id = new_parent_id.clone();
                node.level = new_level;
            } else {
                node.level += level_delta;
            }
            subtree_nodes.push(node);
        } else {
            remaining_tree.push(node);
        }
    }

    // 根据落点位置重插入到 remaining_tree
    if drop_position == "before" {
        let target_pos = remaining_tree
            .iter()
            .position(|n| n.id == target_id)
            .unwrap_or(0);
        for (i, node) in subtree_nodes.into_iter().enumerate() {
            remaining_tree.insert(target_pos + i, node);
        }
    } else if drop_position == "after" {
        let target_pos = remaining_tree
            .iter()
            .position(|n| n.id == target_id)
            .unwrap_or_else(|| remaining_tree.len().saturating_sub(1));

        let mut insert_pos = target_pos + 1;
        // 如果目标也是分组，则插入到该目标分组的整个子树后面
        if remaining_tree[target_pos].is_group {
            let mut target_descendants = HashSet::new();
            let mut q = vec![target_id.to_string()];
            while let Some(p) = q.pop() {
                for n in &remaining_tree {
                    if n.parent_id == p {
                        target_descendants.insert(n.id.clone());
                        if n.is_group {
                            q.push(n.id.clone());
                        }
                    }
                }
            }
            while insert_pos < remaining_tree.len() && target_descendants.contains(&remaining_tree[insert_pos].id) {
                insert_pos += 1;
            }
        }

        for (i, node) in subtree_nodes.into_iter().enumerate() {
            remaining_tree.insert(insert_pos + i, node);
        }
    } else if drop_position == "inside" {
        let target_pos = remaining_tree
            .iter()
            .position(|n| n.id == target_id)
            .unwrap_or(0);

        let mut insert_pos = target_pos + 1;
        let mut target_descendants = HashSet::new();
        let mut q = vec![target_id.to_string()];
        while let Some(p) = q.pop() {
            for n in &remaining_tree {
                if n.parent_id == p {
                    target_descendants.insert(n.id.clone());
                    if n.is_group {
                        q.push(n.id.clone());
                    }
                }
            }
        }
        while insert_pos < remaining_tree.len() && target_descendants.contains(&remaining_tree[insert_pos].id) {
            insert_pos += 1;
        }

        for (i, node) in subtree_nodes.into_iter().enumerate() {
            remaining_tree.insert(insert_pos + i, node);
        }
    } else {
        // "root"
        remaining_tree.extend(subtree_nodes);
    }

    // 重新统计各分组的 item_count
    for i in 0..remaining_tree.len() {
        if remaining_tree[i].is_group {
            let grp_id = remaining_tree[i].id.clone();
            let count = remaining_tree.iter().filter(|n| n.parent_id == grp_id).count() as i32;
            remaining_tree[i].item_count = count;
        }
    }

    *tree = remaining_tree;

    Ok((source_name, target_name))
}

/// 对树形结构节点进行标准化排序 (Canonical Hierarchy Sort: 文件夹始终置顶在上方，主机在下方)
///
/// 排序规则：
/// 1. 同级节点中，文件夹/分组（is_group == true）始终排在最前面，主机排在后面；
/// 2. 分组之间保持名称自然排序；主机之间按名称自然排序；
/// 3. 分组的直接子节点严格紧随父分组之后（深度优先遍历 DFS），保持层级结构清晰。
fn sort_tree_hierarchy(tree: &[RawTreeNode]) -> Vec<RawTreeNode> {
    let mut result = Vec::with_capacity(tree.len());

    fn collect_children(parent_id: &str, tree: &[RawTreeNode], result: &mut Vec<RawTreeNode>) {
        let mut children: Vec<&RawTreeNode> = tree.iter().filter(|n| n.parent_id == parent_id).collect();
        // 关键逻辑：is_group == true 优先排在前面；若同为分组或同为主机，则按名称自然排序
        children.sort_by(|a, b| {
            b.is_group
                .cmp(&a.is_group)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });

        for child in children {
            result.push(child.clone());
            if child.is_group {
                collect_children(&child.id, tree, result);
            }
        }
    }

    collect_children("", tree, &mut result);

    // 容错：如果有孤立节点（其 parent_id 不在树中），追加至末尾
    for node in tree {
        if !result.iter().any(|r| r.id == node.id) {
            result.push(node.clone());
        }
    }

    result
}

/// 获取系统初始化主控树结构数据 (来自 smagical-debug minimal 预设，默认文件夹在上方)
fn get_initial_master_tree() -> Vec<RawTreeNode> {
    let (tree, _) = get_preset_by_id("minimal");
    let raw_tree: Vec<RawTreeNode> = tree.into_iter().map(RawTreeNode::from).collect();
    sort_tree_hierarchy(&raw_tree)
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
/// 保持节点数组中的真实顺序（支持用户自由拖拽调序）。
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

/// 活跃终端会话运行时信息
#[derive(Clone, Debug)]
#[allow(dead_code)]
struct TerminalSessionInfo {
    session_id: String,
    host_id: String,
    host_name: String,
    host_address: String,
    host_status: String,
    ping_ms: i32,
    display_title: String,
}

/// 同步活跃终端会话列表与视口状态到 Slint UI
fn sync_active_session_ui(
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

/// 创建并运行桌面应用主窗口。
pub fn run() -> anyhow::Result<()> {
    let mut core = CoreState::new();
    core.seed_example_host();

    let window = AppWindow::new()?;
    let themes = Rc::new(initialize_theme_service(None)?);

    // 默认应用 Darcula 主题
    apply_theme_by_id(&window, &themes, "builtin.ui.darcula")?;
    window.set_current_theme_name("Darcula".into());

    // 活跃会话管理状态 (初始清空全部 Tab)
    let active_sessions: Rc<RefCell<Vec<TerminalSessionInfo>>> = Rc::new(RefCell::new(Vec::new()));
    let next_session_num: Rc<RefCell<usize>> = Rc::new(RefCell::new(1));
    sync_active_session_ui(&window, &active_sessions.borrow(), "");

    // 初始化探测当前操作系统可用本地 Shell 列表
    let initial_shells = local_shells::detect_local_shells();
    window.set_launcher_local_items(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(initial_shells))));

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
    let active_sessions_close = Rc::clone(&active_sessions);
    window.on_close_tab(move |sess_id| {
        if let Some(w) = window_weak.upgrade() {
            let id_str = sess_id.to_string();
            let mut sessions = active_sessions_close.borrow_mut();
            let cur_active = w.get_active_session_tab().to_string();

            let mut next_active = cur_active.clone();
            if let Some(idx) = sessions.iter().position(|s| s.session_id == id_str) {
                if cur_active == id_str {
                    if idx > 0 {
                        next_active = sessions[idx - 1].session_id.clone();
                    } else if idx + 1 < sessions.len() {
                        next_active = sessions[idx + 1].session_id.clone();
                    } else {
                        next_active = "".to_string();
                    }
                }
                sessions.remove(idx);
            }

            sync_active_session_ui(&w, &sessions, &next_active);
            tracing::info!(target: "smagical_ui::session", "已关闭终端会话: {}", id_str);
            sync_ui_debug_logs(&w);
        }
    });

    // 绑定切换 Tab 回调 (点击 Tab 时激活对应的会话)
    let window_weak = window.as_weak();
    let active_sessions_select = Rc::clone(&active_sessions);
    window.on_select_tab(move |sess_id| {
        if let Some(w) = window_weak.upgrade() {
            let id_str = sess_id.to_string();
            let sessions = active_sessions_select.borrow();
            sync_active_session_ui(&w, &sessions, &id_str);
            tracing::debug!(target: "smagical_ui::session", "切换至终端会话: {}", id_str);
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
    let master_cards = Rc::new(RefCell::new(initial_cards.clone()));
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

    // 绑定拖拽调序与移动节点回调 (支持树形层级迁移与列表纯展示调序双独立机制)
    let window_weak = window.as_weak();
    let master_tree_move = Rc::clone(&master_tree);
    let master_cards_move = Rc::clone(&master_cards);
    let expanded_move = Rc::clone(&expanded_groups);
    let selector_expanded_move = Rc::clone(&selector_expanded_groups);
    let search_query_move = Rc::clone(&search_query);
    window.on_move_tree_node(move |src_id, target_id, drop_position| {
        if let Some(w) = window_weak.upgrade() {
            let src_str = src_id.to_string();
            let target_str = target_id.to_string();
            let pos_str = drop_position.to_string();
            let view_mode = w.get_hosts_view_mode().to_string();

            // 1. 卡片平铺列表模式 (Card View Mode): 纯视觉显示排序调整，绝对锁定所属分组 (parent_id/group) 不变
            if view_mode == "card" {
                let mut cards = master_cards_move.borrow_mut();
                if let (Some(src_idx), Some(tgt_idx)) = (
                    cards.iter().position(|c| c.id == src_str.as_str()),
                    cards.iter().position(|c| c.id == target_str.as_str()),
                ) {
                    if src_idx != tgt_idx {
                        let item = cards.remove(src_idx);
                        let target_insert_idx = if src_idx < tgt_idx {
                            tgt_idx // 移出后前面少了一个元素，原 tgt 后面位置变为 tgt_idx
                        } else {
                            tgt_idx + 1
                        };
                        let final_pos = target_insert_idx.min(cards.len());
                        let item_name = item.name.to_string();
                        let tgt_name = cards.get(tgt_idx.min(cards.len().saturating_sub(1))).map(|c| c.name.to_string()).unwrap_or_default();
                        cards.insert(final_pos, item);

                        let q = search_query_move.borrow().clone();
                        let display_cards: Vec<HostItemData> = if q.is_empty() {
                            cards.clone()
                        } else {
                            cards.iter().filter(|h| {
                                h.name.to_lowercase().contains(&q)
                                    || h.address.to_lowercase().contains(&q)
                                    || h.group.to_lowercase().contains(&q)
                            }).cloned().collect()
                        };
                        w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(display_cards))));

                        tracing::info!(target: "smagical_ui::hosts", "成功调整列表模式主机展示顺序: [{}] 排在 [{}] 之后 (分组保持锁定)", item_name, tgt_name);
                        sync_ui_debug_logs(&w);
                    }
                }
                return;
            }

            // 2. 树形层级模式 (Tree View Mode): 物理资产层级结构与文件夹迁移
            let mut tree = master_tree_move.borrow_mut();

            match move_and_reorder_raw_node(&mut tree, &src_str, &target_str, &pos_str) {
                Ok((src_name, target_name)) => {
                    // 如果移动到了具体分组内部，自动将该目标分组及其祖先加入展开集合
                    let mut exp = expanded_move.borrow_mut();
                    if pos_str == "inside" && !target_str.is_empty() {
                        let mut curr = target_str.clone();
                        while !curr.is_empty() {
                            exp.insert(curr.clone());
                            if let Some(p) = tree.iter().find(|n| n.id == curr) {
                                curr = p.parent_id.clone();
                            } else {
                                break;
                            }
                        }
                    }

                    // 刷新树形视图与选择器选项
                    let q = search_query_move.borrow().clone();
                    let next_nodes = if q.is_empty() {
                        build_visible_tree_nodes(&tree, &exp)
                    } else {
                        build_search_tree_nodes(&tree, &q)
                    };
                    w.set_tree_content_width(calculate_max_tree_width(&next_nodes));
                    w.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_nodes))));

                    let next_options = build_group_options(&tree, &selector_expanded_move.borrow());
                    w.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(next_options))));

                    // 树形模式下移动了主机：同步更新列表模式中的所属分组徽章，同时保留用户在列表模式下的自定义相对排序
                    let new_group_name = if let Some(n) = tree.iter().find(|item| item.id == src_str) {
                        if !n.parent_id.is_empty() {
                            tree.iter().find(|item| item.id == n.parent_id).map(|item| item.name.clone()).unwrap_or_else(|| "未分组".to_string())
                        } else {
                            "未分组".to_string()
                        }
                    } else {
                        "未分组".to_string()
                    };

                    let mut cards = master_cards_move.borrow_mut();
                    for card in cards.iter_mut() {
                        if card.id == src_str.as_str() {
                            card.group = new_group_name.clone().into();
                        }
                    }

                    let display_cards: Vec<HostItemData> = if q.is_empty() {
                        cards.clone()
                    } else {
                        cards.iter().filter(|h| {
                            h.name.to_lowercase().contains(&q)
                                || h.address.to_lowercase().contains(&q)
                                || h.group.to_lowercase().contains(&q)
                        }).cloned().collect()
                    };
                    w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(display_cards))));

                    tracing::info!(target: "smagical_ui::hosts", "成功调序/移动树节点 [{}] (模式: {}, 目标: [{}])", src_name, pos_str, target_name);
                    sync_ui_debug_logs(&w);
                }
                Err(err_msg) => {
                    tracing::warn!(target: "smagical_ui::hosts", "调序/移动树节点失败: {}", err_msg);
                    sync_ui_debug_logs(&w);
                }
            }
        }
    });

    // 绑定实时拖拽悬停落点计算回调 (支持树形层级与卡片平铺双模式)
    let window_weak = window.as_weak();
    let master_tree_hover = Rc::clone(&master_tree);
    window.on_request_drag_hover(move |src_id, target_idx, _offset_in_row| {
        if let Some(w) = window_weak.upgrade() {
            let src_str = src_id.to_string();
            let view_mode = w.get_hosts_view_mode().to_string();

            if target_idx < 0 {
                w.set_drop_target_index(-1);
                w.set_drop_target_id("root".into());
                w.set_drop_target_name("顶级根目录 (未分组)".into());
                w.set_drop_position("root".into());
                w.set_drop_target_valid(true);
                return;
            }

            // 1. 卡片平铺列表模式 (Card View Mode)
            if view_mode == "card" {
                let current_hosts = w.get_hosts();
                let total_len = current_hosts.row_count();
                if (target_idx as usize) < total_len {
                    if let Some(target) = current_hosts.row_data(target_idx as usize) {
                        let target_id = target.id.to_string();
                        let target_name = target.name.to_string();
                        let is_valid = target_id != src_str;

                        w.set_drop_target_index(target_idx);
                        w.set_drop_target_id(target_id.into());
                        w.set_drop_target_name(target_name.into());
                        w.set_drop_position("after".into());
                        w.set_drop_target_valid(is_valid);
                    }
                } else {
                    w.set_drop_target_index(-1);
                    w.set_drop_target_id("".into());
                    w.set_drop_target_name("".into());
                    w.set_drop_position("".into());
                    w.set_drop_target_valid(false);
                }
                return;
            }

            // 2. 树形层级模式 (Tree View Mode)
            let visible_nodes = w.get_tree_nodes();
            let total_len = visible_nodes.row_count();

            if (target_idx as usize) < total_len {
                if let Some(target) = visible_nodes.row_data(target_idx as usize) {
                    let is_target_group = target.is_group;
                    let target_id = target.id.to_string();
                    let target_name = target.name.to_string();

                    // 核心规则：
                    // 1. 拖到文件夹（或文件夹下线） -> 移入该文件夹内部 ("inside")
                    // 2. 拖到文件夹下的主机（或主机下线） -> 排在该主机的下面 ("after")
                    let position = if is_target_group {
                        "inside"
                    } else {
                        "after"
                    };

                    // 循环引用与自身校验 (读取 master_tree)
                    let tree = master_tree_hover.borrow();
                    let mut is_valid = true;
                    if target_id == src_str {
                        is_valid = false;
                    } else {
                        let is_src_group = tree.iter().find(|n| n.id == src_str).map(|n| n.is_group).unwrap_or(false);
                        if is_src_group {
                            let mut curr = if position == "inside" {
                                target_id.clone()
                            } else {
                                target.parent_id.to_string()
                            };
                            while !curr.is_empty() {
                                if curr == src_str {
                                    is_valid = false;
                                    break;
                                }
                                if let Some(pn) = tree.iter().find(|n| n.id == curr) {
                                    curr = pn.parent_id.clone();
                                } else {
                                    break;
                                }
                            }
                        }
                    }

                    w.set_drop_target_index(target_idx);
                    w.set_drop_target_id(target_id.into());
                    w.set_drop_target_name(target_name.into());
                    w.set_drop_position(position.into());
                    w.set_drop_target_valid(is_valid);
                }
            } else {
                w.set_drop_target_index(-1);
                w.set_drop_target_id("".into());
                w.set_drop_target_name("".into());
                w.set_drop_position("".into());
                w.set_drop_target_valid(false);
            }
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

    // 绑定主机实时搜索过滤回调 (双向联动树形视图与卡片列表，保持自定义排序)
    let window_weak = window.as_weak();
    let master_tree_filter = Rc::clone(&master_tree);
    let master_cards_filter = Rc::clone(&master_cards);
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

            // 2. 动态过滤卡片列表 (基于当前 master_cards 列表及用户自定义排序)
            let cards = master_cards_filter.borrow();
            let filtered_cards: Vec<HostItemData> = cards
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
                .cloned()
                .collect();
            w.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(filtered_cards))));

            if !q.is_empty() {
                tracing::debug!(target: "smagical_ui::search", "过滤主机资产: '{}'", q);
                sync_ui_debug_logs(&w);
            }
        }
    });

    // 绑定主机打开回调 (从左侧主机列表双击时打开或多开对应的终端 Tab)
    let window_weak = window.as_weak();
    let master_tree_open = Rc::clone(&master_tree);
    let active_sessions_open = Rc::clone(&active_sessions);
    let next_session_num_open = Rc::clone(&next_session_num);
    window.on_open_host(move |host_id| {
        if let Some(w) = window_weak.upgrade() {
            let h_id = host_id.to_string();

            // 支持启动本地终端环境 (动态匹配探测到的环境)
            if h_id.starts_with("local-") {
                let mut sessions = active_sessions_open.borrow_mut();
                let mut num = next_session_num_open.borrow_mut();

                let sess_id = format!("sess-{}", *num);
                *num += 1;

                let all_shells = local_shells::detect_local_shells();
                let (base_name, addr) = if let Some(sh) = all_shells.iter().find(|s| s.id == h_id.as_str()) {
                    (sh.title.to_string(), format!("Local ({})", sh.subtitle))
                } else {
                    let fallback_name = match h_id.as_str() {
                        "local-pwsh7" => "PowerShell 7",
                        "local-powershell" => "PowerShell",
                        "local-wsl" => "WSL (Linux)",
                        "local-cmd" => "Command Prompt",
                        "local-gitbash" => "Git Bash",
                        "local-bash" => "Bash",
                        "local-zsh" => "Zsh",
                        "local-fish" => "Fish",
                        "local-sh" => "Sh",
                        "local-nushell" => "Nushell",
                        _ => "Local Shell",
                    };
                    (fallback_name.to_string(), "Local Terminal".to_string())
                };

                let count = sessions.iter().filter(|s| s.host_id == h_id).count();
                let display_title = if count == 0 {
                    base_name.clone()
                } else {
                    format!("{} ({})", base_name, count + 1)
                };

                let new_sess = TerminalSessionInfo {
                    session_id: sess_id.clone(),
                    host_id: h_id.clone(),
                    host_name: base_name,
                    host_address: addr,
                    host_status: "online".to_string(),
                    ping_ms: 0,
                    display_title: display_title.clone(),
                };

                sessions.push(new_sess);
                sync_active_session_ui(&w, &sessions, &sess_id);

                tracing::info!(target: "smagical_ui::session", "启动本地终端环境: {} -> Session ID: {}", display_title, sess_id);
                sync_ui_debug_logs(&w);
                return;
            }

            let tree = master_tree_open.borrow();

            // 查找目标主机节点
            if let Some(node) = tree.iter().find(|n| n.id == h_id && !n.is_group) {
                let mut sessions = active_sessions_open.borrow_mut();
                let mut num = next_session_num_open.borrow_mut();

                let sess_id = format!("sess-{}", *num);
                *num += 1;

                // 计算该主机已有多少个活跃会话 (用于智能多开编号: name, name (2), name (3)...)
                let count = sessions.iter().filter(|s| s.host_id == h_id).count();
                let display_title = if count == 0 {
                    node.name.clone()
                } else {
                    format!("{} ({})", node.name, count + 1)
                };

                let addr = if node.address.is_empty() {
                    "127.0.0.1:22".to_string()
                } else if node.port > 0 {
                    format!("{}:{}", node.address, node.port)
                } else {
                    node.address.clone()
                };

                let new_sess = TerminalSessionInfo {
                    session_id: sess_id.clone(),
                    host_id: h_id.clone(),
                    host_name: node.name.clone(),
                    host_address: addr,
                    host_status: node.status.clone(),
                    ping_ms: node.ping_ms,
                    display_title: display_title.clone(),
                };

                sessions.push(new_sess);
                sync_active_session_ui(&w, &sessions, &sess_id);

                tracing::info!(target: "smagical_ui::session", "发起远程终端连接: {} -> Session ID: {}", display_title, sess_id);
                sync_ui_debug_logs(&w);
            }
        }
    });

    // 绑定新建 Tab 回调 (点击 Tab 栏 + 号时打开快速新建终端会话中心居中弹窗)
    let window_weak = window.as_weak();
    let master_tree_reset = Rc::clone(&master_tree);
    window.on_new_tab(move || {
        if let Some(w) = window_weak.upgrade() {
            // 重置搜索框与弹窗列表 (动态探测当前系统的真实终端)
            let detected_shells = local_shells::detect_local_shells();
            w.set_launcher_local_items(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(detected_shells))));

            let tree = master_tree_reset.borrow();
            let all_hosts: Vec<HostItemData> = tree
                .iter()
                .filter(|n| !n.is_group)
                .map(|n| HostItemData {
                    id: n.id.clone().into(),
                    name: n.name.clone().into(),
                    address: n.address.clone().into(),
                    port: n.port,
                    group: "".into(),
                    status: n.status.clone().into(),
                    ping_ms: n.ping_ms,
                })
                .collect();
            w.set_launcher_host_items(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(all_hosts))));

            w.set_is_new_session_modal_open(true);
        }
    });

    // 绑定新建终端会话弹窗实时搜索过滤回调
    let window_weak = window.as_weak();
    let master_tree_launcher = Rc::clone(&master_tree);
    window.on_filter_launcher(move |query| {
        if let Some(w) = window_weak.upgrade() {
            let q = query.trim().to_lowercase();

            let all_local_shells = local_shells::detect_local_shells();

            let filtered_locals: Vec<LocalShellItemData> = if q.is_empty() {
                all_local_shells
            } else {
                all_local_shells
                    .into_iter()
                    .filter(|s| {
                        let t = s.title.to_lowercase();
                        let sub = s.subtitle.to_lowercase();
                        let id = s.id.to_lowercase();
                        let tag = s.tag.to_lowercase();
                        t.contains(&q) || sub.contains(&q) || id.contains(&q) || tag.contains(&q)
                            || (q.contains("wsl") && (id.contains("wsl") || sub.contains("wsl")))
                            || (q.contains("ps") && (id.contains("powershell") || id.contains("pwsh")))
                            || (q.contains("bash") && (id.contains("bash") || id.contains("wsl")))
                            || (q.contains("zsh") && id.contains("zsh"))
                            || (q.contains("fish") && id.contains("fish"))
                            || (q.contains("cmd") && id.contains("cmd"))
                    })
                    .collect()
            };
            w.set_launcher_local_items(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(filtered_locals))));

            let tree = master_tree_launcher.borrow();
            let filtered_hosts: Vec<HostItemData> = tree
                .iter()
                .filter(|n| !n.is_group)
                .filter(|n| {
                    if q.is_empty() {
                        true
                    } else {
                        n.name.to_lowercase().contains(&q)
                            || n.address.to_lowercase().contains(&q)
                            || n.parent_id.to_lowercase().contains(&q)
                    }
                })
                .map(|n| HostItemData {
                    id: n.id.clone().into(),
                    name: n.name.clone().into(),
                    address: n.address.clone().into(),
                    port: n.port,
                    group: "".into(),
                    status: n.status.clone().into(),
                    ping_ms: n.ping_ms,
                })
                .collect();

            w.set_launcher_host_items(slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(filtered_hosts))));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tree() -> Vec<RawTreeNode> {
        vec![
            RawTreeNode {
                id: "grp-a".into(),
                name: "分组A".into(),
                is_group: true,
                parent_id: "".into(),
                level: 0,
                address: "".into(),
                port: 0,
                status: "online".into(),
                ping_ms: 0,
                item_count: 2,
            },
            RawTreeNode {
                id: "grp-a-sub".into(),
                name: "子分组A1".into(),
                is_group: true,
                parent_id: "grp-a".into(),
                level: 1,
                address: "".into(),
                port: 0,
                status: "online".into(),
                ping_ms: 0,
                item_count: 1,
            },
            RawTreeNode {
                id: "host-1".into(),
                name: "host-01".into(),
                is_group: false,
                parent_id: "grp-a-sub".into(),
                level: 2,
                address: "10.0.0.1".into(),
                port: 22,
                status: "online".into(),
                ping_ms: 10,
                item_count: 0,
            },
            RawTreeNode {
                id: "grp-b".into(),
                name: "分组B".into(),
                is_group: true,
                parent_id: "".into(),
                level: 0,
                address: "".into(),
                port: 0,
                status: "online".into(),
                ping_ms: 0,
                item_count: 0,
            },
            RawTreeNode {
                id: "host-root".into(),
                name: "host-root-node".into(),
                is_group: false,
                parent_id: "".into(),
                level: 0,
                address: "10.0.0.99".into(),
                port: 22,
                status: "online".into(),
                ping_ms: 5,
                item_count: 0,
            },
        ]
    }

    #[test]
    fn test_move_host_inside_group() {
        let mut tree = create_test_tree();
        let res = move_and_reorder_raw_node(&mut tree, "host-1", "grp-b", "inside");
        assert!(res.is_ok());
        let (src_name, target_name) = res.unwrap();
        assert_eq!(src_name, "host-01");
        assert_eq!(target_name, "分组B");

        let host = tree.iter().find(|n| n.id == "host-1").unwrap();
        assert_eq!(host.parent_id, "grp-b");
        assert_eq!(host.level, 1);
    }

    #[test]
    fn test_reorder_before() {
        let mut tree = create_test_tree();
        // 将 grp-b 拖到 grp-a 前面 (Before 调序)
        let res = move_and_reorder_raw_node(&mut tree, "grp-b", "grp-a", "before");
        assert!(res.is_ok());

        assert_eq!(tree[0].id, "grp-b");
        assert_eq!(tree[1].id, "grp-a");
    }

    #[test]
    fn test_reorder_after() {
        let mut tree = create_test_tree();
        // 将 host-root 拖到 grp-a 后面 (After 调序)
        let res = move_and_reorder_raw_node(&mut tree, "host-root", "grp-a", "after");
        assert!(res.is_ok());

        // grp-a 包含其子树 (grp-a, grp-a-sub, host-1)，host-root 应位于子树后面
        let host_pos = tree.iter().position(|n| n.id == "host-root").unwrap();
        let host1_pos = tree.iter().position(|n| n.id == "host-1").unwrap();
        assert!(host_pos > host1_pos);
    }

    #[test]
    fn test_move_host_to_root() {
        let mut tree = create_test_tree();
        let res = move_and_reorder_raw_node(&mut tree, "host-1", "root", "root");
        assert!(res.is_ok());

        let host = tree.iter().find(|n| n.id == "host-1").unwrap();
        assert_eq!(host.parent_id, "");
        assert_eq!(host.level, 0);
    }

    #[test]
    fn test_move_group_with_children() {
        let mut tree = create_test_tree();
        let res = move_and_reorder_raw_node(&mut tree, "grp-a-sub", "grp-b", "inside");
        assert!(res.is_ok());

        let sub = tree.iter().find(|n| n.id == "grp-a-sub").unwrap();
        assert_eq!(sub.parent_id, "grp-b");
        assert_eq!(sub.level, 1);

        let host = tree.iter().find(|n| n.id == "host-1").unwrap();
        assert_eq!(host.parent_id, "grp-a-sub");
        assert_eq!(host.level, 2);
    }

    #[test]
    fn test_prevent_cycle_moving_parent_to_child() {
        let mut tree = create_test_tree();
        // grp-a 是 grp-a-sub 的父级，不能将 grp-a 移入 grp-a-sub
        let res = move_and_reorder_raw_node(&mut tree, "grp-a", "grp-a-sub", "inside");
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("循环引用"));
    }

    #[test]
    fn test_cannot_move_inside_self() {
        let mut tree = create_test_tree();
        let res = move_and_reorder_raw_node(&mut tree, "grp-a", "grp-a", "inside");
        assert!(res.is_err());
    }
}
