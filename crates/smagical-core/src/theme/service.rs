use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use super::model::{
    ResolvedTerminalTheme, ResolvedUiTheme, THEME_SCHEMA_VERSION, TerminalThemeDefinition,
    TerminalThemeTokens, TerminalThemeTokensPatch, ThemeError, ThemeId, ThemeKind, ThemeMetadata,
    ThemePeriod, ThemeWarning, UiThemeDefinition, UiThemeMetrics, UiThemeTokens,
};
use super::validation::{validate_terminal, validate_ui, warnings_for_ui};

/// Windows Terminal 导入不会直接保存，调用方可先预览和处理冲突。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalImport {
    /// 已解析且校验通过、尚未写入仓储的候选主题。
    Candidates(Vec<TerminalThemeDefinition>),
}

/// 管理内置和自定义主题的无 UI 服务。
#[derive(Debug, Default)]
pub struct ThemeService {
    ui_themes: HashMap<ThemeId, UiThemeDefinition>,
    builtin_ids: HashSet<ThemeId>,
    terminal_themes: HashMap<ThemeId, TerminalThemeDefinition>,
}

impl ThemeService {
    /// 创建不包含任何内置或自定义主题的空服务。
    pub fn new() -> Self {
        Self::default()
    }

    /// 解析 UI 主题 TOML，但不修改服务状态或写入磁盘。
    pub fn import_ui_toml(&self, source: &str) -> Result<UiThemeDefinition, ThemeError> {
        let definition: UiThemeDefinition = toml::from_str(source)?;
        ensure_metadata(&definition.metadata, ThemeKind::Ui)?;
        validate_ui(&definition)?;
        Ok(definition)
    }

    /// 解析 Windows Terminal scheme 对象或含 `schemes` 数组的设置片段。
    pub fn import_windows_terminal_json(&self, source: &str) -> Result<TerminalImport, ThemeError> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Input {
            Collection { schemes: Vec<WindowsTerminalScheme> },
            Single(Box<WindowsTerminalScheme>),
        }

