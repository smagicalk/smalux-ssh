//! 代码片段树形数据模型与纯函数操作层。
//!
//! 负责多层层级文件夹与代码片段的树形聚合、折叠展开计算、关键字过滤以及路径层级构建。

use std::collections::HashSet;
use smagical_core::AppStorage;
use crate::generated::{GroupOptionData, SnippetTreeNode};

/// 内存层级全量代码片段树节点（包含未展开节点）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSnippetTreeNode {
    /// 唯一标识 (分组如 "sgrp-docker", 片段如 "snip-1")
    pub id: String,
    /// 节点名称/标题
    pub name: String,
    /// 父级 ID ("root" 或具体分组 ID)
    pub parent_id: String,
    /// 嵌套缩进层级 (0 起步)
    pub level: i32,
    /// 是否为文件夹分组 (true: 文件夹, false: 代码片段)
    pub is_group: bool,
    /// 当前是否处于展开状态
    pub is_expanded: bool,
    /// 是否包含子节点
    pub has_children: bool,
    /// 直属子项/脚本数量统计
    pub item_count: i32,
    /// 脚本语言 (Bash, Python 等)
    pub language: String,
    /// 是否自动发送回车执行
    pub auto_execute: bool,
    /// 是否星标置顶
    pub is_favorite: bool,
    /// 排序权重
    pub sort_order: i32,
}

/// 从 AppStorage 构建全量平铺树结构数据
pub fn build_raw_snippet_tree_from_storage(storage: &dyn AppStorage) -> Vec<RawSnippetTreeNode> {
    let mut raw_nodes = Vec::new();
    let groups = storage.snippets().list_groups().unwrap_or_default();
    let snippets = storage.snippets().list_all().unwrap_or_default();

    // 1. 注入文件夹分组节点
    for g in &groups {
        let p_id = g.parent_id.as_deref().unwrap_or("root").to_string();

        let child_group_count = groups.iter()
            .filter(|child_g| child_g.parent_id.as_deref() == Some(g.id.as_str()))
            .count();
        let child_snippet_count = snippets.iter()
            .filter(|s| s.parent_group_id.as_deref() == Some(&g.id))
            .count();

        raw_nodes.push(RawSnippetTreeNode {
            id: g.id.clone(),
            name: g.name.clone(),
            parent_id: p_id,
            level: g.level as i32,
            is_group: true,
            is_expanded: g.is_expanded,
            has_children: (child_group_count + child_snippet_count) > 0,
            item_count: (child_group_count + child_snippet_count) as i32,
            language: String::new(),
            auto_execute: false,
            is_favorite: false,
            sort_order: g.sort_order,
        });
    }

    // 2. 注入代码片段实体节点
    for s in &snippets {
        let p_id = s.parent_group_id.as_deref().unwrap_or("root").to_string();
        let level = if p_id == "root" {
            0
        } else {
            groups.iter().find(|g| g.id == p_id).map(|g| g.level as i32 + 1).unwrap_or(0)
        };

        raw_nodes.push(RawSnippetTreeNode {
            id: s.id.clone(),
            name: s.title.clone(),
            parent_id: p_id,
            level,
            is_group: false,
            is_expanded: false,
            has_children: false,
            item_count: 0,
            language: s.language.clone(),
            auto_execute: s.auto_execute,
            is_favorite: s.is_favorite,
            sort_order: s.sort_order,
        });
    }

    sort_snippet_tree_hierarchy(&mut raw_nodes);
    raw_nodes
}

/// 对多层嵌套树进行深度优先遍历 (DFS) 排序：分组在前，星标片段靠前
pub fn sort_snippet_tree_hierarchy(tree: &mut Vec<RawSnippetTreeNode>) {
    fn collect_children(
        parent_id: &str,
        tree: &[RawSnippetTreeNode],
        result: &mut Vec<RawSnippetTreeNode>,
    ) {
        let mut children: Vec<RawSnippetTreeNode> = tree
            .iter()
            .filter(|n| n.parent_id == parent_id)
            .cloned()
            .collect();

        // 排序规则: 分组在前(true > false)；同类下星标靠前(true > false)；再按 sort_order 升序，最后按名称升序
        children.sort_by(|a, b| {
            b.is_group.cmp(&a.is_group)
                .then_with(|| b.is_favorite.cmp(&a.is_favorite))
                .then_with(|| a.sort_order.cmp(&b.sort_order))
                .then_with(|| a.name.cmp(&b.name))
        });

        for child in children {
            let child_id = child.id.clone();
            let is_grp = child.is_group;
            result.push(child);
            if is_grp {
                collect_children(&child_id, tree, result);
            }
        }
    }

    let mut sorted = Vec::with_capacity(tree.len());
    collect_children("root", tree, &mut sorted);

    // 容错: 如果有孤立节点，追加到最后
    for node in tree.iter() {
        if !sorted.iter().any(|s| s.id == node.id) {
            sorted.push(node.clone());
        }
    }

    *tree = sorted;
}

