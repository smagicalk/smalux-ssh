//! 视觉配置草稿错误。

use std::fmt;

/// 视觉配置草稿错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisualSettingsDraftError {
    InvalidFontSize(String),
    InvalidRotationIntervalSecs(String),
    InvalidOpacity(String),
    InvalidBlur(String),
    InvalidBackgroundSource(String),
}

impl fmt::Display for VisualSettingsDraftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFontSize(value) => write!(f, "无效的字号：{value}"),
            Self::InvalidRotationIntervalSecs(value) => write!(f, "无效的轮转间隔：{value}"),
            Self::InvalidOpacity(value) => write!(f, "无效的透明度：{value}"),
            Self::InvalidBlur(value) => write!(f, "无效的模糊度：{value}"),
            Self::InvalidBackgroundSource(value) => write!(f, "无效的背景来源：{value}"),
        }
    }
}
