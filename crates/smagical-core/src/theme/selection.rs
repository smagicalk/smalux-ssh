use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ThemeError, ThemeId};

/// 应用主题选择及主机级终端主题覆盖。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ThemeSelectionConfig {
    /// 当前选择的应用 UI 主题 ID。
    pub ui_theme: ThemeId,
    /// 未配置主机级覆盖时使用的终端主题 ID。
    pub default_terminal_theme: ThemeId,
    /// 按主机 UUID 保存的终端主题覆盖。
    #[serde(default)]
    pub host_terminal_themes: HashMap<Uuid, ThemeId>,
}

impl Default for ThemeSelectionConfig {
    fn default() -> Self {
        Self {
            ui_theme: ThemeId::new("builtin.ui.darcula"),
            default_terminal_theme: ThemeId::new("builtin.terminal.darcula"),
            host_terminal_themes: HashMap::new(),
        }
    }
}

impl ThemeSelectionConfig {
    /// 从应用配置 TOML 读取主题选择，不访问主题文件。
    pub fn from_toml(source: &str) -> Result<Self, ThemeError> {
        Ok(toml::from_str(source)?)
    }

    /// 将主题选择编码为可持久化 TOML。
    pub fn to_toml(&self) -> Result<String, ThemeError> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// 返回指定主机最终生效的终端主题 ID。
    pub fn effective_terminal_theme(&self, host: Uuid) -> &ThemeId {
        self.host_terminal_themes
            .get(&host)
            .unwrap_or(&self.default_terminal_theme)
    }

    /// 计算删除主题会影响哪些配置引用，但不修改当前配置。
    pub fn impact_of_delete(&self, theme: &ThemeId) -> ThemeDeleteImpact {
        ThemeDeleteImpact {
            ui_selected: &self.ui_theme == theme,
            terminal_default: &self.default_terminal_theme == theme,
            host_ids: self
                .host_terminal_themes
                .iter()
                .filter_map(|(id, value)| (value == theme).then_some(*id))
                .collect(),
        }
    }

    /// 迁移已删除主题的全部配置引用，并返回迁移前的影响范围。
    pub fn migrate_deleted(
        &mut self,
        theme: &ThemeId,
        ui_fallback: ThemeId,
        terminal_fallback: ThemeId,
    ) -> ThemeDeleteImpact {
        let impact = self.impact_of_delete(theme);
        if impact.ui_selected {
            self.ui_theme = ui_fallback;
        }
        if impact.terminal_default {
            self.default_terminal_theme = terminal_fallback;
        }
        self.host_terminal_themes.retain(|_, value| value != theme);
        impact
    }
}

/// 删除主题前计算出的配置引用影响范围。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeDeleteImpact {
    /// 被删除主题是否正由 UI 使用。
    pub ui_selected: bool,
    /// 被删除主题是否为默认终端主题。
    pub terminal_default: bool,
    /// 使用该终端主题作为覆盖的主机 UUID。
    pub host_ids: Vec<Uuid>,
}