        let input: Input = serde_json::from_str(source)?;
        let schemes = match input {
            Input::Collection { schemes } => schemes,
            Input::Single(scheme) => vec![*scheme],
        };
        let candidates: Vec<_> = schemes
            .into_iter()
            .map(TerminalThemeDefinition::from)
            .collect();
        for candidate in &candidates {
            validate_terminal(candidate)?;
        }
        Ok(TerminalImport::Candidates(candidates))
    }

    /// 按显示名称排序列出全部终端主题。
    pub fn list_terminal(&self) -> Vec<&TerminalThemeDefinition> {
        let mut themes: Vec<_> = self.terminal_themes.values().collect();
        themes.sort_by(|left, right| left.metadata.name.cmp(&right.metadata.name));
        themes
    }

    /// 按显示名称排序列出全部 UI 主题。
    pub fn list_ui(&self) -> Vec<&UiThemeDefinition> {
        let mut themes: Vec<_> = self.ui_themes.values().collect();
        themes.sort_by(|left, right| left.metadata.name.cmp(&right.metadata.name));
        themes
    }

    /// 按白天/黑夜变体列出 UI 主题；`System` 不会出现在任一列表中。
    pub fn list_ui_by_period(&self, period: ThemePeriod) -> Vec<&UiThemeDefinition> {
        let mut themes: Vec<_> = self
            .ui_themes
            .values()
            .filter(|theme| theme.metadata.period == Some(period))
            .collect();
        themes.sort_by(|left, right| left.metadata.name.cmp(&right.metadata.name));
        themes
    }

    /// 按稳定 ID 查找 UI 主题定义。
    pub fn get_ui(&self, id: impl AsRef<str>) -> Option<&UiThemeDefinition> {
        self.ui_themes.get(&ThemeId::new(id.as_ref()))
    }

    /// 按稳定 ID 查找终端主题定义。
    pub fn get_terminal(&self, id: impl AsRef<str>) -> Option<&TerminalThemeDefinition> {
        self.terminal_themes.get(&ThemeId::new(id.as_ref()))
    }

    /// 注册一个只读内置 UI 主题。
    pub fn register_builtin_ui(&mut self, definition: UiThemeDefinition) -> Result<(), ThemeError> {
        let id = definition.metadata.id.clone();
        self.insert_ui(definition)?;
        self.builtin_ids.insert(id);
        Ok(())
    }

    /// 保存一个自定义 UI 主题到当前服务。文件持久化由 repository API 负责。
    pub fn save_ui(&mut self, definition: UiThemeDefinition) -> Result<(), ThemeError> {
        self.insert_ui(definition)
    }

    /// 替换已存在的自定义 UI 主题。内置主题始终只读。
    pub fn replace_ui(&mut self, definition: UiThemeDefinition) -> Result<(), ThemeError> {
        ensure_metadata(&definition.metadata, ThemeKind::Ui)?;
        validate_ui(&definition)?;
        let id = definition.metadata.id.clone();
        if self.builtin_ids.contains(&id) {
            return Err(ThemeError::ReadOnlyBuiltin(id));
        }
        if !self.ui_themes.contains_key(&id) {
            return Err(ThemeError::NotFound(id));
        }
        self.ui_themes.insert(id, definition);
        Ok(())
    }

    /// 从服务中移除自定义 UI 主题。配置引用迁移由调用方负责。
    pub fn remove_ui(&mut self, id: impl AsRef<str>) -> Result<UiThemeDefinition, ThemeError> {
        let id = ThemeId::new(id.as_ref());
        if self.builtin_ids.contains(&id) {
            return Err(ThemeError::ReadOnlyBuiltin(id));
        }
        self.ui_themes.remove(&id).ok_or(ThemeError::NotFound(id))
    }

    /// 解析终端主题 TOML，但不修改服务状态或写入磁盘。
    pub fn import_terminal_toml(
        &self,
        source: &str,
    ) -> Result<TerminalThemeDefinition, ThemeError> {
        let definition: TerminalThemeDefinition = toml::from_str(source)?;
        ensure_metadata(&definition.metadata, ThemeKind::Terminal)?;
        validate_terminal(&definition)?;
        Ok(definition)
    }

    /// 将 UI 主题定义编码为格式化 TOML。
    pub fn export_ui_toml(&self, definition: &UiThemeDefinition) -> Result<String, ThemeError> {
        Ok(toml::to_string_pretty(definition)?)
    }

    /// 将终端主题定义编码为格式化 TOML。
    pub fn export_terminal_toml(
        &self,
        definition: &TerminalThemeDefinition,
    ) -> Result<String, ThemeError> {
        Ok(toml::to_string_pretty(definition)?)
    }

    /// 注册一个只读内置终端主题。
    pub fn register_builtin_terminal(
        &mut self,
        definition: TerminalThemeDefinition,
    ) -> Result<(), ThemeError> {
        let id = definition.metadata.id.clone();
        self.insert_terminal(definition)?;
        self.builtin_ids.insert(id);
        Ok(())
    }

    /// 保存一个自定义终端主题到当前服务。
    pub fn save_terminal(&mut self, definition: TerminalThemeDefinition) -> Result<(), ThemeError> {
        self.insert_terminal(definition)
    }

    /// 替换已存在的自定义终端主题。内置主题始终只读。
    pub fn replace_terminal(
        &mut self,
        definition: TerminalThemeDefinition,
    ) -> Result<(), ThemeError> {
        ensure_metadata(&definition.metadata, ThemeKind::Terminal)?;
        validate_terminal(&definition)?;
        let id = definition.metadata.id.clone();
        if self.builtin_ids.contains(&id) {
            return Err(ThemeError::ReadOnlyBuiltin(id));
        }
        if !self.terminal_themes.contains_key(&id) {
            return Err(ThemeError::NotFound(id));
        }
        self.terminal_themes.insert(id, definition);
        Ok(())
    }

    /// 从服务中移除自定义终端主题。配置引用迁移由调用方负责。
    pub fn remove_terminal(
        &mut self,
        id: impl AsRef<str>,
    ) -> Result<TerminalThemeDefinition, ThemeError> {
        let id = ThemeId::new(id.as_ref());
        if self.builtin_ids.contains(&id) {
            return Err(ThemeError::ReadOnlyBuiltin(id));
        }
        self.terminal_themes
            .remove(&id)
            .ok_or(ThemeError::NotFound(id))
    }

    fn insert_terminal(&mut self, definition: TerminalThemeDefinition) -> Result<(), ThemeError> {
        ensure_metadata(&definition.metadata, ThemeKind::Terminal)?;
        validate_terminal(&definition)?;
        let id = definition.metadata.id.clone();
        if self.terminal_themes.contains_key(&id) {
            return Err(ThemeError::DuplicateId(id));
        }
        self.terminal_themes.insert(id, definition);
        Ok(())
    }

    /// 解析完整终端主题，并按继承顺序合并缺失令牌。
    pub fn resolve_terminal(
        &self,
        id: impl AsRef<str>,
    ) -> Result<ResolvedTerminalTheme, ThemeError> {
        let id = ThemeId::new(id.as_ref());
        let mut chain = Vec::new();
        let mut visiting = HashSet::new();
        self.collect_terminal_chain(&id, &mut visiting, &mut chain)?;

        let mut tokens = TerminalThemeTokens::default();
        for definition in &chain {
            tokens.apply(&definition.terminal);
        }
        let definition = chain.last().expect("已解析的主题链不为空");
        Ok(ResolvedTerminalTheme {
            metadata: definition.metadata.clone(),
            tokens,
        })
    }

    /// 校验 UI 主题定义，并返回不阻止保存的可访问性警告。
    pub fn validate_ui(
        &self,
        definition: &UiThemeDefinition,
    ) -> Result<Vec<ThemeWarning>, ThemeError> {
        validate_ui(definition)?;
        let mut tokens = UiThemeTokens::default();
        tokens.apply(&definition.ui);
        Ok(warnings_for_ui(&tokens))
    }

    /// 校验终端主题定义。成功不会将主题加入服务或写入磁盘。
    pub fn validate_terminal(
        &self,
        definition: &TerminalThemeDefinition,
    ) -> Result<(), ThemeError> {
        ensure_metadata(&definition.metadata, ThemeKind::Terminal)?;
        validate_terminal(definition)
    }

    /// 返回已解析 UI 主题的非阻塞诊断，包含继承后的真实颜色。
    pub fn warnings_for_resolved_ui(
        &self,
        id: impl AsRef<str>,
    ) -> Result<Vec<ThemeWarning>, ThemeError> {
        Ok(warnings_for_ui(&self.resolve_ui(id)?.tokens))
    }

    /// 解析配置中的 UI 主题；主题不存在时回退 Darcula。
    pub fn resolve_ui_or_default(
        &self,
        id: impl AsRef<str>,
    ) -> Result<ResolvedUiTheme, ThemeError> {
        match self.resolve_ui(id) {
            Err(ThemeError::NotFound(_)) => self.resolve_ui("builtin.ui.darcula"),
            result => result,
        }
    }

    /// 解析配置中的终端主题；主题不存在时回退 Darcula。
    pub fn resolve_terminal_or_default(
        &self,
        id: impl AsRef<str>,
    ) -> Result<ResolvedTerminalTheme, ThemeError> {
        match self.resolve_terminal(id) {
            Err(ThemeError::NotFound(_)) => self.resolve_terminal("builtin.terminal.darcula"),
            result => result,
        }
    }

    /// 返回主题 ID 是否属于只读内置主题。
    pub fn is_builtin(&self, id: impl AsRef<str>) -> bool {
        self.builtin_ids.contains(&ThemeId::new(id.as_ref()))
    }

    fn insert_ui(&mut self, definition: UiThemeDefinition) -> Result<(), ThemeError> {
        ensure_metadata(&definition.metadata, ThemeKind::Ui)?;
        validate_ui(&definition)?;
        let id = definition.metadata.id.clone();
        if self.ui_themes.contains_key(&id) {
            return Err(ThemeError::DuplicateId(id));
        }
        self.ui_themes.insert(id, definition);
        Ok(())
    }

    /// 解析完整 UI 主题。根主题未提供的令牌使用稳定的 Darcula 基线值。
    pub fn resolve_ui(&self, id: impl AsRef<str>) -> Result<ResolvedUiTheme, ThemeError> {
        let id = ThemeId::new(id.as_ref());
        let mut chain = Vec::new();
        let mut visiting = HashSet::new();
        self.collect_ui_chain(&id, &mut visiting, &mut chain)?;

        let mut tokens = UiThemeTokens::default();
        let mut metrics = UiThemeMetrics::default();
        for definition in &chain {
            tokens.apply(&definition.ui);
            metrics.apply(&definition.metrics);
        }
        let definition = chain.last().expect("已解析的主题链不为空");
        if let Some(period) = definition.metadata.period {
            let expected = match period {
                ThemePeriod::Day => super::model::ColorScheme::Light,
                ThemePeriod::Night => super::model::ColorScheme::Dark,
            };
            if tokens.color_scheme != expected {
                return Err(ThemeError::PeriodSchemeMismatch {
                    id: definition.metadata.id.clone(),
                    period,
                    color_scheme: tokens.color_scheme,
                });
            }
        }
        Ok(ResolvedUiTheme {
            metadata: definition.metadata.clone(),
            tokens,
            metrics,
        })
    }

    fn collect_ui_chain<'a>(
        &'a self,
        id: &ThemeId,
        visiting: &mut HashSet<ThemeId>,
        chain: &mut Vec<&'a UiThemeDefinition>,
    ) -> Result<(), ThemeError> {
        if !visiting.insert(id.clone()) {
            return Err(ThemeError::InheritanceCycle(id.clone()));
        }
        let definition = self
            .ui_themes
            .get(id)
            .ok_or_else(|| ThemeError::NotFound(id.clone()))?;
        if let Some(base) = &definition.metadata.base {
            self.collect_ui_chain(base, visiting, chain)?;
        }
        visiting.remove(id);
        chain.push(definition);
        Ok(())
    }

    fn collect_terminal_chain<'a>(
        &'a self,
        id: &ThemeId,
        visiting: &mut HashSet<ThemeId>,
        chain: &mut Vec<&'a TerminalThemeDefinition>,
    ) -> Result<(), ThemeError> {
        if !visiting.insert(id.clone()) {
            return Err(ThemeError::InheritanceCycle(id.clone()));
        }
        let definition = self
            .terminal_themes
            .get(id)
            .ok_or_else(|| ThemeError::NotFound(id.clone()))?;
        if let Some(base) = &definition.metadata.base {
            self.collect_terminal_chain(base, visiting, chain)?;
        }
        visiting.remove(id);
        chain.push(definition);
        Ok(())
    }
}

