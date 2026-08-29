//! 主机资产树形数据模型与纯函数操作层。
//!
//! 所有函数均为无副作用的纯变换函数，供 `run()` 组装层调用。

use std::collections::HashSet;
use smagical_core::{AppStorage, GroupRecord, HostRecord};
use smagical_debug::{calculate_node_width, DebugRawNode};

use crate::generated::{GroupOptionData, HostTreeNode};

/// 原始树形节点数据结构 (Raw Tree Node)
///
/// 内部核心状态模型，用于完整表达主机管理中所有的分组节点与主机实例节点。
#[derive(Clone, Debug)]
pub(crate) struct RawTreeNode {
    /// 节点的全局唯一 ID (如: "grp-prod"、"host-k8s-w1")
    pub(crate) id: String,
    /// 节点的展示名称 (如: "生产集群 (Production)"、"k8s-control-plane")
    pub(crate) name: String,
    /// 是否为分组节点 (true: 文件夹分组, false: 具体主机资产)
    pub(crate) is_group: bool,
    /// 所属直接父级节点的 ID (顶级根节点为空字符串 "")
    pub(crate) parent_id: String,
    /// 树状层级深度 (0: 顶级根节点, 1: 一级子节点, 2: 二级子节点...)
    pub(crate) level: i32,
    /// 主机 IP 地址或域名 (仅主机节点有效，分组节点为空字符串)
    pub(crate) address: String,
    /// SSH 连接端口 (例如: 22, 6443, 5432)
    pub(crate) port: i32,
    /// 主机在线状态字符串 ("online" 在线, "warning" 告警, "offline" 离线)
    pub(crate) status: String,
    /// ICMP 网络延迟测速结果 (单位: 毫秒，0 表示未测速或离线)
    pub(crate) ping_ms: i32,
    /// 分组下包含的直属子项总数量 (含子分组 + 直属主机，仅分组节点有效)
    pub(crate) item_count: i32,
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

/// 解析路径（如 "集群/k8s" 或 "亚太/中国区/杭州"）并在树中逐级确保嵌套分组节点存在。
///
/// 返回 (叶子分组 ID, 叶子分组深度层级, 叶子分组展示名称)
pub(crate) fn ensure_raw_group_hierarchy(tree: &mut Vec<RawTreeNode>, path: &str) -> (String, i32, String) {
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

/// 移动与调序树形节点（主机或分组）。
///
/// 支持四种落点模式：
/// - `"inside"`: 移入目标分组内部作为其子节点
/// - `"before"`: 插在目标节点上方（成为目标节点的同级前序节点）
/// - `"after"`: 插在目标节点下方（成为目标节点的同级后序节点）
/// - `"root"`: 移至顶级根目录
pub(crate) fn move_and_reorder_raw_node(
    tree: &mut Vec<RawTreeNode>,
    source_id: &str,
    target_id: &str,
    drop_position: &str,
) -> Result<(String, String), String> {
    let source_idx = tree
        .iter()
        .position(|n| n.id == source_id)
        .ok_or_else(|| "未找到源节点".to_string())?;

    let is_source_group = tree[source_idx].is_group;
    let source_name = tree[source_idx].name.clone();
    let old_level = tree[source_idx].level;

    if source_id == target_id {
        if drop_position == "inside" {
            return Err("不能将节点移入自身内部".to_string());
        }
        return Ok((source_name.clone(), source_name));
    }

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

    if source_descendant_ids.contains(target_id) {
        return Err("不能将父分组移动至其子孙节点中 (循环引用)".to_string());
    }

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
            (target_node.parent_id.clone(), target_node.level, target_node.name.clone())
        }
    };

    let level_delta = new_level - old_level;

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

