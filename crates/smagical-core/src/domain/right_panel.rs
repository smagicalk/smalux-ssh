//! 右侧辅助抽屉栏动态注册与面板管理模型。
//!
//! 支持右侧辅助面板（如主机详情、常用脚本片段、SFTP 快速穿透、AI 助手）的声明式注册、动态调序与热插拔挂载。

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// 右侧辅助抽屉栏条目模型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RightPanelItem {
    /// 面板唯一标识 (如 "info", "snippets", "sftp", "ai")
    pub id: String,
    /// 矢量图标名称 (如 "info", "terminal", "folder", "sparkles")
    pub icon_name: String,
    /// 悬停提示文字 (如 "主机与会话详情")
    pub tooltip: String,
    /// 排序权重 (数字越小越靠前，如 10, 20, 30...)
    pub order: i32,
    /// 是否在右侧栏可见启用
    pub is_visible: bool,
    /// 动态角标计数 (0 表示不展示角标)
    pub badge_count: i32,
    /// 可选全局快捷键 (如 "Ctrl+I")
    pub shortcut: Option<String>,
}

impl RightPanelItem {
    /// 构造标准右侧辅助面板项
    pub fn new(
        id: impl Into<String>,
        icon_name: impl Into<String>,
        tooltip: impl Into<String>,
        order: i32,
    ) -> Self {
        Self {
            id: id.into(),
            icon_name: icon_name.into(),
            tooltip: tooltip.into(),
            order,
            is_visible: true,
            badge_count: 0,
            shortcut: None,
        }
    }

    /// 设置快捷键绑定
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }
}

/// 右侧辅助面板注册中心
#[derive(Debug, Clone)]
pub struct RightPanelRegistry {
    items: HashMap<String, RightPanelItem>,
    active_panel_id: Option<String>,
    is_drawer_open: bool,
    drawer_width: u32,
}

impl Default for RightPanelRegistry {
    fn default() -> Self {
        let mut registry = Self {
            items: HashMap::new(),
            active_panel_id: Some("info".into()),
            is_drawer_open: false,
            drawer_width: 320,
        };

        // 预置默认右侧伴生面板 (标准权重顺序)
        registry.register(RightPanelItem::new("info", "info", "主机与会话详情", 10).with_shortcut("Ctrl+Shift+I"));
        registry.register(RightPanelItem::new("tunnel", "tunnel", "主机专属端口转发与隧道", 15).with_shortcut("Ctrl+Shift+T"));
        registry.register(RightPanelItem::new("snippets", "terminal", "关联常用脚本与片段", 20).with_shortcut("Ctrl+Shift+S"));
        registry.register(RightPanelItem::new("sftp", "folder", "远端文件快速传输", 30).with_shortcut("Ctrl+Shift+F"));
        registry.register(RightPanelItem::new("ai", "sparkles", "AI 终端智能助手", 40).with_shortcut("Ctrl+Shift+A"));

        registry
    }
}

impl RightPanelRegistry {
    /// 创建空的注册中心
    pub fn empty() -> Self {
        Self {
            items: HashMap::new(),
            active_panel_id: None,
            is_drawer_open: false,
            drawer_width: 320,
        }
    }

    /// 动态注册新面板
    pub fn register(&mut self, item: RightPanelItem) {
        self.items.insert(item.id.clone(), item);
    }

    /// 动态注销面板
    pub fn unregister(&mut self, id: &str) -> Option<RightPanelItem> {
        if self.active_panel_id.as_deref() == Some(id) {
            self.active_panel_id = None;
            self.is_drawer_open = false;
        }
        self.items.remove(id)
    }

    /// 动态修改面板排序权重
    pub fn set_order(&mut self, id: &str, order: i32) -> bool {
        if let Some(item) = self.items.get_mut(id) {
            item.order = order;
            true
        } else {
            false
        }
    }

    /// 动态设置面板显隐状态
    pub fn set_visible(&mut self, id: &str, is_visible: bool) -> bool {
        if let Some(item) = self.items.get_mut(id) {
            item.is_visible = is_visible;
            if !is_visible && self.active_panel_id.as_deref() == Some(id) {
                self.is_drawer_open = false;
            }
            true
        } else {
            false
        }
    }

    /// 获取所有可见面板列表 (严格按 order 升序排列)
    pub fn list_visible(&self) -> Vec<RightPanelItem> {
        let mut list: Vec<RightPanelItem> = self
            .items
            .values()
            .filter(|item| item.is_visible)
            .cloned()
            .collect();
        list.sort_by_key(|item| item.order);
        list
    }

    /// 获取当前激活展开的面板 ID
    pub fn active_panel_id(&self) -> Option<&str> {
        self.active_panel_id.as_deref()
    }

    /// 设置当前激活的面板 ID
    pub fn set_active_panel_id(&mut self, id: Option<String>) {
        self.active_panel_id = id;
    }

    /// 抽屉是否展开
    pub fn is_drawer_open(&self) -> bool {
        self.is_drawer_open
    }

    /// 设置抽屉展开/折叠状态
    pub fn set_drawer_open(&mut self, open: bool) {
        self.is_drawer_open = open;
    }

    /// 切换某个面板的展开状态 (若点击相同面板则折叠，若点击不同面板则切入展开)
    pub fn toggle_panel(&mut self, id: &str) -> bool {
        if self.is_drawer_open && self.active_panel_id.as_deref() == Some(id) {
            self.is_drawer_open = false;
            false
        } else {
            self.active_panel_id = Some(id.to_string());
            self.is_drawer_open = true;
            true
        }
    }

    /// 获取抽屉宽度
    pub fn drawer_width(&self) -> u32 {
        self.drawer_width
    }

    /// 设置抽屉宽度
    pub fn set_drawer_width(&mut self, width: u32) {
        self.drawer_width = width.clamp(240, 600);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_right_panel_registry_defaults_and_sorting() {
        let mut reg = RightPanelRegistry::default();
        let list = reg.list_visible();
        assert_eq!(list.len(), 5);
        assert_eq!(list[0].id, "info");
        assert_eq!(list[1].id, "tunnel");
        assert_eq!(list[2].id, "snippets");
        assert_eq!(list[3].id, "sftp");
        assert_eq!(list[4].id, "ai");

        // 调整 ai 排序到第一位
        assert!(reg.set_order("ai", 5));
        let list2 = reg.list_visible();
        assert_eq!(list2[0].id, "ai");
        assert_eq!(list2[1].id, "info");
    }

    #[test]
    fn test_right_panel_toggle() {
        let mut reg = RightPanelRegistry::default();
        assert!(!reg.is_drawer_open());

        // 首次点击展开
        let opened = reg.toggle_panel("info");
        assert!(opened);
        assert!(reg.is_drawer_open());
        assert_eq!(reg.active_panel_id(), Some("info"));

        // 再次点击相同面板则折叠
        let opened_again = reg.toggle_panel("info");
        assert!(!opened_again);
        assert!(!reg.is_drawer_open());

        // 点击不同面板则展开新面板
        let opened_diff = reg.toggle_panel("ai");
        assert!(opened_diff);
        assert!(reg.is_drawer_open());
        assert_eq!(reg.active_panel_id(), Some("ai"));
    }
}
