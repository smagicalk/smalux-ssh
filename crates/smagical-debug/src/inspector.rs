//! GUI 运行时探针与布局测算工具 (Runtime Inspector & Metrics)

use serde::{Deserialize, Serialize};

/// 估算单行文本的呈现宽度 (像素)
///
/// 算法规则：ASCII 字符约 7.5px，中文字符及全角符号约 13px。
pub fn calculate_text_width(text: &str) -> f32 {
    text.chars().map(|c| if c.is_ascii() { 7.5 } else { 13.0 }).sum()
}

/// 计算单个树形节点所需的视口呈现宽度 (像素)
///
/// # 参数
/// * `name` - 节点展示文本
/// * `level` - 树形缩进层级 (0, 1, 2...)
pub fn calculate_node_width(name: &str, level: i32) -> f32 {
    let text_w = calculate_text_width(name);
    // 左边距 (6px + level * 14px) + 折叠箭头与图标区 (~48px) + 文本宽度 + 状态/计数徽章区 (~50px)
    6.0 + (level as f32) * 14.0 + 48.0 + text_w + 50.0
}

/// 运行时诊断状态快照 (Runtime State Snapshot)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DebugRuntimeMetrics {
    /// 全量节点总数
    pub total_nodes: usize,
    /// 当前可见节点数
    pub visible_nodes: usize,
    /// 全量主机卡片数
    pub total_hosts: usize,
    /// 树形视口测算最大宽度 (像素)
    pub max_tree_width: f32,
    /// 活动主题名称
    pub current_theme: String,
    /// 是否为深色模式
    pub is_dark: bool,
}

impl Default for DebugRuntimeMetrics {
    fn default() -> Self {
        Self {
            total_nodes: 0,
            visible_nodes: 0,
            total_hosts: 0,
            max_tree_width: 240.0,
            current_theme: "Darcula".to_string(),
            is_dark: true,
        }
    }
}
