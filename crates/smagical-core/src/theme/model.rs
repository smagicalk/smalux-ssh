use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 当前支持的主题文件格式版本。
pub const THEME_SCHEMA_VERSION: u32 = 1;

/// 稳定主题标识。显示名称改变时，配置中的引用仍保持有效。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ThemeId(pub String);

impl ThemeId {
    /// 从字符串创建主题标识；合法性由导入或保存流程校验。
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// 返回该 ID 是否可安全用于配置引用和主题文件名。
    pub fn is_valid(&self) -> bool {
        !self.0.is_empty()
            && self.0.len() <= 128
            && self
                .0
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    }
}

impl AsRef<str> for ThemeId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ThemeId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// 主题文件的类型。UI 与终端主题不能互相继承。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeKind {
    /// 应用界面主题。
    Ui,
    /// 终端 ANSI 配色主题。
    Terminal,
}

/// UI 主题希望 Slint 标准控件采用的明暗模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColorScheme {
    /// 使用操作系统和 Slint 当前提供的颜色模式。
    System,
    /// 强制使用浅色模式。
    Light,
    /// 强制使用深色模式。
    Dark,
}

/// UI 主题所属的日间/夜间变体。`System` 选择器不属于任一变体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemePeriod {
    /// 适合白天环境的浅色主题。
    Day,
    /// 适合夜间环境的深色主题。
    Night,
}

/// 所有主题共有的元数据。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeMetadata {
    /// 主题文件格式版本。
    #[serde(rename = "schema-version")]
    pub schema_version: u32,
    /// 用于配置引用、继承和文件命名的稳定 ID。
    pub id: ThemeId,
    /// 面向用户显示的主题名称。
    pub name: String,
    /// 主题包含 UI 令牌还是终端令牌。
    pub kind: ThemeKind,
    /// UI 主题所属的日间或夜间分组。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<ThemePeriod>,
    /// 可选的父主题 ID；缺失令牌从父主题继承。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<ThemeId>,
    /// 可选的主题作者或维护组织。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// 可选的原始来源 URL 或来源说明。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// 可继承的 UI 主题颜色覆盖。缺失字段从 `base` 主题继承。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UiThemeTokensPatch {
    /// 标准控件应采用的明暗模式。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_scheme: Option<ColorScheme>,
    /// 应用窗口的基础背景色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_background: Option<String>,
    /// 侧栏、工具栏和面板的背景色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel_background: Option<String>,
    /// 内容表面和弹层的背景色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_background: Option<String>,
    /// 输入框、按钮等控件的背景色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_background: Option<String>,
    /// 主要文字和图标颜色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foreground: Option<String>,
    /// 次要文字和弱化图标颜色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_foreground: Option<String>,
    /// 禁用状态的文字和图标颜色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_foreground: Option<String>,
    /// 品牌强调色和主要操作颜色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    /// 指针悬停状态的背景色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover_background: Option<String>,
    /// 控件按下状态的背景色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pressed_background: Option<String>,
    /// 当前选中项目的背景色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_background: Option<String>,
    /// 当前选中项目的前景色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_foreground: Option<String>,
    /// 普通分隔线和控件边框颜色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    /// 键盘焦点边框颜色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus_border: Option<String>,
    /// 成功状态颜色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<String>,
    /// 警告状态颜色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    /// 错误和危险操作颜色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub danger: Option<String>,
    /// 一般信息状态颜色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub info: Option<String>,
}

/// 可继承的界面尺寸。单位均为逻辑像素。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UiThemeMetricsPatch {
    /// 小型控件的圆角半径，单位为逻辑像素。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius_small: Option<f32>,
    /// 常规控件的圆角半径，单位为逻辑像素。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius_medium: Option<f32>,
    /// 大型容器的圆角半径，单位为逻辑像素。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius_large: Option<f32>,
    /// 紧凑元素间距，单位为逻辑像素。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing_small: Option<f32>,
    /// 常规元素间距，单位为逻辑像素。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing_medium: Option<f32>,
    /// 区块间距，单位为逻辑像素。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing_large: Option<f32>,
    /// 标准边框宽度，单位为逻辑像素。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_width: Option<f32>,
    /// 常规交互控件高度，单位为逻辑像素。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_height: Option<f32>,
    /// 常规图标尺寸，单位为逻辑像素。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_size: Option<f32>,
}

/// 解析后的完整界面尺寸。
#[derive(Debug, Clone, PartialEq)]
pub struct UiThemeMetrics {
    /// 小型控件的圆角半径。
    pub radius_small: f32,
    /// 常规控件的圆角半径。
    pub radius_medium: f32,
    /// 大型容器的圆角半径。
    pub radius_large: f32,
    /// 紧凑元素间距。
    pub spacing_small: f32,
    /// 常规元素间距。
    pub spacing_medium: f32,
    /// 区块间距。
    pub spacing_large: f32,
    /// 标准边框宽度。
    pub border_width: f32,
    /// 常规交互控件高度。
    pub control_height: f32,
    /// 常规图标尺寸。
    pub icon_size: f32,
}