/// 移动与调序代码片段树形节点（片段或文件夹分组）。
///
/// 严格保证代码片段层级树的拓扑一致性，自动阻止自身移入自身或将父文件夹移入其子孙节点的环路行为。
/// 若移动的是分组文件夹，会自动递归迁移其下属整棵子树并自动重算所有子节点的深度 `level` 与各分组的 `item_count`。
pub fn move_and_reorder_raw_snippet_node(
    tree: &mut Vec<RawSnippetTreeNode>,
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
        return Err("不能将父文件夹移动至其子孙节点中 (循环引用)".to_string());
    }

    let (new_parent_id, new_level, target_name) = if drop_position == "root" || target_id == "root" || target_id.is_empty() {
        ("root".to_string(), 0, "顶级根目录".to_string())
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
    } else if drop_position == "after" || drop_position == "inside" {
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
    } else {
        remaining_tree.extend(subtree_nodes);
    }

    // 重新统计所有文件夹分组的直属 item_count
    for i in 0..remaining_tree.len() {
        if remaining_tree[i].is_group {
            let grp_id = remaining_tree[i].id.clone();
            let count = remaining_tree.iter().filter(|n| n.parent_id == grp_id).count() as i32;
            remaining_tree[i].item_count = count;
            remaining_tree[i].has_children = count > 0;
        }
    }

    *tree = remaining_tree;
    Ok((source_name, target_name))
}

/// 根据当前已展开的分组 ID 集合，计算并生成给 Slint UI 渲染的可视节点序列
pub fn build_visible_snippet_tree_nodes(
    master_tree: &[RawSnippetTreeNode],
    expanded_groups: &HashSet<String>,
) -> Vec<SnippetTreeNode> {
    let mut visible = Vec::new();

    for node in master_tree {
        // 检查从根节点到该节点的所有祖先是否都处于展开状态
        let mut curr_parent = node.parent_id.as_str();
        let mut is_visible = true;

        while curr_parent != "root" && !curr_parent.is_empty() {
            if !expanded_groups.contains(curr_parent) {
                is_visible = false;
                break;
            }
            // 向上寻找祖先的 parent
            if let Some(parent_node) = master_tree.iter().find(|n| n.id == curr_parent) {
                curr_parent = parent_node.parent_id.as_str();
            } else {
                break;
            }
        }

        if is_visible {
            let is_expanded = if node.is_group {
                expanded_groups.contains(&node.id)
            } else {
                false
            };

            visible.push(SnippetTreeNode {
                id: node.id.clone().into(),
                name: node.name.clone().into(),
                parent_id: node.parent_id.clone().into(),
                level: node.level,
                is_group: node.is_group,
                is_expanded,
                has_children: node.has_children,
                item_count: node.item_count,
                language: node.language.clone().into(),
                auto_execute: node.auto_execute,
                is_favorite: node.is_favorite,
            });
        }
    }

    visible
}