    if drop_position == "before" {
        let target_pos = remaining_tree.iter().position(|n| n.id == target_id).unwrap_or(0);
        for (i, node) in subtree_nodes.into_iter().enumerate() {
            remaining_tree.insert(target_pos + i, node);
        }
    } else if drop_position == "after" {
        let target_pos = remaining_tree
            .iter()
            .position(|n| n.id == target_id)
            .unwrap_or_else(|| remaining_tree.len().saturating_sub(1));

        let mut insert_pos = target_pos + 1;
        if remaining_tree[target_pos].is_group {
            let mut target_descendants = HashSet::new();
            let mut q = vec![target_id.to_string()];
            while let Some(p) = q.pop() {
                for n in &remaining_tree {
                    if n.parent_id == p {
                        target_descendants.insert(n.id.clone());
                        if n.is_group { q.push(n.id.clone()); }
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
        let target_pos = remaining_tree.iter().position(|n| n.id == target_id).unwrap_or(0);
        let mut insert_pos = target_pos + 1;
        let mut target_descendants = HashSet::new();
        let mut q = vec![target_id.to_string()];
        while let Some(p) = q.pop() {
            for n in &remaining_tree {
                if n.parent_id == p {
                    target_descendants.insert(n.id.clone());
                    if n.is_group { q.push(n.id.clone()); }
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
        remaining_tree.extend(subtree_nodes);
    }

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

/// 对树形结构节点进行标准化排序 (文件夹始终置顶，主机在下方，深度优先)。
pub(crate) fn sort_tree_hierarchy(tree: &[RawTreeNode]) -> Vec<RawTreeNode> {
    let mut result = Vec::with_capacity(tree.len());

    fn collect_children(parent_id: &str, tree: &[RawTreeNode], result: &mut Vec<RawTreeNode>) {
        let mut children: Vec<&RawTreeNode> = tree.iter().filter(|n| n.parent_id == parent_id).collect();
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

    for node in tree {
        if !result.iter().any(|r| r.id == node.id) {
            result.push(node.clone());
        }
    }

    result
}

/// 从底层存储门面 (AppStorage) 构建 UI 层专用的全量原始树形节点列表。
pub(crate) fn build_raw_tree_from_storage(storage: &dyn AppStorage) -> Vec<RawTreeNode> {
    let groups = storage.groups().list_all().unwrap_or_default();
    let hosts = storage.hosts().list_all().unwrap_or_default();

    let mut result = Vec::new();

    fn insert_children(
        parent_id_opt: Option<&str>,
        level: i32,
        groups: &[GroupRecord],
        hosts: &[HostRecord],
        out: &mut Vec<RawTreeNode>,
    ) {
        let current_groups: Vec<&GroupRecord> = groups
            .iter()
            .filter(|g| match parent_id_opt {
                Some(p_id) => g.parent_id.as_deref() == Some(p_id),
                None => g.parent_id.is_none() || g.parent_id.as_deref() == Some(""),
            })
            .collect();

        for g in current_groups {
            let child_group_count = groups
                .iter()
                .filter(|child_g| child_g.parent_id.as_deref() == Some(g.id.as_str()))
                .count();
            let child_host_count = hosts
                .iter()
                .filter(|h| h.parent_group_id.as_deref() == Some(&g.id))
                .count();

            out.push(RawTreeNode {
                id: g.id.clone(),
                name: g.name.clone(),
                is_group: true,
                parent_id: g.parent_id.clone().unwrap_or_default(),
                level,
                address: String::new(),
                port: 0,
                status: "online".to_string(),
                ping_ms: 0,
                item_count: (child_group_count + child_host_count) as i32,
            });

            insert_children(Some(&g.id), level + 1, groups, hosts, out);
        }

        let current_hosts: Vec<&HostRecord> = hosts
            .iter()
            .filter(|h| match parent_id_opt {
                Some(p_id) => h.parent_group_id.as_deref() == Some(p_id),
                None => h.parent_group_id.is_none() || h.parent_group_id.as_deref() == Some(""),
            })
            .collect();

        for h in current_hosts {
            out.push(RawTreeNode {
                id: h.id.clone(),
                name: h.name.clone(),
                is_group: false,
                parent_id: h.parent_group_id.clone().unwrap_or_default(),
                level,
                address: h.address.clone(),
                port: h.port as i32,
                status: h.status.to_string(),
                ping_ms: h.ping_ms,
                item_count: 0,
            });
        }
    }

    insert_children(None, 0, &groups, &hosts, &mut result);
    sort_tree_hierarchy(&result)
}

/// 构建新建分组弹窗中的上级分组树形选项数据模型。
pub(crate) fn build_group_options(tree: &[RawTreeNode], expanded: &HashSet<String>) -> Vec<GroupOptionData> {
    let mut options = Vec::new();

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

        let mut is_visible = true;
        let mut current_parent = if node.parent_id.is_empty() { "root" } else { node.parent_id.as_str() };
        while !current_parent.is_empty() {
            if !expanded.contains(current_parent) {
                is_visible = false;
                break;
            }
            if current_parent == "root" { break; }
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

/// 构建主界面当前可见的树形视图节点。
pub(crate) fn build_visible_tree_nodes(tree: &[RawTreeNode], expanded: &HashSet<String>) -> Vec<HostTreeNode> {
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

/// 根据搜索关键字构建匹配的树形结构节点。
pub(crate) fn build_search_tree_nodes(tree: &[RawTreeNode], query: &str) -> Vec<HostTreeNode> {
    let q = query.to_lowercase();
    let mut matching_or_needed_ids = HashSet::new();

    for node in tree {
        let is_match = node.name.to_lowercase().contains(&q)
            || node.address.to_lowercase().contains(&q);
        if is_match {
            matching_or_needed_ids.insert(node.id.clone());
            if node.is_group {
                for child in tree {
                    if child.parent_id == node.id {
                        matching_or_needed_ids.insert(child.id.clone());
                    }
                }
            }
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

/// 计算可见树形节点列表所需的最大呈现宽度 (像素)。
pub(crate) fn calculate_max_tree_width(nodes: &[HostTreeNode]) -> f32 {
    let mut max_w: f32 = 240.0;
    for node in nodes {
        let w = calculate_node_width(node.name.as_str(), node.level);
        if w > max_w { max_w = w; }
    }
    max_w
}