impl Default for UiThemeMetrics {
    fn default() -> Self {
        Self {
            radius_small: 2.0,
            radius_medium: 4.0,
            radius_large: 8.0,
            spacing_small: 4.0,
            spacing_medium: 8.0,
            spacing_large: 16.0,
            border_width: 1.0,
            control_height: 32.0,
            icon_size: 16.0,
        }
    }
}

impl UiThemeMetrics {
    pub(crate) fn apply(&mut self, patch: &UiThemeMetricsPatch) {
        macro_rules! apply {
            ($field:ident) => {
                if let Some(value) = patch.$field {
                    self.$field = value;
                }
            };
        }
        apply!(radius_small);
        apply!(radius_medium);
        apply!(radius_large);
        apply!(spacing_small);
        apply!(spacing_medium);
        apply!(spacing_large);
        apply!(border_width);
        apply!(control_height);
        apply!(icon_size);
    }
}

/// 完整解析后的 UI 主题令牌。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiThemeTokens {
    /// 标准控件采用的明暗模式。
    pub color_scheme: ColorScheme,
    /// 应用窗口的基础背景色。
    pub window_background: String,
    /// 侧栏、工具栏和面板的背景色。
    pub panel_background: String,
    /// 内容表面和弹层的背景色。
    pub surface_background: String,
    /// 输入框、按钮等控件的背景色。
    pub control_background: String,
    /// 主要文字和图标颜色。
    pub foreground: String,
    /// 次要文字和弱化图标颜色。
    pub secondary_foreground: String,
    /// 禁用状态的文字和图标颜色。
    pub disabled_foreground: String,
    /// 品牌强调色和主要操作颜色。
    pub accent: String,
    /// 指针悬停状态的背景色。
    pub hover_background: String,
    /// 控件按下状态的背景色。
    pub pressed_background: String,
    /// 当前选中项目的背景色。
    pub selected_background: String,
    /// 当前选中项目的前景色。
    pub selected_foreground: String,
    /// 普通分隔线和控件边框颜色。
    pub border: String,
    /// 键盘焦点边框颜色。
    pub focus_border: String,
    /// 成功状态颜色。
    pub success: String,
    /// 警告状态颜色。
    pub warning: String,
    /// 错误和危险操作颜色。
    pub danger: String,
    /// 一般信息状态颜色。
    pub info: String,
}

impl Default for UiThemeTokens {
    fn default() -> Self {
        Self {
            color_scheme: ColorScheme::Dark,
            window_background: "#2B2B2B".into(),
            panel_background: "#3C3F41".into(),
            surface_background: "#313335".into(),
            control_background: "#45494A".into(),
            foreground: "#BBBBBB".into(),
            secondary_foreground: "#A0A0A0".into(),
            disabled_foreground: "#777777".into(),
            accent: "#589DF6".into(),
            hover_background: "#4C5052".into(),
            pressed_background: "#5C6164".into(),
            selected_background: "#4B6EAF".into(),
            selected_foreground: "#DEDEDE".into(),
            border: "#515151".into(),
            focus_border: "#466D94".into(),
            success: "#008F50".into(),
            warning: "#AC7920".into(),
            danger: "#E74848".into(),
            info: "#589DF6".into(),
        }
    }
}

impl UiThemeTokens {
    pub(crate) fn apply(&mut self, patch: &UiThemeTokensPatch) {
        macro_rules! apply {
            ($field:ident) => {
                if let Some(value) = &patch.$field {
                    self.$field = value.clone();
                }
            };
        }
        if let Some(value) = patch.color_scheme {
            self.color_scheme = value;
        }
        apply!(window_background);
        apply!(panel_background);
        apply!(surface_background);
        apply!(control_background);
        apply!(foreground);
        apply!(secondary_foreground);
        apply!(disabled_foreground);
        apply!(accent);
        apply!(hover_background);
        apply!(pressed_background);
        apply!(selected_background);
        apply!(selected_foreground);
        apply!(border);
        apply!(focus_border);
        apply!(success);
        apply!(warning);
        apply!(danger);
        apply!(info);
    }
}

/// 可持久化的 UI 主题定义。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiThemeDefinition {
    /// 主题身份、类型、继承和来源信息。
    #[serde(flatten)]
    pub metadata: ThemeMetadata,
    /// 此主题覆盖的 UI 颜色令牌。
    pub ui: UiThemeTokensPatch,
    /// 此主题覆盖的 UI 尺寸令牌。
    #[serde(default)]
    pub metrics: UiThemeMetricsPatch,
}

