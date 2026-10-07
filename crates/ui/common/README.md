# smagical-ui-common: 共享设计系统、原子组件库与领域单例状态总线

> **模块定位**：Smalux GUI 体系的跨插件共享底座。作为单一真相源 (Single Source of Truth, SSOT)，它统一托管全局设计规范令牌 (`AppTheme`)、15+ 套内置配色预设、基础原子与复合 UI 控件库，以及贯穿所有视图和后台 Handlers 的 13 个强类型领域单例状态总线 (`Domain Bridges`)。

---

## 一、 模块职责与着力点 (Focus & Scope)

1. **全局设计规范与动态主题令牌 (`ui/themes/app-theme.slint`)**：
   - 托管全局 `global AppTheme`，定义背景色、前景色、强调色、卡片边框、内边距与圆角阶梯；
   - 支持运行时由后端动态全量注入，实现无需重启应用的 0ms 平滑主题换肤；
   - 随附 `presets/ui/` 与 `presets/terminal/` 共 15 套现代暗黑/明亮配色（如 Catppuccin, Darcula, Nord, Tokyo Night 等）。
2. **标准化原子与复合组件体系 (`ui/shared/`)**：
   - **基础原子控件 (`base/`)**：统一按钮 (`AppButton`, `AppIconButton`)、输入框 (`AppTextInput`, `AppPasswordInput`, `AppTextArea`)、选择与控制 (`AppDropdown`, `AppSwitch`, `AppSlider`, `AppSegmentedControl`)、滚动条 (`AppScrollBar`)；
   - **复合组件 (`composite/`)**：右键上下文菜单 (`AppContextMenu`)、自适应数据表格 (`AppDataTable`)、标签页条 (`AppTabStrip`)、跨插件树形弹窗选择器 (`GroupTreeSelector`, `HostTreeSelector`)；
   - **脚手架 (`scaffolds/`)**：模态弹窗骨架 (`AppModalScaffold`) 与标准表单行 (`AppFormRow`)。
3. **领域单例状态总线 (`ui/features/`)**：
   - 严格解耦视图层与底层 Rust 数据结构，前端各插件与主视口通过 `*Bridge` 互相订阅属性与触发业务回调，消灭跨层级深度属性透传。

---

## 二、 核心领域状态单例总线清单 (Domain State Bridges)

所有 Bridge 均为全局单例 (`global`)，在 Slint DSL 中直接通过 `<BridgeName>.<property>` 访问，在 Rust 中通过 `window.global::<BridgeName>()` 绑定：

| 单例总线名称 | 领域范围 | 核心属性与关键回调 |
| :--- | :--- | :--- |
| **`HostsBridge`** | 主机与资产树 | `hosts`, `tree-nodes`, `selected-host-id`, `filter-query`, `open-host(id)`, `create-host(...)` |
| **`TerminalBridge`** | 终端会话与多视口 | `tabs`, `active-session-tab`, `has-active-session`, `select-tab(id)`, `close-tab(id)`, `send-snippet(cmd)` |
| **`FilesBridge`** | 双盘与 SFTP 传输 | `local-path`, `remote-path`, `local-files`, `remote-files`, `transfers`, `upload-file()`, `open-remote-item(path)` |
| **`CredentialsBridge`** | 凭据与密钥管理 | `credentials`, `is-generate-key-modal-open`, `generate-key-pair-advanced(algo, pass, comment)` |
| **`SnippetsBridge`** | 代码片段与模板 | `snippets`, `run-rendered-command`, `submit-create-snippet(...)`, `execute-run-modal()` |
| **`TunnelsBridge`** | 端口转发与拓扑 | `tunnels`, `active-tunnel-count`, `toggle-tunnel(id)`, `save-tunnel()` |
| **`HistoryBridge`** | 审计历史与屏幕快照 | `history-groups`, `detail-snapshot`, `reconnect(id)`, `filter-history(q)` |
| **`SettingsBridge`** | 系统设置与外观 | `setting-ui-font`, `setting-terminal-font`, `is-unlock-modal-open`, `save-setting(...)` |
| **`WindowBridge`** | 窗口外壳与路由 | `main-view`, `active-left-tab`, `is-left-drawer-open`, `is-right-drawer-open`, `navigate-to(view, sub)` |
| **`AiBridge`** | AI 辅助运维对话 | `chat-messages`, `is-generating`, `send-prompt(text)`, `execute-suggested-cmd(cmd)` |
| **`MonitorBridge`** | 远程实时性能探针 | `metrics`, `is-sampling`, `toggle-sampling(bool)` |
| **`DebugBridge`** | 开发者诊断台 | `is-open`, `logs`, `clear-logs()`, `simulate-traffic()` |
| **`ThemeEditorBridge`** | 主题实时定制工坊 | `is-open`, `active-tab`, `preview-primary-color`, `apply-theme(...)` |

---

## 三、 核心 Slint 共享组件与参数规范 (Components & API)

### 1. `AppButton` (基础主/次/危险按钮)
```slint
import { AppButton } from "@common/shared/base/app-button.slint";

AppButton {
    text: @tr("连接主机");
    variant: "primary"; // "primary" | "secondary" | "danger" | "ghost"
    size: "medium";     // "small" | "medium" | "large"
    icon: @image-url("assets/icons/play.svg");
    enabled: true;
    clicked => { HostsBridge.open-host("host-id"); }
}
```

### 2. `AppTextInput` (标准化文本输入框)
```slint
import { AppTextInput } from "@common/shared/base/app-text-input.slint";

AppTextInput {
    text <=> SettingsBridge.custom-setting-value;
    placeholder: @tr("请输入主机域名或 IP 地址...");
    is-password: false;
    clearable: true;
    accepted(text) => { /* 用户敲击回车 */ }
}
```

### 3. `AppModalScaffold` (高品质遮罩与弹窗容器)
```slint
import { AppModalScaffold } from "@common/shared/scaffolds/app-modal-scaffold.slint";

AppModalScaffold {
    is-open <=> HostsBridge.is-create-group-open;
    title: @tr("新建主机分组");
    card-width: 480px;
    close => { HostsBridge.is-create-group-open = false; }
    // 弹窗插槽主体
    VerticalLayout {
        // ... 表单项
    }
}
```

### 4. `GroupTreeSelector` (无冗余通用层级分组选择器)
```slint
import { GroupTreeSelector } from "@common/shared/composite/group-tree-selector.slint";

GroupTreeSelector {
    group-options: HostsBridge.group-options;
    selected-group-id <=> HostsBridge.target-group-id;
    toggle-group(id) => { HostsBridge.toggle-selector-group(id); }
}
```

---

## 四、 Rust 端与 Bridge 单例交互范式 (Rust Invocation Patterns)

在 Rust 处理器 (`handlers`) 中，统一通过 `window.global::<T>()` 进行状态读写与回调挂载：

```rust
use smagical_ui_common::HostsBridge;
use slint::ComponentHandle;

pub fn bind_hosts_bridge(window: &AppWindow) {
    let bridge = window.global::<HostsBridge>();

    // 1. 设置状态属性 (ModelRc / SharedString / bool)
    bridge.set_is_loading(false);
    bridge.set_filter_query("192.168.".into());

    // 2. 挂载 UI 触发的回调
    bridge.on_open_host(|host_id| {
        tracing::info!("用户请求连接主机: {}", host_id);
    });
}
```
