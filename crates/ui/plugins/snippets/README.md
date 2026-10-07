# plugin-snippets: 自动化运维代码片段与参数化模板库插件

> **模块定位**：Smalux 常用脚本与参数化命令中枢（左侧活动栏第三位）。支持将高频运维命令、Docker/K8s 编排脚本组织为多层树形库 (`SnippetsCenterView`)，提供参数占位符动态填充 (`{{var}}`)、一键向当前终端注入执行，以及终端内嵌快速命令伴生抽屉 (`SnippetToolDrawer`)。

---

## 一、 模块职责与着力点 (Focus & Scope)

1. **多层级分类树与代码片段管理 (`views/components/`)**：
   - **`SnippetTreePane`**：多层级文件夹结构，支持拖拽排序、模糊搜索（ASCII 零分配）；
   - **`SnippetEditorPane`**：支持 Shell、Python、SQL、PowerShell 等主流语言高亮与全屏编辑；
   - **`SnippetGroupDetail`**：展示分类文件夹下的命令卡片网格与快速执行入口。
2. **动态参数占位符引擎 (`modals/snippet-run-modal.slint`)**：
   - 自动识别脚本中的 `{{container_name}}`、`{{port:8080}}` 等参数占位符；
   - 弹出交互式填报表单，实时在底部预览替换完成后的完整指令；
   - 支持回车一键将指令通过 PTY 管道发送至活跃终端执行。
3. **终端内嵌伴生抽屉 (`companion/snippet_tool_drawer.slint`)**：
   - 在终端使用过程中一键呼出，点击即投递执行常用命令，大幅减少键盘重复输入。

---

## 二、 核心 Slint 组件与调用方法 (Slint Components & Usage)

### 1. `SnippetsCenterView` (全屏代码片段中心)
```slint
import { SnippetsCenterView } from "@plugin-snippets/snippets_plugin.slint";

SnippetsCenterView {
    horizontal-stretch: 1;
}
```

### 2. `SnippetToolDrawer` (终端右侧伴生快速命令抽屉)
```slint
import { SnippetToolDrawer } from "@plugin-snippets/snippets_plugin.slint";

SnippetToolDrawer {
    width: 100%;
    height: 100%;
    collapse => { WindowBridge.is-right-drawer-open = false; }
}
```

### 3. `SnippetRunModal` (动态参数填报与执行弹窗)
```slint
import { SnippetRunModal } from "@plugin-snippets/snippets_plugin.slint";

SnippetRunModal {
    is_open <=> SnippetsBridge.is-run-modal-open;
    snippet_id: SnippetsBridge.run-snippet-id;
    rendered_command <=> SnippetsBridge.run-rendered-command;
    submit => { SnippetsBridge.execute-run-modal(); }
    cancel => { SnippetsBridge.cancel-run-modal(); }
}
```

---

## 三、 核心 Rust 驱动与 API 接口 (Rust Handlers & APIs)

在 `crates/smagical-ui/src/handlers/snippet_handlers.rs` 中集中管理：

### 1. 解析命令占位符与生成参数表单
```rust
pub fn parse_snippet_variables(template: &str) -> Vec<SnippetParamFieldData>
```
- **参数**：包含 `{{key}}` 或 `{{key:default}}` 的脚本原文字符串；
- **返回值**：供 UI 绑定的参数列表结构（含键名、默认值与当前填报值）；
- **说明**：通过高效正规化切片一次性提取所有变量槽位。

### 2. 动态渲染并注入终端执行
```rust
pub fn render_and_execute_snippet(
    template: &str,
    params: &[SnippetParamFieldData],
    terminal_instance: &mut TerminalInstance,
) -> Result<()>
```
- **参数**：脚本模板、用户填入的参数键值对切片、当前选中的终端实例引用；
- **行为**：执行字符串替换，添加换行符并通过 `send_input` 模拟用户敲击发送至远端 shell。

---

## 四、 核心数据模型 (Data Models)

```rust
// 对应 Slint 中的 SnippetTreeNode
pub struct SnippetTreeNode {
    pub id: SharedString,
    pub title: SharedString,
    pub code: SharedString,
    pub is_folder: bool,
    pub parent_id: SharedString,
    pub language: SharedString, // "bash" | "python" | "sql"
    pub auto_execute: bool,
    pub level: i32,
    pub is_expanded: bool,
    pub item_count: i32,
}
```