/// 关键字模糊搜索树构建（自动展开命中节点的所有父级祖先）
pub fn build_search_snippet_tree_nodes(
    master_tree: &[RawSnippetTreeNode],
    query: &str,
) -> Vec<SnippetTreeNode> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }

    let mut matched_ids = HashSet::new();
    let mut needed_ancestors = HashSet::new();

    // 1. 查找所有直接命中的节点
    for node in master_tree {
        if node.name.to_lowercase().contains(&q)
            || node.language.to_lowercase().contains(&q)
        {
            matched_ids.insert(node.id.clone());

            // 收集祖先链
            let mut curr = node.parent_id.as_str();
            while curr != "root" && !curr.is_empty() {
                needed_ancestors.insert(curr.to_string());
                if let Some(p) = master_tree.iter().find(|n| n.id == curr) {
                    curr = p.parent_id.as_str();
                } else {
                    break;
                }
            }
        }
    }

    // 2. 生成过滤后的可见列表（按 master_tree 的 DFS 顺序）
    let mut result = Vec::new();
    for node in master_tree {
        if matched_ids.contains(&node.id) || needed_ancestors.contains(&node.id) {
            result.push(SnippetTreeNode {
                id: node.id.clone().into(),
                name: node.name.clone().into(),
                parent_id: node.parent_id.clone().into(),
                level: node.level,
                is_group: node.is_group,
                is_expanded: true, // 搜索模式下分组默认全部展开
                has_children: node.has_children,
                item_count: node.item_count,
                language: node.language.clone().into(),
                auto_execute: node.auto_execute,
                is_favorite: node.is_favorite,
            });
        }
    }

    result
}

/// 构建分组选择下拉列表项数据
pub fn build_snippet_group_options(storage: &dyn AppStorage) -> Vec<GroupOptionData> {
    let mut options = vec![
        GroupOptionData {
            id: "root".into(),
            name: "根目录 (顶级)".into(),
            level: 0,
            parent_id: "".into(),
            has_children: false,
            is_expanded: false,
        }
    ];

    let groups = storage.snippets().list_groups().unwrap_or_default();
    for g in groups {
        let prefix = "  ".repeat(g.level as usize);
        options.push(GroupOptionData {
            id: g.id.clone().into(),
            name: format!("{}📁 {}", prefix, g.name).into(),
            level: g.level as i32 + 1,
            parent_id: g.parent_id.unwrap_or_default().into(),
            has_children: false,
            is_expanded: false,
        });
    }

    options
}

#[cfg(test)]
mod tests {
    use super::*;
    use smagical_core::MockStorage;

    #[test]
    fn test_snippet_tree_model_building_and_expansion() {
        let storage = MockStorage::new_seeded();
        let master = build_raw_snippet_tree_from_storage(&storage);
        assert!(!master.is_empty());

        // 默认展开所有顶级分组
        let mut expanded = HashSet::new();
        expanded.insert("sgrp-docker".to_string());
        expanded.insert("sgrp-ops".to_string());

        let visible = build_visible_snippet_tree_nodes(&master, &expanded);
        assert!(!visible.is_empty());

        // 搜索测试
        let searched = build_search_snippet_tree_nodes(&master, "清理");
        assert_eq!(searched.len(), 2); // 包含 sgrp-docker (祖先) 与 snip-docker-prune (清理悬空镜像与未用卷)
    }

    #[test]
    fn test_move_snippet_node_inside_group_and_to_root() {
        let storage = MockStorage::new_seeded();
        let mut master = build_raw_snippet_tree_from_storage(&storage);

        // 1. 将 snip-docker-ps (原本在 sgrp-docker 下) 移动至 sgrp-k8s 内部
        let res = move_and_reorder_raw_snippet_node(&mut master, "snip-docker-ps", "sgrp-k8s", "inside");
        assert!(res.is_ok());

        let node = master.iter().find(|n| n.id == "snip-docker-ps").unwrap();
        assert_eq!(node.parent_id, "sgrp-k8s");

        // 2. 将 snip-docker-ps 移动至根目录
        let res_root = move_and_reorder_raw_snippet_node(&mut master, "snip-docker-ps", "root", "root");
        assert!(res_root.is_ok());

        let node_root = master.iter().find(|n| n.id == "snip-docker-ps").unwrap();
        assert_eq!(node_root.parent_id, "root");
        assert_eq!(node_root.level, 0);

        // 3. 防呆测试：自身移入自身
        let res_self = move_and_reorder_raw_snippet_node(&mut master, "sgrp-docker", "sgrp-docker", "inside");
        assert!(res_self.is_err());

        // 4. 防呆测试：父分组移入其子孙节点 (循环引用)
        let res_cycle = move_and_reorder_raw_snippet_node(&mut master, "sgrp-ops", "sgrp-ops-net", "inside");
        assert!(res_cycle.is_err());
    }
}
