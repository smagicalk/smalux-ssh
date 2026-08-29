//! 批量数据生成与批量修改引擎 (Batch Generator & Modifier)

use crate::models::{DebugHostCard, DebugRawNode};

/// 批量生成主机参数模型
#[derive(Clone, Debug)]
pub struct BatchGenerateConfig {
    /// 主机名称前缀 (例如: "node-", "k8s-worker-", "db-slave-")
    pub name_prefix: String,
    /// 生成主机数量 (如: 10, 20, 50, 100)
    pub count: usize,
    /// 起始序号 (例如: 1, 101)
    pub start_index: usize,
    /// IP 网段前缀 (例如: "192.168.1.", "10.0.0.")
    pub ip_prefix: String,
    /// IP 起始末位 (例如: 10)
    pub start_ip: usize,
    /// SSH 端口
    pub port: i32,
    /// 归属分组名称 (支持路径嵌套格式，如 "集群/k8s" 或 "亚太/中国区/杭州")
    pub group_name: String,
    /// 状态生成策略 ("online", "offline", "warning", "random", "mixed")
    pub status_mode: String,
}

impl Default for BatchGenerateConfig {
    fn default() -> Self {
        Self {
            name_prefix: "k8s-node-".to_string(),
            count: 10,
            start_index: 1,
            ip_prefix: "192.168.1.".to_string(),
            start_ip: 10,
            port: 22,
            group_name: "集群/k8s".to_string(),
            status_mode: "random".to_string(),
        }
    }
}

/// 解析路径（如 "集群/k8s" 或 "亚太/中国区/杭州/POD-01"）并在树中逐级确保嵌套分组节点存在
///
/// 返回 (叶子分组 ID, 叶子分组深度层级, 叶子分组展示名称)
pub fn ensure_group_hierarchy(tree: &mut Vec<DebugRawNode>, path: &str) -> (String, i32, String) {
    let clean_path = path.replace('\\', "/");
    let segments: Vec<&str> = clean_path
        .split('/')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if segments.is_empty() {
        let default_id = "grp-default".to_string();
        if !tree.iter().any(|n| n.id == default_id) {
            tree.push(DebugRawNode {
                id: default_id.clone(),
                name: "默认分组".to_string(),
                is_group: true,
                parent_id: "".to_string(),
                level: 0,
                address: "".to_string(),
                port: 0,
                status: "online".to_string(),
                ping_ms: 0,
                item_count: 0,
            });
        }
        return (default_id, 0, "默认分组".to_string());
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
            tree.push(DebugRawNode {
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

/// 批量生成主机及所属嵌套分组树节点与卡片数据
pub fn generate_batch_hosts(config: &BatchGenerateConfig) -> (Vec<DebugRawNode>, Vec<DebugHostCard>) {
    let mut tree = Vec::new();
    let mut cards = Vec::new();

    // 1. 递归建立层级嵌套分组
    let (leaf_group_id, leaf_level, leaf_name) = ensure_group_hierarchy(&mut tree, &config.group_name);

    // 2. 循环生成各主机节点
    for i in 0..config.count {
        let seq = config.start_index + i;
        let host_id = format!("batch-host-{}", seq);
        let host_name = format!("{}{:02}", config.name_prefix, seq);
        
        let ip_suffix = config.start_ip + i;
        let ip_addr = format!("{}{}", config.ip_prefix, ip_suffix);

        let (status, ping_ms) = match config.status_mode.as_str() {
            "online" => ("online".to_string(), 15 + (i as i32 % 25)),
            "offline" => ("offline".to_string(), 0),
            "warning" => ("warning".to_string(), 120 + (i as i32 * 5)),
            "random" | "mixed" => {
                let rem = i % 5;
                if rem == 0 {
                    ("warning".to_string(), 145)
                } else if rem == 1 {
                    ("offline".to_string(), 0)
                } else {
                    ("online".to_string(), 12 + (i as i32 % 30))
                }
            }
            _ => ("online".to_string(), 20),
        };

        // 树节点 (归属于叶子分组，深度为 leaf_level + 1)
        tree.push(DebugRawNode {
            id: host_id.clone(),
            name: host_name.clone(),
            is_group: false,
            parent_id: leaf_group_id.clone(),
            level: leaf_level + 1,
            address: ip_addr.clone(),
            port: config.port,
            status: status.clone(),
            ping_ms,
            item_count: 0,
        });

        // 卡片模型
        cards.push(DebugHostCard {
            id: host_id,
            name: host_name,
            address: ip_addr,
            port: config.port,
            group: leaf_name.clone(),
            status,
            ping_ms,
        });
    }

    (tree, cards)
}

/// 批量修改状态 ("all_online", "all_offline", "all_warning", "random")
pub fn batch_update_status(
    tree: &mut [DebugRawNode],
    cards: &mut [DebugHostCard],
    status_mode: &str,
) {
    for (i, node) in tree.iter_mut().enumerate() {
        if !node.is_group {
            let (st, ping) = match status_mode {
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
            node.status = st.to_string();
            node.ping_ms = ping;
        }
    }

    for (i, card) in cards.iter_mut().enumerate() {
        let (st, ping) = match status_mode {
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
        card.status = st.to_string();
        card.ping_ms = ping;
    }
}

/// 批量修改端口
pub fn batch_update_port(
    tree: &mut [DebugRawNode],
    cards: &mut [DebugHostCard],
    new_port: i32,
) {
    for node in tree.iter_mut() {
        if !node.is_group {
            node.port = new_port;
        }
    }
    for card in cards.iter_mut() {
        card.port = new_port;
    }
}
