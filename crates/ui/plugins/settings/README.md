# plugin-settings: 系统偏好设置、外观工坊与数据安全中心插件

> **模块定位**：Smalux 系统级偏好与全局配置中枢（左侧活动栏第七位）。托管全屏设置大页面 (`SettingsCenterView`)、侧边快速设置抽屉 (`SettingsDrawer`)、数据快照备份抽屉 (`BackupDrawer`)，以及支持可视化微调、沙盒视窗与 TOML 双向编辑的**主题定制工坊 (`ThemeEditorModal`)**。

---

## 一、 模块职责与着力点 (Focus & Scope)

1. **八大设置选项卡全面覆盖 (`views/tabs/`)**：
   - **`AppearanceTab` (外观设置)**：15+ 套官方内置预设主题切换、UI 字体与终端字体选择、窗口透明度调节、全局壁纸上传与透明度微调；
   - **`TerminalTab` (终端行为)**：滚动缓冲区行数（1万~10万行）、光标样式（方块/下划线/竖线）、蜂鸣器模式、字形伽马增益；
   - **`NetworkTab` (网络与 SSH 保活)**：全局应用层心跳间隔 (`keepalive_interval`)、重试阈值 (`keepalive_count_max`)、连接超时时间 (`connect_timeout`)、断线自动重连开关 (`auto_reconnect`)、全局 HTTP/SOCKS5 代理设置；
   - **`SecurityTab` (安全与主密码)**：Argon2id 主密码启用/修改、自动锁定闲置超时、物理 SQLite 与内存 Mock 存储模式无缝热切换；
   - **`AiTab` / `BackupTab` / `FilesTab` / `AboutTab`**：AI 模型接入配置、数据全量快照备份管理。
2. **主题设计与定制工坊 (`components/theme-editor-modal.slint`)**：
   - 交互式色轮拾色器 (`color-wheel-picker.slint`)；
   - 包含终端、活动栏、边框与按钮的微型沙盒视窗实时预览 (`theme_preview_sandbox.slint`)；
   - TOML 双向数据同步编辑器 (`theme_editor_inputs.slint`)。

---

## 二、 核心 Slint 组件与调用方法 (Slint Components & Usage)

### 1. `SettingsCenterView` (全屏偏好设置中心)
```slint
import { SettingsCenterView } from "@plugin-settings/settings_plugin.slint";

SettingsCenterView {
    horizontal-stretch: 1;
}
```

### 2. `ThemeEditorModal` (主题定制独立模态工坊)
```slint
import { ThemeEditorModal } from "@plugin-settings/settings_plugin.slint";

ThemeEditorModal { }
```

### 3. `SettingsDrawer` & `BackupDrawer` (轻量抽屉)
```slint
import { SettingsDrawer, BackupDrawer } from "@plugin-settings/settings_plugin.slint";

SettingsDrawer {
    width: 100%;
    height: 100%;
    collapse => { WindowBridge.is-left-drawer-open = false; }
}
```

---

## 三、 核心 Rust 驱动与 API 接口 (Rust Handlers & APIs)

在 `crates/smagical-ui/src/handlers/settings_handlers/` 中集中管理：

### 1. 主题实时热注入
```rust
// theme_handlers.rs
pub fn apply_theme_definition(
    window: &AppWindow,
    theme: &ThemeDefinition,
)
```
- **说明**：将主题中的 40+ 颜色令牌（背景、前景色、高亮色、终端 ANSI 16 色）即时灌入 Slint `AppTheme` 全局单例，无需重启客户端即刻换肤。

### 2. 存储模式切换治理
```rust
// security.rs
pub async fn switch_storage_mode(
    new_mode: &str, // "physical" | "mock"
) -> Result<()>
```
- **说明**：持久化写入 `~/.config/smalux-ssh/storage_mode.txt`，支持用户在开发测试与生产落地之间平滑切换。

---

## 四、 核心数据模型 (Data Models)

```rust
// 对应 Slint 中的 SettingSnapshotItem
pub struct SettingSnapshotItem {
    pub id: SharedString,
    pub name: SharedString,
    pub value_summary: SharedString,
    pub updated_at: SharedString,
}

// 对应 Slint 中的 ThemeOption
pub struct ThemeOption {
    pub id: SharedString,
    pub name: SharedString,
    pub is_dark: bool,
    pub preview_color: Color,
}
```
