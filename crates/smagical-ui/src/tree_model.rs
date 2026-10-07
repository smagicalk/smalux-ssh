//! 主机资产树形数据模型与纯函数操作层。
//!
//! 所有函数均为无副作用的纯变换函数，供 `run()` 组装层调用。

use std::collections::{HashMap, HashSet};
use smagical_core::{GroupRecord, HostRecord};
use crate::common::{matches_any_ignore_case, ToSharedString};
use crate::debug::{calculate_node_width, DebugRawNode};

use crate::generated::{GroupOptionData, HostItemData, HostTreeNode};

/// 原始树形节点数据结构 (Raw Tree Node)
///
/// 内部核心状态模型，用于完整表达主机管理中所有的分组节点与主机实例节点。
#[derive(Clone, Debug, Default)]
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
    /// 快速登录生效用户名 (经凭据与主机配置计算解析得出的有效用户)
    pub(crate) effective_username: Option<String>,
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
            effective_username: None,
        }
    }
}

/// 解析路径（如 "集群/k8s" 或 "亚太/中国区/杭州"）并在树中逐级确保嵌套分组节点存在。
///
/// 若路径中某个中间分组不存在，则会自动创建并在内存树中追加对应的分组节点。
///
/// # 参数
/// - `tree`: 内存原始节点列表的可变借用
/// - `path`: 以正斜杠或反斜杠分隔的分组层级路径
///
/// # 返回值
/// `(leaf_id, leaf_level, leaf_display_name)`
/// - `leaf_id`: 最终叶子分组节点的唯一标识 ID
/// - `leaf_level`: 最终叶子分组在树中的深度（顶级为 0）
/// - `leaf_display_name`: 最终叶子分组的显示名称
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
                effective_username: None,
            });
            current_parent_id = grp_id;
            current_level = idx as i32;
        }
    }

    (current_parent_id, current_level, last_name)
}

/// 移动与调序树形节点（主机或分组）。
///
/// 严格保证树形拓扑一致性，自动阻止自身移入自身或将父分组移入其子孙节点的环路行为。
/// 若移动的是分组节点，会自动递归迁移其下属整棵子树并自动重算所有子节点的深度 `level` 与各分组的 `item_count`。
///
/// # 参数
/// - `tree`: 内存原始节点列表的可变借用
/// - `source_id`: 待移动源节点 ID
/// - `target_id`: 目标节点 ID (若为 "root" 或空字符串则移动至顶级)
/// - `drop_position`: 落点模式：
///   - `"inside"`: 移入目标分组内部作为其直属子节点
///   - `"before"`: 插在目标节点上方（成为同级前序节点）
///   - `"after"`: 插在目标节点下方（成为同级后序节点）
///   - `"root"`: 移至顶级根目录
///
/// # 返回值
/// `Ok((source_name, target_name))` 成功返回源名称与目标名称；`Err(err_msg)` 失败返回防呆拒绝原因。
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