/// 经过继承和默认值合并后的 UI 主题。
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedUiTheme {
    /// 最终主题自身的元数据。
    pub metadata: ThemeMetadata,
    /// 应用继承链后的完整颜色令牌。
    pub tokens: UiThemeTokens,
    /// 应用继承链后的完整尺寸令牌。
    pub metrics: UiThemeMetrics,
}

/// 非阻塞主题诊断。警告不会阻止主题保存或应用。
#[derive(Debug, Clone, PartialEq)]
pub enum ThemeWarning {
    /// 主要文字和窗口背景未达到 WCAG AA 常规文本对比度。
    LowContrast {
        /// 参与检测的前景色。
        foreground: String,
        /// 参与检测的背景色。
        background: String,
        /// 两个颜色计算得到的对比度。
        ratio: f32,
    },
}

/// 可继承的终端颜色覆盖。缺失字段从 `base` 主题继承。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalThemeTokensPatch {
    /// 终端背景色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    /// 终端默认前景色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foreground: Option<String>,
    /// 光标颜色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_color: Option<String>,
    /// 选区背景色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_background: Option<String>,
    /// ANSI 黑色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub black: Option<String>,
    /// ANSI 红色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub red: Option<String>,
    /// ANSI 绿色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub green: Option<String>,
    /// ANSI 黄色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yellow: Option<String>,
    /// ANSI 蓝色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blue: Option<String>,
    /// ANSI 紫色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purple: Option<String>,
    /// ANSI 青色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cyan: Option<String>,
    /// ANSI 白色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub white: Option<String>,
    /// ANSI 高亮黑色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bright_black: Option<String>,
    /// ANSI 高亮红色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bright_red: Option<String>,
    /// ANSI 高亮绿色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bright_green: Option<String>,
    /// ANSI 高亮黄色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bright_yellow: Option<String>,
    /// ANSI 高亮蓝色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bright_blue: Option<String>,
    /// ANSI 高亮紫色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bright_purple: Option<String>,
    /// ANSI 高亮青色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bright_cyan: Option<String>,
    /// ANSI 高亮白色。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bright_white: Option<String>,
}

/// 解析后的完整终端背景、前景、光标、选区及 ANSI 16 色。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalThemeTokens {
    /// 终端背景色。
    pub background: String,
    /// 终端默认前景色。
    pub foreground: String,
    /// 光标颜色。
    pub cursor_color: String,
    /// 选区背景色。
    pub selection_background: String,
    /// ANSI 黑色。
    pub black: String,
    /// ANSI 红色。
    pub red: String,
    /// ANSI 绿色。
    pub green: String,
    /// ANSI 黄色。
    pub yellow: String,
    /// ANSI 蓝色。
    pub blue: String,
    /// ANSI 紫色。
    pub purple: String,
    /// ANSI 青色。
    pub cyan: String,
    /// ANSI 白色。
    pub white: String,
    /// ANSI 高亮黑色。
    pub bright_black: String,
    /// ANSI 高亮红色。
    pub bright_red: String,
    /// ANSI 高亮绿色。
    pub bright_green: String,
    /// ANSI 高亮黄色。
    pub bright_yellow: String,
    /// ANSI 高亮蓝色。
    pub bright_blue: String,
    /// ANSI 高亮紫色。
    pub bright_purple: String,
    /// ANSI 高亮青色。
    pub bright_cyan: String,
    /// ANSI 高亮白色。
    pub bright_white: String,
}

impl Default for TerminalThemeTokens {
    fn default() -> Self {
        Self {
            background: "#2B2B2B".into(),
            foreground: "#BBBBBB".into(),
            cursor_color: "#BBBBBB".into(),
            selection_background: "#4B6EAF".into(),
            black: "#000000".into(),
            red: "#CC6666".into(),
            green: "#6A8759".into(),
            yellow: "#BBB529".into(),
            blue: "#6897BB".into(),
            purple: "#9876AA".into(),
            cyan: "#629755".into(),
            white: "#A9B7C6".into(),
            bright_black: "#555555".into(),
            bright_red: "#FF6B68".into(),
            bright_green: "#A5C261".into(),
            bright_yellow: "#F0C674".into(),
            bright_blue: "#75A8D3".into(),
            bright_purple: "#C586C0".into(),
            bright_cyan: "#8ABEB7".into(),
            bright_white: "#FFFFFF".into(),
        }
    }
}