fn ensure_metadata(metadata: &ThemeMetadata, expected: ThemeKind) -> Result<(), ThemeError> {
    if metadata.schema_version != THEME_SCHEMA_VERSION {
        return Err(ThemeError::UnsupportedSchema(metadata.schema_version));
    }
    if metadata.kind != expected {
        return Err(ThemeError::KindMismatch {
            expected,
            actual: metadata.kind,
        });
    }
    if !metadata.id.is_valid() {
        return Err(ThemeError::InvalidId(metadata.id.clone()));
    }
    if metadata.name.trim().is_empty() {
        return Err(ThemeError::EmptyName);
    }
    if expected == ThemeKind::Ui
        && metadata.id.as_ref() != "builtin.ui.system"
        && metadata.period.is_none()
    {
        return Err(ThemeError::MissingPeriod(metadata.id.clone()));
    }
    if expected == ThemeKind::Terminal && metadata.period.is_some() {
        return Err(ThemeError::InvalidPeriod(metadata.id.clone()));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WindowsTerminalScheme {
    name: String,
    background: String,
    foreground: String,
    #[serde(default)]
    cursor_color: Option<String>,
    #[serde(default)]
    selection_background: Option<String>,
    black: String,
    red: String,
    green: String,
    yellow: String,
    blue: String,
    purple: String,
    cyan: String,
    white: String,
    bright_black: String,
    bright_red: String,
    bright_green: String,
    bright_yellow: String,
    bright_blue: String,
    bright_purple: String,
    bright_cyan: String,
    bright_white: String,
}

impl From<WindowsTerminalScheme> for TerminalThemeDefinition {
    fn from(value: WindowsTerminalScheme) -> Self {
        Self {
            metadata: ThemeMetadata {
                schema_version: THEME_SCHEMA_VERSION,
                id: ThemeId::new(uuid::Uuid::new_v4().to_string()),
                name: value.name,
                kind: ThemeKind::Terminal,
                period: None,
                base: None,
                author: None,
                source: Some("windows-terminal-json".into()),
            },
            terminal: TerminalThemeTokensPatch {
                background: Some(value.background),
                foreground: Some(value.foreground),
                cursor_color: value.cursor_color,
                selection_background: value.selection_background,
                black: Some(value.black),
                red: Some(value.red),
                green: Some(value.green),
                yellow: Some(value.yellow),
                blue: Some(value.blue),
                purple: Some(value.purple),
                cyan: Some(value.cyan),
                white: Some(value.white),
                bright_black: Some(value.bright_black),
                bright_red: Some(value.bright_red),
                bright_green: Some(value.bright_green),
                bright_yellow: Some(value.bright_yellow),
                bright_blue: Some(value.bright_blue),
                bright_purple: Some(value.bright_purple),
                bright_cyan: Some(value.bright_cyan),
                bright_white: Some(value.bright_white),
            },
        }
    }
}