/// 对树形结构节点进行标准化深度优先排序。
///
/// 排序规则：
/// 1. 深度优先递归遍历 (DFS)
/// 2. 同级节点中，文件夹分组 (`is_group = true`) 始终置顶排列在具体主机前面
/// 3. 同类型节点按名称不区分大小写升序排列 (`name.to_lowercase()`)
///
/// # 参数
/// - `tree`: 乱序的原始节点切片
///
/// # 返回值
/// 排序后的线性树形节点向量
pub(crate) fn sort_tree_hierarchy(tree: &[RawTreeNode]) -> Vec<RawTreeNode> {
    let mut result = Vec::with_capacity(tree.len());

    // 预分组：按 parent_id 归纳直接子节点列表，避免递归中重复 O(N) 线性过滤
    let mut children_map: HashMap<&str, Vec<&RawTreeNode>> = HashMap::with_capacity(tree.len());
    for n in tree {
        children_map.entry(n.parent_id.as_str()).or_default().push(n);
    }

    fn collect_children<'a>(
        parent_id: &str,
        children_map: &mut HashMap<&'a str, Vec<&'a RawTreeNode>>,
        result: &mut Vec<RawTreeNode>,
    ) {
        if let Some(mut children) = children_map.remove(parent_id) {
            children.sort_by(|a, b| {
                b.is_group
                    .cmp(&a.is_group)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
            for child in children {
                let is_grp = child.is_group;
                let cid = child.id.clone();
                result.push(child.clone());
                if is_grp {
                    collect_children(&cid, children_map, result);
                }
            }
        }
    }

    collect_children("", &mut children_map, &mut result);

    // 容错处理：将无有效父节点的游离孤立节点追加至末尾
    if result.len() < tree.len() {
        let seen: HashSet<&str> = result.iter().map(|r| r.id.as_str()).collect();
        let missing: Vec<RawTreeNode> = tree
            .iter()
            .filter(|n| !seen.contains(n.id.as_str()))
            .cloned()
            .collect();
        drop(seen);
        result.extend(missing);
    }

    result
}

/// 纯内存装配全量主机/分组树结构数据 (0ms UI 阻塞，无任何 I/O 耗时)
pub(crate) fn build_raw_tree(
    groups: &[GroupRecord],
    hosts: &[HostRecord],
    credentials: &[smagical_core::CredentialRecord],
) -> Vec<RawTreeNode> {
    let mut result = Vec::with_capacity(groups.len() + hosts.len());

    // 预构建凭据索引 (O(1) 用户名查询)
    let cred_map: HashMap<&str, &str> = credentials
        .iter()
        .filter_map(|c| c.username.as_deref().map(|u| (c.id.as_str(), u)))
        .collect();

    // 预先建立分组与主机的 parent_id 映射，将装配复杂度从 O(N^2) 降至 O(N)
    let mut group_children: HashMap<&str, Vec<&GroupRecord>> = HashMap::new();
    let mut host_children: HashMap<&str, Vec<&HostRecord>> = HashMap::new();

    for g in groups {
        let pid = g.parent_id.as_deref().unwrap_or("");
        group_children.entry(pid).or_default().push(g);
    }

    for h in hosts {
        let pid = h.parent_group_id.as_deref().unwrap_or("");
        host_children.entry(pid).or_default().push(h);
    }

    fn insert_children(
        parent_id: &str,
        level: i32,
        group_children: &HashMap<&str, Vec<&GroupRecord>>,
        host_children: &HashMap<&str, Vec<&HostRecord>>,
        cred_map: &HashMap<&str, &str>,
        out: &mut Vec<RawTreeNode>,
    ) {
        if let Some(current_groups) = group_children.get(parent_id) {
            for g in current_groups {
                let child_group_count = group_children.get(g.id.as_str()).map(|v| v.len()).unwrap_or(0);
                let child_host_count = host_children.get(g.id.as_str()).map(|v| v.len()).unwrap_or(0);

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
                    effective_username: None,
                });

                insert_children(g.id.as_str(), level + 1, group_children, host_children, cred_map, out);
            }
        }

        if let Some(current_hosts) = host_children.get(parent_id) {
            for h in current_hosts {
                let eff_user = if let Some(ref cid) = h.credential_id {
                    cred_map.get(cid.as_str()).map(|&u| u.to_string()).or_else(|| h.username.clone())
                } else {
                    h.username.clone()
                };

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
                    effective_username: eff_user,
                });
            }
        }
    }

    insert_children("", 0, &group_children, &host_children, &cred_map, &mut result);
    sort_tree_hierarchy(&result)
}

