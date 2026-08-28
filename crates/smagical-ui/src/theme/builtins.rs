use smagical_core::theme::{LoadedTheme, ThemeError, ThemeRepository, ThemeService};

const UI_PRESETS: &[(&str, &str)] = &[
    (
        "builtin.ui.darcula",
        include_str!("../../ui/themes/presets/ui/darcula.toml"),
    ),
    (
        "builtin.ui.system",
        include_str!("../../ui/themes/presets/ui/system.toml"),
    ),
    (
        "builtin.ui.light",
        include_str!("../../ui/themes/presets/ui/light.toml"),
    ),
    (
        "builtin.ui.one-dark",
        include_str!("../../ui/themes/presets/ui/one-dark.toml"),
    ),
    (
        "builtin.ui.nord",
        include_str!("../../ui/themes/presets/ui/nord.toml"),
    ),
    (
        "builtin.ui.github-light",
        include_str!("../../ui/themes/presets/ui/github-light.toml"),
    ),
    (
        "builtin.ui.github-dark",
        include_str!("../../ui/themes/presets/ui/github-dark.toml"),
    ),
    (
        "builtin.ui.monokai",
        include_str!("../../ui/themes/presets/ui/monokai.toml"),
    ),
    (
        "builtin.ui.solarized-light",
        include_str!("../../ui/themes/presets/ui/solarized-light.toml"),
    ),
    (
        "builtin.ui.solarized-dark",
        include_str!("../../ui/themes/presets/ui/solarized-dark.toml"),
    ),
    (
        "builtin.ui.catppuccin-latte",
        include_str!("../../ui/themes/presets/ui/catppuccin-latte.toml"),
    ),
    (
        "builtin.ui.catppuccin-mocha",
        include_str!("../../ui/themes/presets/ui/catppuccin-mocha.toml"),
    ),
    (
        "builtin.ui.tokyo-night",
        include_str!("../../ui/themes/presets/ui/tokyo-night.toml"),
    ),
    (
        "builtin.ui.rose-pine-dawn",
        include_str!("../../ui/themes/presets/ui/rose-pine-dawn.toml"),
    ),
    (
        "builtin.ui.rose-pine",
        include_str!("../../ui/themes/presets/ui/rose-pine.toml"),
    ),
];

const TERMINAL_PRESETS: &[(&str, &str)] = &[
    (
        "builtin.terminal.darcula",
        include_str!("../../ui/themes/presets/terminal/darcula.toml"),
    ),
    (
        "builtin.terminal.dracula",
        include_str!("../../ui/themes/presets/terminal/dracula.toml"),
    ),
    (
        "builtin.terminal.one-dark",
        include_str!("../../ui/themes/presets/terminal/one-dark.toml"),
    ),
    (
        "builtin.terminal.nord",
        include_str!("../../ui/themes/presets/terminal/nord.toml"),
    ),
    (
        "builtin.terminal.solarized-dark",
        include_str!("../../ui/themes/presets/terminal/solarized-dark.toml"),
    ),
    (
        "builtin.terminal.solarized-light",
        include_str!("../../ui/themes/presets/terminal/solarized-light.toml"),
    ),
    (
        "builtin.terminal.gruvbox-dark",
        include_str!("../../ui/themes/presets/terminal/gruvbox-dark.toml"),
    ),
    (
        "builtin.terminal.tokyo-night",
        include_str!("../../ui/themes/presets/terminal/tokyo-night.toml"),
    ),
    (
        "builtin.terminal.catppuccin-mocha",
        include_str!("../../ui/themes/presets/terminal/catppuccin-mocha.toml"),
    ),
    (
        "builtin.terminal.github-light",
        include_str!("../../ui/themes/presets/terminal/github-light.toml"),
    ),
    (
        "builtin.terminal.github-dark",
        include_str!("../../ui/themes/presets/terminal/github-dark.toml"),
    ),
    (
        "builtin.terminal.monokai",
        include_str!("../../ui/themes/presets/terminal/monokai.toml"),
    ),
    (
        "builtin.terminal.catppuccin-latte",
        include_str!("../../ui/themes/presets/terminal/catppuccin-latte.toml"),
    ),
    (
        "builtin.terminal.rose-pine-dawn",
        include_str!("../../ui/themes/presets/terminal/rose-pine-dawn.toml"),
    ),
    (
        "builtin.terminal.rose-pine",
        include_str!("../../ui/themes/presets/terminal/rose-pine.toml"),
    ),
];

/// 返回所有内置主题的稳定 ID，供设置界面建立初始列表。
pub fn builtin_themes() -> impl Iterator<Item = &'static str> {
    UI_PRESETS.iter().chain(TERMINAL_PRESETS).map(|(id, _)| *id)
}

/// 创建已注册内置预设的服务，并可选加载 repository 中的自定义主题。
pub fn initialize_theme_service(
    repository: Option<&dyn ThemeRepository>,
) -> Result<ThemeService, ThemeError> {
    let mut service = ThemeService::new();
    for (_, source) in UI_PRESETS {
        let definition = service.import_ui_toml(source)?;
        service.register_builtin_ui(definition)?;
    }
    for (_, source) in TERMINAL_PRESETS {
        let definition = service.import_terminal_toml(source)?;
        service.register_builtin_terminal(definition)?;
    }
    if let Some(repository) = repository {
        for theme in repository.discover()? {
            match theme {
                LoadedTheme::Ui(theme) => service.save_ui(theme)?,
                LoadedTheme::Terminal(theme) => service.save_terminal(theme)?,
            }
        }
    }
    Ok(service)
}

#[cfg(test)]
mod tests {
    use smagical_core::theme::ThemePeriod;

    use super::*;

    #[test]
    fn every_builtin_theme_is_valid_and_resolvable() {
        let service = initialize_theme_service(None).unwrap();
        for (id, _) in UI_PRESETS {
            service.resolve_ui(id).unwrap();
        }
        for (id, _) in TERMINAL_PRESETS {
            service.resolve_terminal(id).unwrap();
        }
        assert_eq!(
            builtin_themes().count(),
            UI_PRESETS.len() + TERMINAL_PRESETS.len()
        );
        assert_eq!(service.list_ui_by_period(ThemePeriod::Day).len(), 5);
        assert_eq!(service.list_ui_by_period(ThemePeriod::Night).len(), 9);
        assert_eq!(
            service.get_ui("builtin.ui.system").unwrap().metadata.period,
            None
        );
    }
}
