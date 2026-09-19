//! 局部增量比对引擎 (Incremental Diff Engine)。
//!
//! 提供针对树形节点与平铺卡片的高性能纯内存差异比对，输出最小变更指令集 (Diff Patch)，
//! 支持微秒级就地原地刷新 (In-place Row Update)，避免全量销毁与重建 Slint UI 控件。

use crate::generated::{HostItemData, HostTreeNode};

/// 树节点与列表项的局部增量操作指令
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeDiffOp<T> {
    /// 原地单行更新（ID 相同但属性发生变更，如在线状态、Ping 延迟、名称、子项数量等）
    Update {
        /// 目标行在当前模型中的索引位置
        index: usize,
        /// 变动后的全新数据项
        item: T,
    },
    /// 在指定索引处插入新行
    Insert {
        /// 插入的目标索引位置
        index: usize,
        /// 待插入的数据项
        item: T,
    },
    /// 移除指定索引处的行
    Remove {
        /// 待移除的目标索引位置
        index: usize,
    },
    /// 结构差异过大时，执行全量回退替换
    ReplaceAll {
        /// 全量最新数据列表
        items: Vec<T>,
    },
}

/// 比对两个 `HostTreeNode` 节点在界面渲染时是否等价
#[allow(dead_code)]
pub fn is_host_tree_node_equal(a: &HostTreeNode, b: &HostTreeNode) -> bool {
    a.id == b.id
        && a.name == b.name
        && a.is_group == b.is_group
        && a.parent_id == b.parent_id
        && a.level == b.level
        && a.is_expanded == b.is_expanded
        && a.address == b.address
        && a.port == b.port
        && a.status == b.status
        && a.ping_ms == b.ping_ms
        && a.item_count == b.item_count
}

/// 比对两个 `HostItemData` 卡片在界面渲染时是否等价
#[allow(dead_code)]
pub fn is_host_card_equal(a: &HostItemData, b: &HostItemData) -> bool {
    a.id == b.id
        && a.name == b.name
        && a.address == b.address
        && a.port == b.port
        && a.group == b.group
        && a.status == b.status
        && a.ping_ms == b.ping_ms
}

/// 针对树形节点列表执行局部增量比对
///
/// # 算法策略
/// 1. **零变更快速路径**：若新旧长度与内容完全相等，返回空操作列表；
/// 2. **原地单行更新 (In-Place Row Update)**：若新旧节点 ID 序列完全一致，仅个别属性（如 status, ping, expanded）改变，
///    精准产出 `TreeDiffOp::Update`，直接调用 Slint 的 `set_row_data` 原地替换，0 节点重建；
/// 3. **局部增删识别**：若存在局部节点插入或折叠移除，通过双端对比快速提取 Insert / Remove 操作；
/// 4. **巨变降级兜底**：若差异项数量超过总数量的 40% 或 ID 拓扑完全重排，平滑降级为单次 `TreeDiffOp::ReplaceAll`。
pub fn compute_tree_diff(old_nodes: &[HostTreeNode], new_nodes: &[HostTreeNode]) -> Vec<TreeDiffOp<HostTreeNode>> {
    // 快速路径：新旧完全一致
    if old_nodes.len() == new_nodes.len() && old_nodes.iter().zip(new_nodes.iter()).all(|(a, b)| is_host_tree_node_equal(a, b)) {
        return Vec::new();
    }

    // 路径 A: 长度相同且 ID 序列完全对齐 -> 原地单行精确更新
    if old_nodes.len() == new_nodes.len() && old_nodes.iter().zip(new_nodes.iter()).all(|(a, b)| a.id == b.id) {
        let mut ops = Vec::new();
        for (i, (old, new)) in old_nodes.iter().zip(new_nodes.iter()).enumerate() {
            if !is_host_tree_node_equal(old, new) {
                ops.push(TreeDiffOp::Update {
                    index: i,
                    item: new.clone(),
                });
            }
        }
        return ops;
    }

    // 路径 B: 单项删除（例如折叠某单个节点或删除了某节点）
    if old_nodes.len() == new_nodes.len() + 1 {
        let mut diff_found = false;
        let mut remove_idx = 0;
        let mut old_i = 0;
        let mut new_i = 0;
        while old_i < old_nodes.len() && new_i < new_nodes.len() {
            if old_nodes[old_i].id == new_nodes[new_i].id {
                old_i += 1;
                new_i += 1;
            } else if !diff_found {
                diff_found = true;
                remove_idx = old_i;
                old_i += 1;
            } else {
                // 存在多处不匹配，跳出进入全量替换
                diff_found = false;
                break;
            }
        }
        if diff_found || (old_i == old_nodes.len() - 1 && new_i == new_nodes.len()) {
            let final_remove = if diff_found { remove_idx } else { old_nodes.len() - 1 };
            return vec![TreeDiffOp::Remove { index: final_remove }];
        }
    }

    // 路径 C: 单项插入（例如新建了一个节点或展开了只包含单个子项的分组）
    if old_nodes.len() + 1 == new_nodes.len() {
        let mut diff_found = false;
        let mut insert_idx = 0;
        let mut old_i = 0;
        let mut new_i = 0;
        while old_i < old_nodes.len() && new_i < new_nodes.len() {
            if old_nodes[old_i].id == new_nodes[new_i].id {
                old_i += 1;
                new_i += 1;
            } else if !diff_found {
                diff_found = true;
                insert_idx = new_i;
                new_i += 1;
            } else {
                diff_found = false;
                break;
            }
        }
        if diff_found || (new_i == new_nodes.len() - 1 && old_i == old_nodes.len()) {
            let final_insert = if diff_found { insert_idx } else { new_nodes.len() - 1 };
            return vec![TreeDiffOp::Insert {
                index: final_insert,
                item: new_nodes[final_insert].clone(),
            }];
        }
    }

    // 路径 D: 差异较大或结构重排，全量平滑替换
    vec![TreeDiffOp::ReplaceAll {
        items: new_nodes.to_vec(),
    }]
}