/// 纯内存生成平铺卡片列表数据 (O(1) 分组关联查询)
pub(crate) fn build_cards_from_records(
    hosts: &[HostRecord],
    groups: &[GroupRecord],
) -> Vec<HostItemData> {
    let grp_map: HashMap<&str, &str> = groups
        .iter()
        .map(|g| (g.id.as_str(), g.name.as_str()))
        .collect();

    hosts.iter().map(|h| {
        let g_name = if let Some(ref pid) = h.parent_group_id {
            grp_map.get(pid.as_str()).copied().unwrap_or("根目录")
        } else {
            "根目录"
        };
        HostItemData {
            id: h.id.clone().into(),
            name: h.name.clone().into(),
            address: h.address.clone().into(),
            port: h.port as i32,
            group: g_name.into(),
            status: h.status.to_string().into(),
            ping_ms: h.ping_ms,
        }
    }).collect()
}

/// 构建新建分组/主机弹窗中的“上级分组选择器”树形扁平数据模型。
///
/// 仅提取所有分组节点并根据选择器当前的展开集合 (`expanded`) 计算其可见性与展开箭头状态。
///
/// # 性能设计 (Phase 35)
/// 引入 `parent_map` 与 `groups_with_subgroups` 快速索引及记忆化递归，将时间复杂度从 $O(G^2)$ 降为严格 $O(G)$。
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

    // 1. 建立 O(1) 父级索引与子分组指示集
    let mut parent_map = HashMap::with_capacity(tree.len());
    let mut groups_with_subgroups = HashSet::new();
    for node in tree {
        if node.is_group {
            parent_map.insert(node.id.as_str(), node.parent_id.as_str());
            let pid = if node.parent_id.is_empty() { "root" } else { node.parent_id.as_str() };
            groups_with_subgroups.insert(pid);
        }
    }

    fn is_selector_ancestor_expanded<'a>(
        group_id: &'a str,
        expanded: &HashSet<String>,
        parent_map: &HashMap<&'a str, &'a str>,
        memo: &mut HashMap<&'a str, bool>,
    ) -> bool {
        if group_id.is_empty() || group_id == "root" {
            return expanded.contains("root");
        }
        if let Some(&cached) = memo.get(group_id) {
            return cached;
        }
        let is_vis = if !expanded.contains(group_id) {
            false
        } else {
            let parent = parent_map.get(group_id).copied().unwrap_or("");
            let parent_effective = if parent.is_empty() { "root" } else { parent };
            is_selector_ancestor_expanded(parent_effective, expanded, parent_map, memo)
        };
        memo.insert(group_id, is_vis);
        is_vis
    }

    let mut memo = HashMap::with_capacity(tree.len().min(64));

    for node in tree {
        if !node.is_group {
            continue;
        }

        let current_parent = if node.parent_id.is_empty() { "root" } else { node.parent_id.as_str() };
        let is_visible = is_selector_ancestor_expanded(current_parent, expanded, &parent_map, &mut memo);

        if is_visible {
            let has_children = groups_with_subgroups.contains(node.id.as_str());
            let is_expanded = has_children && expanded.contains(&node.id);
            options.push(GroupOptionData {
                id: node.id.to_shared(),
                name: node.name.to_shared(),
                level: node.level + 1,
                parent_id: if node.parent_id.is_empty() { "root".into() } else { node.parent_id.to_shared() },
                has_children,
                is_expanded,
            });
        }
    }
    options
}