impl TerminalThemeTokens {
    pub(crate) fn apply(&mut self, patch: &TerminalThemeTokensPatch) {
        macro_rules! apply {
            ($field:ident) => {
                if let Some(value) = &patch.$field {
                    self.$field = value.clone();
                }
            };
        }
        apply!(background);
        apply!(foreground);
        apply!(cursor_color);
        apply!(selection_background);
        apply!(black);
        apply!(red);
        apply!(green);
        apply!(yellow);
        apply!(blue);
        apply!(purple);
        apply!(cyan);
        apply!(white);
        apply!(bright_black);
        apply!(bright_red);
        apply!(bright_green);
        apply!(bright_yellow);
        apply!(bright_blue);
        apply!(bright_purple);
        apply!(bright_cyan);
        apply!(bright_white);
    }
}

/// 可持久化的终端主题定义。终端主题当前保存完整 ANSI 色表。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalThemeDefinition {
    /// 主题身份、类型、继承和来源信息。
    #[serde(flatten)]
    pub metadata: ThemeMetadata,
    /// 此主题覆盖的终端颜色令牌。
    pub terminal: TerminalThemeTokensPatch,
}

/// 解析后的终端主题。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTerminalTheme {
    /// 最终主题自身的元数据。
    pub metadata: ThemeMetadata,
    /// 应用继承链后的完整终端颜色令牌。
    pub tokens: TerminalThemeTokens,
}

/// 主题 API 的稳定错误类型。
#[derive(Debug, Error)]
pub enum ThemeError {
    /// TOML 内容无法反序列化为主题定义。
    #[error("无法解析主题 TOML: {0}")]
    TomlDecode(#[from] toml::de::Error),
    /// 主题定义无法序列化为 TOML。
    #[error("无法编码主题 TOML: {0}")]
    TomlEncode(#[from] toml::ser::Error),
    /// Windows Terminal JSON 内容无法解析。
    #[error("无法解析主题 JSON: {0}")]
    JsonDecode(#[from] serde_json::Error),
    /// 主题文件使用了当前程序不支持的格式版本。
    #[error("主题格式版本 {0} 不受支持")]
    UnsupportedSchema(u32),
    /// 主题文件声明的类型与调用方要求的类型不同。
    #[error("期望 {expected:?} 主题，但文件声明为 {actual:?}")]
    KindMismatch {
        /// 当前导入流程要求的主题类型。
        expected: ThemeKind,
        /// 主题文件实际声明的类型。
        actual: ThemeKind,
    },
    /// 服务中已经存在相同主题 ID。
    #[error("主题 ID 已存在: {0}")]
    DuplicateId(ThemeId),
    /// 服务中不存在指定主题或其父主题。
    #[error("找不到主题: {0}")]
    NotFound(ThemeId),
    /// 主题继承链重复访问同一主题。
    #[error("主题继承形成循环: {0}")]
    InheritanceCycle(ThemeId),
    /// 颜色字段不是受支持的十六进制颜色。
    #[error("主题字段 {field} 包含非法颜色 {value}")]
    InvalidColor {
        /// 出错字段的 TOML 名称。
        field: String,
        /// 出错字段的原始值。
        value: String,
    },
    /// 尺寸字段不是有限值或超出允许范围。
    #[error("主题字段 {field} 的尺寸 {value} 超出范围")]
    InvalidMetric {
        /// 出错字段的 TOML 名称。
        field: String,
        /// 出错字段的原始值。
        value: f32,
    },
    /// 主题目录或文件操作失败。
    #[error("主题 I/O 失败: {0}")]
    Io(#[from] std::io::Error),
    /// 当前平台无法提供应用配置目录。
    #[error("无法确定用户配置目录")]
    ConfigDirectoryUnavailable,
    /// 调用方尝试替换或删除只读内置主题。
    #[error("内置主题只读: {0}")]
    ReadOnlyBuiltin(ThemeId),
    /// 主题 ID 含非法字符、为空或长度超限。
    #[error("主题 ID 非法: {0}")]
    InvalidId(ThemeId),
    /// 主题显示名称去除空白后为空。
    #[error("主题名称不能为空")]
    EmptyName,
    /// 非系统 UI 主题没有声明日间或夜间分组。
    #[error("UI 主题必须声明 period = day 或 night: {0}")]
    MissingPeriod(ThemeId),
    /// 终端主题错误地声明了 UI 专用的日间或夜间分组。
    #[error("终端主题不能声明 day/night period: {0}")]
    InvalidPeriod(ThemeId),
    /// UI 主题分组与最终解析出的明暗模式不一致。
    #[error("主题 {id} 的 period {period:?} 与 color-scheme {color_scheme:?} 不一致")]
    PeriodSchemeMismatch {
        /// 不一致主题的 ID。
        id: ThemeId,
        /// 主题声明的日间或夜间分组。
        period: ThemePeriod,
        /// 最终解析出的颜色模式。
        color_scheme: ColorScheme,
    },
}
