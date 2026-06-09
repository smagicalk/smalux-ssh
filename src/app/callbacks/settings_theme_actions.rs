//! 设置页主题和语言回调。
//!
//! 这里只解析 UI 稳定 key，并把语言、内置主题、主题导入导出动作转换成核心 `Message`。
//! 主题文件解析和写入仍由核心状态与 theme 模块处理。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::app::callbacks::{AppWindow, SharedAppState, apply_and_sync};
use crate::model::{BuiltInTheme, LanguageMode, Message};
use crate::theme::ThemeExchangeFormat;

pub(super) fn bind(window: &AppWindow, state: &SharedAppState) {
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_set_language(move |language| {
            let Some(language) = parse_language_mode(&language) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::SetLanguage { language });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_set_built_in_theme(move |theme| {
            let Some(theme) = parse_built_in_theme(&theme) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::SetBuiltInTheme { theme });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_export_current_theme(move |target_path, format| {
            let Some(format) = parse_theme_exchange_format(&format) else {
                return;
            };
            apply_and_sync(
                &weak,
                &state,
                Message::ExportCurrentTheme {
                    target_path: target_path.to_string(),
                    format,
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_copy_current_built_in_theme(move || {
            apply_and_sync(&weak, &state, Message::CopyCurrentBuiltInTheme);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_import_theme(move |source_path| {
            apply_and_sync(
                &weak,
                &state,
                Message::ImportTheme {
                    source_path: source_path.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_apply_theme_profile(move |name| {
            apply_and_sync(
                &weak,
                &state,
                Message::ApplyThemeProfile {
                    name: name.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_remove_theme_profile(move |name| {
            apply_and_sync(
                &weak,
                &state,
                Message::RemoveThemeProfile {
                    name: name.to_string(),
                },
            );
        });
    }
}

fn parse_language_mode(value: &str) -> Option<LanguageMode> {
    // 同时接受 UI 稳定 key 和标准 locale code，方便以后设置页直接显示语言代码。
    match value.trim() {
        "FollowSystem" | "system" => Some(LanguageMode::FollowSystem),
        "Chinese" | "zh-CN" => Some(LanguageMode::Chinese),
        "English" | "en-US" => Some(LanguageMode::English),
        _ => None,
    }
}

fn parse_built_in_theme(value: &str) -> Option<BuiltInTheme> {
    // 内置主题 key 由核心枚举提供，避免回调层硬编码全部主题。
    BuiltInTheme::ALL
        .into_iter()
        .find(|theme| theme.key() == value.trim())
}

fn parse_theme_exchange_format(value: &str) -> Option<ThemeExchangeFormat> {
    // 主题导入导出格式使用协议 key，展示标签由 i18n/view_model 负责。
    match value.trim() {
        "NativeToml" => Some(ThemeExchangeFormat::NativeToml),
        "VsCodeJson" => Some(ThemeExchangeFormat::VsCodeJson),
        "WindowsTerminalJson" => Some(ThemeExchangeFormat::WindowsTerminalJson),
        "AlacrittyToml" => Some(ThemeExchangeFormat::AlacrittyToml),
        "ItermColors" => Some(ThemeExchangeFormat::ItermColors),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_callbacks_parse_language_keys() {
        assert_eq!(
            parse_language_mode("FollowSystem"),
            Some(LanguageMode::FollowSystem)
        );
        assert_eq!(parse_language_mode("zh-CN"), Some(LanguageMode::Chinese));
        assert_eq!(parse_language_mode("en-US"), Some(LanguageMode::English));
        assert_eq!(parse_language_mode("missing"), None);
    }

    #[test]
    fn settings_callbacks_parse_builtin_theme_keys() {
        assert_eq!(parse_built_in_theme("Dracula"), Some(BuiltInTheme::Dracula));
        assert_eq!(
            parse_built_in_theme("OceanDark"),
            Some(BuiltInTheme::OceanDark)
        );
        assert_eq!(parse_built_in_theme("missing"), None);
    }

    #[test]
    fn settings_callbacks_parse_theme_exchange_formats() {
        assert_eq!(
            parse_theme_exchange_format("NativeToml"),
            Some(ThemeExchangeFormat::NativeToml)
        );
        assert_eq!(
            parse_theme_exchange_format("WindowsTerminalJson"),
            Some(ThemeExchangeFormat::WindowsTerminalJson)
        );
        assert_eq!(parse_theme_exchange_format("missing"), None);
    }
}