/// 构建左侧抽屉树形视图中当前实际可见的树形节点列表。
///
/// 遵循祖先链折叠可见性规则：只有当一个节点的所有祖先分组均处于 `expanded` 集合中时，该节点才输出至前端渲染。
///
/// # 性能设计 (Phase 35: $O(N)$ 虚拟化极速装配)
/// 1. 建立 `parent_map` ID 索引，消除每次迭代中 $O(N)$ 递归回溯 `tree.iter().find()`；
/// 2. 引入 `memo` 祖先可见性记忆表，对同一分组分支的可见性仅计算一次 ($O(1)$)；
/// 3. 全局时间复杂度由原始的 $O(N \cdot D \cdot N)$ 优化至严格的 $O(N)$ 线性时间。
pub(crate) fn build_visible_tree_nodes(tree: &[RawTreeNode], expanded: &HashSet<String>) -> Vec<HostTreeNode> {
    if tree.is_empty() {
        return Vec::new();
    }

    let mut parent_map = HashMap::with_capacity(tree.len());
    for node in tree {
        parent_map.insert(node.id.as_str(), node.parent_id.as_str());
    }

    fn is_ancestor_chain_visible<'a>(
        parent_id: &'a str,
        expanded: &HashSet<String>,
        parent_map: &HashMap<&'a str, &'a str>,
        memo: &mut HashMap<&'a str, bool>,
    ) -> bool {
        if parent_id.is_empty() {
            return true;
        }
        if let Some(&cached) = memo.get(parent_id) {
            return cached;
        }
        let is_vis = if !expanded.contains(parent_id) {
            false
        } else if let Some(&grandparent_id) = parent_map.get(parent_id) {
            is_ancestor_chain_visible(grandparent_id, expanded, parent_map, memo)
        } else {
            true
        };
        memo.insert(parent_id, is_vis);
        is_vis
    }

    let mut memo = HashMap::with_capacity(tree.len().min(128));
    let mut visible = Vec::with_capacity(tree.len());

    for node in tree {
        let is_visible = if node.parent_id.is_empty() {
            true
        } else {
            is_ancestor_chain_visible(node.parent_id.as_str(), expanded, &parent_map, &mut memo)
        };

        if is_visible {
            let is_expanded = node.is_group && expanded.contains(&node.id);
            visible.push(HostTreeNode {
                id: node.id.to_shared(),
                name: node.name.to_shared(),
                is_group: node.is_group,
                parent_id: node.parent_id.to_shared(),
                level: node.level,
                is_expanded,
                address: node.address.to_shared(),
                port: node.port,
                status: node.status.to_shared(),
                ping_ms: node.ping_ms,
                item_count: node.item_count,
            });
        }
    }
    visible
}

/// 根据搜索关键字构建匹配的高亮树形结构节点。
///
/// 搜索匹配算法：
/// 1. 匹配节点自身名称、IP 地址、端口号、用户名或在线状态 (零堆分配 ASCII 快径)；
/// 2. 若某个分组匹配，则其所有子孙节点全部展开显示；
/// 3. 若某个子节点匹配，则自动向上递归回溯保留其所有祖先节点并强制置为展开状态 (短路去重)；
/// 4. 基于双向索引映射，整体计算从 $O(N^2)$ 压缩至 $O(N)$。
pub(crate) fn build_search_tree_nodes(tree: &[RawTreeNode], query: &str) -> Vec<HostTreeNode> {
    let q = query.trim();
    if q.is_empty() {
        return Vec::new();
    }

    // 1. 构建 O(1) 父子双向拓扑映射
    let mut parent_map = HashMap::with_capacity(tree.len());
    let mut children_map: HashMap<&str, Vec<&str>> = HashMap::with_capacity(tree.len());
    for node in tree {
        parent_map.insert(node.id.as_str(), node.parent_id.as_str());
        if !node.parent_id.is_empty() {
            children_map.entry(node.parent_id.as_str()).or_default().push(node.id.as_str());
        }
    }

    let mut matching_or_needed_ids = HashSet::new();

    for node in tree {
        // 多维度轻量快速匹配：名称、IP地址、端口、生效用户名、状态
        let port_buf = if node.port > 0 { node.port.to_string() } else { String::new() };
        let eff_user = node.effective_username.as_deref().unwrap_or("");
        let is_match = matches_any_ignore_case(
            &[&node.name, &node.address, &port_buf, eff_user, &node.status],
            q,
        );

        if is_match {
            matching_or_needed_ids.insert(node.id.clone());

            // 若匹配的是分组节点，递归将所有子孙节点纳入匹配集合
            if node.is_group {
                let mut q_children = vec![node.id.as_str()];
                while let Some(parent) = q_children.pop() {
                    if let Some(children) = children_map.get(parent) {
                        for &child_id in children {
                            if matching_or_needed_ids.insert(child_id.to_string()) {
                                q_children.push(child_id);
                            }
                        }
                    }
                }
            }

            // 沿祖先链向上回溯，将全部父级分组加入展开集合 (已存在则提前短路)
            let mut cur_parent = node.parent_id.as_str();
            while !cur_parent.is_empty() {
                if !matching_or_needed_ids.insert(cur_parent.to_string()) {
                    break;
                }
                cur_parent = parent_map.get(cur_parent).copied().unwrap_or("");
            }
        }
    }

    let mut result = Vec::new();
    for node in tree {
        if matching_or_needed_ids.contains(&node.id) {
            result.push(HostTreeNode {
                id: node.id.to_shared(),
                name: node.name.to_shared(),
                is_group: node.is_group,
                parent_id: node.parent_id.to_shared(),
                level: node.level,
                is_expanded: true,
                address: node.address.to_shared(),
                port: node.port,
                status: node.status.to_shared(),
                ping_ms: node.ping_ms,
                item_count: node.item_count,
            });
        }
    }
    result
}

