//! 视觉配置草稿字段。

/// 全局视觉配置草稿字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualSettingsDraftField {
    ThemeName,
    FontFamily,
    FontSize,
    BackgroundSources,
    RotationIntervalSecs,
    Opacity,
    Blur,
}
