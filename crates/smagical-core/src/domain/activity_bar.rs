//! 侧边栏动态注册与菜单项领域模型。

use std::sync::RwLock;

/// 侧边栏菜单项配置与状态模型。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActivityBarItem {
    /// 唯一标识 (如 "hosts", "keys", "history", "snippets", "settings", "debug")
    pub id: String,
    /// 矢量图标名称 (如 "server", "key", "clock", "terminal", "settings", "bug")
    pub icon_name: String,
    /// 悬浮与辅助提示文案 (如 "主机管理", "SSH 密钥")
    pub tooltip: String,
    /// 排序权重 (数值越小越靠前/靠上，默认 0)
    pub order: i32,
    /// 是否吸底展示 (true 为底部系统菜单如设置/调试，false 为顶部主要功能)
    pub is_bottom: bool,
    /// 是否可见 (可用于根据配置开关动态隐藏/显示)
    pub is_visible: bool,
    /// 徽标角标计数 (0 表示不展示徽标)
    pub badge_count: i32,
    /// 快捷键提示说明 (如 "Ctrl+1", "F12")
    pub shortcut: Option<String>,
}

impl ActivityBarItem {
    /// 创建一个新的顶部主功能菜单项。
    pub fn top(
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
            is_bottom: false,
            is_visible: true,
            badge_count: 0,
            shortcut: None,
        }
    }

    /// 创建一个底部系统/配置菜单项。
    pub fn bottom(
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
            is_bottom: true,
            is_visible: true,
            badge_count: 0,
            shortcut: None,
        }
    }

    /// 设置快捷键提示。
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// 设置初始可见性。
    pub fn with_visibility(mut self, is_visible: bool) -> Self {
        self.is_visible = is_visible;
        self
    }
}

/// 侧边栏动态注册管理器。
#[derive(Debug)]
pub struct ActivityBarRegistry {
    items: RwLock<Vec<ActivityBarItem>>,
}

impl Default for ActivityBarRegistry {
    fn default() -> Self {
        Self::new_with_defaults()
    }
}

impl ActivityBarRegistry {
    /// 创建一个空的注册管理器。
    pub fn new() -> Self {
        Self {
            items: RwLock::new(Vec::new()),
        }
    }

    /// 创建预置默认核心菜单项的注册管理器。
    pub fn new_with_defaults() -> Self {
        let registry = Self::new();
        // 顶部核心业务功能 (严格按序: 主机 -> SFTP文件 -> 密钥 -> 代码片段 -> 隧道代理 -> 历史会话)
        registry.register(ActivityBarItem::top("hosts", "server", "主机资产管理", 10).with_shortcut("Ctrl+1"));
        registry.register(ActivityBarItem::top("files", "folder", "SFTP 文件管理", 20).with_shortcut("Ctrl+2"));
        registry.register(ActivityBarItem::top("keys", "key", "SSH 密钥保险箱", 30).with_shortcut("Ctrl+3"));
        registry.register(ActivityBarItem::top("snippets", "terminal", "常用脚本与代码段", 40).with_shortcut("Ctrl+4"));
        registry.register(ActivityBarItem::top("tunnels", "tunnel", "网络隧道代理", 50).with_shortcut("Ctrl+5"));
        registry.register(ActivityBarItem::top("history", "clock", "历史会话与审计", 60).with_shortcut("Ctrl+6"));

        // 底部系统级功能
        registry.register(ActivityBarItem::bottom("settings", "settings", "偏好设置", 90).with_shortcut("Ctrl+,"));
        registry.register(ActivityBarItem::bottom("debug", "bug", "开发者调试控制台", 100).with_shortcut("F12"));
        registry
    }


    /// 动态注册或覆盖一个菜单项。
    pub fn register(&self, item: ActivityBarItem) {
        let mut items = self.items.write().unwrap();
        if let Some(pos) = items.iter().position(|i| i.id == item.id) {
            items[pos] = item;
        } else {
            items.push(item);
        }
    }

    /// 动态卸载或移除一个菜单项。
    pub fn unregister(&self, id: &str) -> bool {
        let mut items = self.items.write().unwrap();
        if let Some(pos) = items.iter().position(|i| i.id == id) {
            items.remove(pos);
            true
        } else {
            false
        }
    }

    /// 动态设置指定菜单项的显隐状态。
    pub fn set_visible(&self, id: &str, is_visible: bool) -> bool {
        let mut items = self.items.write().unwrap();
        if let Some(item) = items.iter_mut().find(|i| i.id == id) {
            item.is_visible = is_visible;
            true
        } else {
            false
        }
    }

    /// 动态修改指定菜单项的排序权重。
    pub fn set_order(&self, id: &str, order: i32) -> bool {
        let mut items = self.items.write().unwrap();
        if let Some(item) = items.iter_mut().find(|i| i.id == id) {
            item.order = order;
            true
        } else {
            false
        }
    }

    /// 动态设置角标数值。
    pub fn set_badge(&self, id: &str, count: i32) -> bool {

        let mut items = self.items.write().unwrap();
        if let Some(item) = items.iter_mut().find(|i| i.id == id) {
            item.badge_count = count;
            true
        } else {
            false
        }
    }

    /// 获取所有可见的顶部主功能菜单项（按 order 升序排序）。
    pub fn list_top_items(&self) -> Vec<ActivityBarItem> {
        let items = self.items.read().unwrap();
        let mut tops: Vec<ActivityBarItem> = items
            .iter()
            .filter(|i| !i.is_bottom && i.is_visible)
            .cloned()
            .collect();
        tops.sort_by_key(|i| i.order);
        tops
    }

    /// 获取所有可见的底部系统菜单项（按 order 升序排序）。
    pub fn list_bottom_items(&self) -> Vec<ActivityBarItem> {
        let items = self.items.read().unwrap();
        let mut bottoms: Vec<ActivityBarItem> = items
            .iter()
            .filter(|i| i.is_bottom && i.is_visible)
            .cloned()
            .collect();
        bottoms.sort_by_key(|i| i.order);
        bottoms
    }

    /// 获取所有已注册的菜单项（包含隐藏项）。
    pub fn list_all(&self) -> Vec<ActivityBarItem> {
        self.items.read().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_bar_registry_defaults_and_filtering() {
        let registry = ActivityBarRegistry::new_with_defaults();
        let tops = registry.list_top_items();
        let bottoms = registry.list_bottom_items();

        assert_eq!(tops.len(), 6);
        assert_eq!(tops[0].id, "hosts");
        assert_eq!(tops[1].id, "files");
        assert_eq!(tops[2].id, "keys");
        assert_eq!(tops[3].id, "snippets");
        assert_eq!(tops[4].id, "tunnels");
        assert_eq!(tops[5].id, "history");

        assert_eq!(bottoms.len(), 2);
        assert_eq!(bottoms[0].id, "settings");
        assert_eq!(bottoms[1].id, "debug");

        // 隐藏 debug
        registry.set_visible("debug", false);
        assert_eq!(registry.list_bottom_items().len(), 1);

        // 动态注册新插件 (如 AI 助手)
        registry.register(ActivityBarItem::top("ai_assistant", "sparkles", "AI 终端助手", 25));
        let tops_updated = registry.list_top_items();
        assert_eq!(tops_updated.len(), 7);
        assert_eq!(tops_updated[2].id, "ai_assistant"); // order 25 插入在 files (20) 和 keys (30) 之间
    }

}