/// 计算可见树形节点列表所需的最大呈现宽度 (单位: 逻辑像素 px)。
///
/// 综合考虑节点缩进层级 `level * 14px`、图标宽度、文本长度及右侧状态标签宽度，用于横向滚动条自适应撑开。
pub(crate) fn calculate_max_tree_width(nodes: &[HostTreeNode]) -> f32 {
    let mut max_w: f32 = 240.0;
    for node in nodes {
        let w = calculate_node_width(node.name.as_str(), node.level);
        if w > max_w { max_w = w; }
    }
    max_w
}

/// 单次视口最大推送节点/卡片数量（超过则截断并提供“显示全部”交互，防止 Slint ModelRc 产生百万级瞬态堆分配）
pub(crate) const MAX_VIEWPORT_ITEMS: usize = 100;

/// 截断卡片列表并返回 (展示列表, 是否被截断, 实际总匹配数)
pub(crate) fn filter_cards(
    cards: &[HostItemData],
    query: &str,
    show_all: bool,
) -> (Vec<HostItemData>, bool, usize) {
    let q = query.trim();
    let matched: Vec<HostItemData> = if q.is_empty() {
        cards.to_vec()
    } else {
        cards
            .iter()
            .filter(|h| {
                let port_str = if h.port > 0 { h.port.to_string() } else { String::new() };
                matches_any_ignore_case(&[&h.name, &h.address, &h.group, &port_str, &h.status], q)
            })
            .cloned()
            .collect()
    };
    let total = matched.len();
    if !show_all && total > MAX_VIEWPORT_ITEMS {
        (matched[..MAX_VIEWPORT_ITEMS].to_vec(), true, total)
    } else {
        (matched, false, total)
    }
}

