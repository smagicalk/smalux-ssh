# 主题 API

本目录只保存 Slint 主题接口和随应用发布的内置预设。主题解析、继承、校验、
配置和文件读写位于 `smagical_core::theme`；Slint 映射位于
`smagical_ui::theme`。未来主题设置页面应该调用这些 API，不要直接操作文件。

## 目录

- `app-theme.slint`：业务组件使用的 `AppTheme` global。
- `presets/ui/`：按 `period` 分为白天（浅色）和黑夜（深色）；System 是跟随系统的选择器。
- `presets/terminal/`：完整终端 ANSI 16 色预设。

应用主题和终端主题是两个独立选择。UI 主题不能作为终端主题使用，反之亦然。

## UI 主题格式

```toml
schema-version = 1
id = "3c09ad54-819a-41af-8dd8-e55ad65d19a8"
name = "My Darcula"
kind = "ui"
period = "night"
base = "builtin.ui.darcula"
author = "Example"

[ui]
accent = "#71A8FF"
selected-background = "#365880"

[metrics]
radius-medium = 4
control-height = 32
```

允许的颜色格式为 `#RRGGBB` 或 `#RRGGBBAA`。缺失字段从 `base` 继承；根主题
缺失字段使用稳定的 Darcula 基线。尺寸范围为 0 到 256 逻辑像素。

UI 主题必须声明 `period = "day"`（白天/浅色）或 `period = "night"`
（黑夜/深色）；只有 `builtin.ui.system` 可以省略该字段并跟随系统。
Rust 侧可通过 `ThemeService::list_ui_by_period(ThemePeriod::Day)` 和
`ThemeService::list_ui_by_period(ThemePeriod::Night)` 分别取得两组主题。

UI 颜色令牌：

```text
window-background, panel-background, surface-background, control-background
foreground, secondary-foreground, disabled-foreground
accent, hover-background, pressed-background
selected-background, selected-foreground
border, focus-border, success, warning, danger, info
```

尺寸令牌：

```text
radius-small, radius-medium, radius-large
spacing-small, spacing-medium, spacing-large
border-width, control-height, icon-size
```

## 终端主题格式

终端 TOML 的 `[terminal]` 包含 `background`、`foreground`、可选的
`cursorColor` 和 `selectionBackground`，以及：

```text
black red green yellow blue purple cyan white
brightBlack brightRed brightGreen brightYellow
brightBlue brightPurple brightCyan brightWhite
```

字段名与 Windows Terminal scheme 兼容。`import_windows_terminal_json` 接受单个
scheme 或包含 `schemes` 数组的 JSON。导入只产生候选，不会自动写盘。

## Rust 接入

```rust,ignore
use smagical_core::theme::{FileThemeRepository, ThemeRepository};
use smagical_ui::theme::{apply_theme_by_id, initialize_theme_service};

let repository = FileThemeRepository::new(std::env::current_exe()?.parent().unwrap())?;
let service = initialize_theme_service(Some(&repository))?;
apply_theme_by_id(&window, &service, "builtin.ui.nord")?;
```

导入、验证、预览并保存：

```rust,ignore
let candidate = service.import_ui_toml(&source)?;
let warnings = service.validate_ui(&candidate)?;
// UI 可展示 warnings，并用临时 ThemeService 解析后调用 apply_ui_theme 预览。
repository.save_ui(&candidate)?;
```

导出：

```rust,ignore
let source = service.export_ui_toml(theme)?;
std::fs::write(destination, source)?;
```

删除前先调用 `ThemeSelectionConfig::impact_of_delete` 展示影响范围，用户确认后再调用
`ThemeRepository::delete` 并迁移配置。内置主题由 `ThemeService::is_builtin` 标识为只读。

## 存储规则

`FileThemeRepository::new` 优先使用程序旁 `themes/`；不可写时使用系统用户配置目录。
UI 应显示 `active_directory()` 的实际结果。写入使用临时文件和原子替换，单个发现文件
最大 1 MiB，损坏文件会被隔离跳过。保存先写入同目录临时文件，再替换目标文件。

## 来源与许可证

- Darcula：根据 JetBrains `intellij-community` Darcula 色值派生，不是 JetBrains 官方产品。
- One Dark：Atom/GitHub，MIT。
- Nord：Nord 配色项目，MIT。
- Dracula：Dracula Theme，MIT。
- Solarized：Ethan Schoonover，MIT。
- Catppuccin：Catppuccin，MIT。
- Tokyo Night：folke/tokyonight.nvim，Apache-2.0。
- Gruvbox：morhetz/gruvbox。
- GitHub：GitHub Primer，MIT。
- Monokai：Monokai 配色项目。
- Rosé Pine：Rosé Pine 主题项目，MIT。

发布时应在第三方声明中保留上述来源和相应许可证文本。