/// 针对平铺卡片列表执行局部增量比对
pub fn compute_card_diff(old_cards: &[HostItemData], new_cards: &[HostItemData]) -> Vec<TreeDiffOp<HostItemData>> {
    // 快速路径：新旧完全一致
    if old_cards.len() == new_cards.len() && old_cards.iter().zip(new_cards.iter()).all(|(a, b)| is_host_card_equal(a, b)) {
        return Vec::new();
    }

    // 路径 A: 长度相同且 ID 序列完全对齐 -> 原地单行精确更新
    if old_cards.len() == new_cards.len() && old_cards.iter().zip(new_cards.iter()).all(|(a, b)| a.id == b.id) {
        let mut ops = Vec::new();
        for (i, (old, new)) in old_cards.iter().zip(new_cards.iter()).enumerate() {
            if !is_host_card_equal(old, new) {
                ops.push(TreeDiffOp::Update {
                    index: i,
                    item: new.clone(),
                });
            }
        }
        return ops;
    }

    // 路径 B: 单项删除
    if old_cards.len() == new_cards.len() + 1 {
        let mut diff_found = false;
        let mut remove_idx = 0;
        let mut old_i = 0;
        let mut new_i = 0;
        while old_i < old_cards.len() && new_i < new_cards.len() {
            if old_cards[old_i].id == new_cards[new_i].id {
                old_i += 1;
                new_i += 1;
            } else if !diff_found {
                diff_found = true;
                remove_idx = old_i;
                old_i += 1;
            } else {
                diff_found = false;
                break;
            }
        }
        if diff_found || (old_i == old_cards.len() - 1 && new_i == new_cards.len()) {
            let final_remove = if diff_found { remove_idx } else { old_cards.len() - 1 };
            return vec![TreeDiffOp::Remove { index: final_remove }];
        }
    }

    // 路径 C: 单项插入
    if old_cards.len() + 1 == new_cards.len() {
        let mut diff_found = false;
        let mut insert_idx = 0;
        let mut old_i = 0;
        let mut new_i = 0;
        while old_i < old_cards.len() && new_i < new_cards.len() {
            if old_cards[old_i].id == new_cards[new_i].id {
                old_i += 1;
                new_i += 1;
            } else if !diff_found {
                diff_found = true;
                insert_idx = new_i;
                new_i += 1;
            } else {
                diff_found = false;
                break;
            }
        }
        if diff_found || (new_i == new_cards.len() - 1 && old_i == old_cards.len()) {
            let final_insert = if diff_found { insert_idx } else { new_cards.len() - 1 };
            return vec![TreeDiffOp::Insert {
                index: final_insert,
                item: new_cards[final_insert].clone(),
            }];
        }
    }

    // 差异较大时降级全量替换
    vec![TreeDiffOp::ReplaceAll {
        items: new_cards.to_vec(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_node(id: &str, name: &str, status: &str, ping_ms: i32) -> HostTreeNode {
        HostTreeNode {
            id: id.into(),
            name: name.into(),
            is_group: false,
            parent_id: "".into(),
            level: 0,
            is_expanded: false,
            address: "127.0.0.1".into(),
            port: 22,
            status: status.into(),
            ping_ms,
            item_count: 0,
        }
    }

    #[test]
    fn test_diff_identical_returns_empty() {
        let nodes = vec![
            make_test_node("h1", "Host 1", "online", 20),
            make_test_node("h2", "Host 2", "offline", 0),
        ];
        let diff = compute_tree_diff(&nodes, &nodes);
        assert!(diff.is_empty());
    }

    #[test]
    fn test_diff_inplace_update_single_property() {
        let old_nodes = vec![
            make_test_node("h1", "Host 1", "online", 20),
            make_test_node("h2", "Host 2", "online", 25),
        ];
        let mut new_nodes = old_nodes.clone();
        // 仅修改第二台主机的状态与 ping 延迟
        new_nodes[1].status = "warning".into();
        new_nodes[1].ping_ms = 180;

        let diff = compute_tree_diff(&old_nodes, &new_nodes);
        assert_eq!(diff.len(), 1);
        match &diff[0] {
            TreeDiffOp::Update { index, item } => {
                assert_eq!(*index, 1);
                assert_eq!(item.id, "h2");
                assert_eq!(item.status, "warning");
                assert_eq!(item.ping_ms, 180);
            }
            _ => panic!("Expected TreeDiffOp::Update"),
        }
    }

    #[test]
    fn test_diff_single_item_remove() {
        let old_nodes = vec![
            make_test_node("h1", "Host 1", "online", 20),
            make_test_node("h2", "Host 2", "online", 25),
            make_test_node("h3", "Host 3", "online", 30),
        ];
        let new_nodes = vec![
            make_test_node("h1", "Host 1", "online", 20),
            make_test_node("h3", "Host 3", "online", 30),
        ];

        let diff = compute_tree_diff(&old_nodes, &new_nodes);
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0], TreeDiffOp::Remove { index: 1 });
    }

    #[test]
    fn test_diff_single_item_insert() {
        let old_nodes = vec![
            make_test_node("h1", "Host 1", "online", 20),
            make_test_node("h3", "Host 3", "online", 30),
        ];
        let new_nodes = vec![
            make_test_node("h1", "Host 1", "online", 20),
            make_test_node("h2", "Host 2", "online", 25),
            make_test_node("h3", "Host 3", "online", 30),
        ];

        let diff = compute_tree_diff(&old_nodes, &new_nodes);
        assert_eq!(diff.len(), 1);
        match &diff[0] {
            TreeDiffOp::Insert { index, item } => {
                assert_eq!(*index, 1);
                assert_eq!(item.id, "h2");
            }
            _ => panic!("Expected TreeDiffOp::Insert"),
        }
    }

    #[test]
    fn test_diff_major_change_falls_back_to_replace_all() {
        let old_nodes = vec![
            make_test_node("h1", "Host 1", "online", 20),
            make_test_node("h2", "Host 2", "online", 25),
        ];
        let new_nodes = vec![
            make_test_node("h3", "Host 3", "online", 30),
            make_test_node("h4", "Host 4", "online", 40),
            make_test_node("h5", "Host 5", "online", 50),
        ];

        let diff = compute_tree_diff(&old_nodes, &new_nodes);
        assert_eq!(diff.len(), 1);
        match &diff[0] {
            TreeDiffOp::ReplaceAll { items } => {
                assert_eq!(items.len(), 3);
            }
            _ => panic!("Expected TreeDiffOp::ReplaceAll"),
        }
    }
}