/// 截断树形可视化节点列表并返回 (展示节点列表, 是否被截断, 实际总匹配数)
pub(crate) fn filter_and_cap_tree_nodes(
    nodes: Vec<HostTreeNode>,
    show_all: bool,
) -> (Vec<HostTreeNode>, bool, usize) {
    let total = nodes.len();
    if !show_all && total > MAX_VIEWPORT_ITEMS {
        (nodes[..MAX_VIEWPORT_ITEMS].to_vec(), true, total)
    } else {
        (nodes, false, total)
    }
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
                effective_username: None,
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
                effective_username: None,
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
                effective_username: None,
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
                effective_username: None,
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
                effective_username: None,
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
        let res = move_and_reorder_raw_node(&mut tree, "grp-b", "grp-a", "before");
        assert!(res.is_ok());

        assert_eq!(tree[0].id, "grp-b");
        assert_eq!(tree[1].id, "grp-a");
    }

    #[test]
    fn test_reorder_after() {
        let mut tree = create_test_tree();
        let res = move_and_reorder_raw_node(&mut tree, "host-root", "grp-a", "after");
        assert!(res.is_ok());

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

    #[test]
    fn test_build_visible_tree_nodes_memoized() {
        let tree = create_test_tree();
        let mut expanded = HashSet::new();

        // 默认无展开：仅根级节点可见
        let vis0 = build_visible_tree_nodes(&tree, &expanded);
        let ids0: Vec<&str> = vis0.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids0, vec!["grp-a", "grp-b", "host-root"]);

        // 展开 grp-a：子分组 grp-a-sub 变为可见，但其子主机 host-1 仍不可见
        expanded.insert("grp-a".to_string());
        let vis1 = build_visible_tree_nodes(&tree, &expanded);
        let ids1: Vec<&str> = vis1.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids1, vec!["grp-a", "grp-a-sub", "grp-b", "host-root"]);

        // 展开 grp-a-sub：host-1 也变为可见
        expanded.insert("grp-a-sub".to_string());
        let vis2 = build_visible_tree_nodes(&tree, &expanded);
        let ids2: Vec<&str> = vis2.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids2, vec!["grp-a", "grp-a-sub", "host-1", "grp-b", "host-root"]);
    }

    #[test]
    fn test_build_search_tree_nodes_multi_field() {
        let mut tree = create_test_tree();
        // 设置主机生效用户名
        if let Some(h) = tree.iter_mut().find(|n| n.id == "host-1") {
            h.effective_username = Some("devops_admin".to_string());
        }

        // 1. IP 地址模糊匹配：匹配到 host-1，自动级联展开其祖先 grp-a-sub 与 grp-a
        let res_ip = build_search_tree_nodes(&tree, "10.0.0.1");
        let ip_ids: Vec<&str> = res_ip.iter().map(|n| n.id.as_str()).collect();
        assert!(ip_ids.contains(&"host-1"));
        assert!(ip_ids.contains(&"grp-a-sub"));
        assert!(ip_ids.contains(&"grp-a"));
        assert!(!ip_ids.contains(&"grp-b"));

        // 2. 生效用户名模糊匹配
        let res_user = build_search_tree_nodes(&tree, "devops");
        let user_ids: Vec<&str> = res_user.iter().map(|n| n.id.as_str()).collect();
        assert!(user_ids.contains(&"host-1"));

        // 3. 分组名称匹配：匹配 grp-a，自动展开其下全部子孙节点
        let res_grp = build_search_tree_nodes(&tree, "分组A");
        let grp_ids: Vec<&str> = res_grp.iter().map(|n| n.id.as_str()).collect();
        assert!(grp_ids.contains(&"grp-a"));
        assert!(grp_ids.contains(&"grp-a-sub"));
        assert!(grp_ids.contains(&"host-1"));
    }

    #[test]
    fn test_filter_cards_and_viewport_capping() {
        let dummy_cards: Vec<HostItemData> = (0..150)
            .map(|i| HostItemData {
                id: format!("h-{}", i).into(),
                name: format!("server-{}", i).into(),
                address: format!("192.168.1.{}", i).into(),
                port: 22,
                group: "测试集群".into(),
                status: "online".into(),
                ping_ms: 12,
            })
            .collect();

        // 默认不展开全部：最多推送 MAX_VIEWPORT_ITEMS (100) 条，并标记截断
        let (capped, is_trunc, total) = filter_cards(&dummy_cards, "", false);
        assert_eq!(total, 150);
        assert_eq!(capped.len(), MAX_VIEWPORT_ITEMS);
        assert!(is_trunc);

        // 开启 show_all：推送全量 150 条，不截断
        let (all, is_trunc_all, total_all) = filter_cards(&dummy_cards, "", true);
        assert_eq!(total_all, 150);
        assert_eq!(all.len(), 150);
        assert!(!is_trunc_all);
    }
}

