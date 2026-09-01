# 📜 Smalux-SSH 代码片段与层级运维脚本中心 (Code Snippets & Multi-Level Hierarchy)

## 📌 模块概述与设计目标

代码片段 (Code Snippets) 是现代化运维终端的核心生产力工具之一。运维工程师常需频繁执行高危排障、容器生命周期管理、数据库慢查询追踪及多集群巡检脚本。
Smalux-SSH 代码片段系统旨在提供：
1. **多层文件夹层级树 (Multi-Level Folder Hierarchy)**：支持无限层级文件夹归类组织，左侧树与主机资产树完全统一规范；
2. **参数化动态占位符引擎 (`{{key:default}}`)**：自动解析代码片段中的变量与默认值，在运行前提供结构化填报表单；
3. **双重视图形态**：
   - **全屏代码片段中心 (`SnippetsCenterView`)**：左侧多层树形导航，右侧沉浸式脚本编辑器、元数据与变量提取面板；
   - **双侧边栏伴生抽屉 (`SnippetsDrawer` & `SnippetToolDrawer`)**：左侧独立层级抽屉与右侧伴生抽屉，在终端敲击时随时快速检索并一键注入活动终端；
4. **即时执行 / 仅粘贴模式切换 (`auto_execute`)**：区分自动执行与安全确认。

---

## 🏛️ 领域模型与仓储设计

```text
┌────────────────────────┐           ┌────────────────────────────────┐
│   SnippetGroupRecord   │ 1 ──── n  │         SnippetRecord          │
├────────────────────────┤           ├────────────────────────────────┤
│ id: String             │           │ id: String                     │
│ name: String           │           │ parent_group_id: Option<String>│
│ parent_id: Opt<String> │           │ title: String                  │
│ level: u32             │           │ content: String                │
│ sort_order: i32        │           │ language: String (bash/sql/...)│
│ is_expanded: bool      │           │ auto_execute: bool             │
└────────────────────────┘           │ is_favorite: bool              │
                                     │ description: String            │
                                     │ tags: Vec<String>              │
                                     └────────────────────────────────┘
```

### 1. 动态变量占位符规范

代码片段内容支持使用标准语法 `{{key}}` 或带默认值的 `{{key:default_value}}`：

```bash
docker logs -f --tail={{lines:100}} {{container_name}}
mysql -u {{user:root}} -p{{password}} -h {{host:127.0.0.1}} -e 'SHOW FULL PROCESSLIST;'
kubectl rollout restart deployment/{{deployment_name}} -n {{namespace:default}}
```

- **解析器**：[`SnippetRecord::extract_variables()`](file:///F:/code/rust/smalux-ssh/crates/smagical-core/src/domain/snippet.rs) 自动提取占位符键名与默认初值；
- **渲染器**：[`SnippetRecord::render_template()`](file:///F:/code/rust/smalux-ssh/crates/smagical-core/src/domain/snippet.rs) 根据填报的键值对完成替换，若未填则回退至默认值。

---

## 🌳 树形模型与 DFS 扁平投影 (`snippet_tree_model.rs`)

为了在 Slint 声明式 UI 中以高性能单列表呈现多层树形结构，采用深度优先遍历 (DFS) 将文件夹与片段平铺为有序序列：

```text
📁 Docker 运维 (sgrp-docker)
  ├─ 📄 Docker 容器健康列表 (snip-docker-ps)
  ├─ 📄 Docker 实时日志追踪 (snip-docker-log)
  └─ 📄 清理悬空镜像与未用卷 (snip-docker-prune)
📁 Kubernetes 集群 (sgrp-k8s)
  ├─ 📄 K8s 全命名空间异常 Pod 排查 (snip-k8s-abnormal)
  └─ 📄 K8s 滚动重启 Deployment (snip-k8s-restart)
📁 系统巡检 (sgrp-ops)
  ├─ 📁 网络诊断 (sgrp-ops-net)
  │    └─ 📄 查询指定端口占用与监听进程 (snip-net-port)
  ├─ 📄 Linux CPU 与内存负载 Top 20 (snip-sys-load)
  └─ 📄 磁盘大文件扫描 (>500MB) (snip-disk-large)
```

- **缩进规范**：`padding-left: 6px + node.level * 14px;`；
- **搜索祖先展开链**：当搜索命中子节点时，自动向上回溯并强制展开该路径上的全部父级文件夹；
- **拖拽调序与分组迁移 (`move_and_reorder_raw_snippet_node`)**：
  - 支持拖拽变更层级、移入/移出文件夹、前插/后插及拖至顶级根目录；
  - 内置循环依赖防呆检测（禁止将父文件夹移入其自身或子孙文件夹中）；
- **节点状态与视觉表现**：
  - 文件夹：展开/折叠矢量图标 (`chevron-down.svg` / `chevron-right.svg`、`folder-open.svg` / `folder.svg`) + 直属子项统计胶囊 `Rectangle { height: 16px; border-radius: 8px; ... }`；
  - 脚本：极简纯净终端图标 (`terminal.svg`) + 语言胶囊徽标 (`bash`/`sql`/`python` 等)，保持与主机树完全对齐的清爽视觉。

---

## 🖼️ UI 表现层设计

### 1. 全屏代码片段中心 (`SnippetsCenterView`)

- **左侧树形导航区 (320px)**：
  - 统一暗色搜索框 [`SearchInput`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/search-input.slint)；
  - 标题栏总项数统计与 [`AppIconButton`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/base/app-icon-button.slint)（新建文件夹与新建脚本）；
  - 极细分割线与当前高亮选择背景；
  - 纯净折叠视觉：移除文件夹前的 `>` 符号，直接点击文件夹行进行展开/折叠。
- **右侧资产查看与编辑区 (自适应填充)**：
  - **空状态**：居中对齐终端图标与引导说明；
  - **顶部只读展示标题栏 (36px)**：展示固定标题、所在目录徽标、语言胶囊与更新时间戳，右侧收纳 `[ 复制 ]`、`[ 保存修改 ]`、`[ 删除 ]` 操作；
  - **表单配置行 (28px)**：片段名称修改输入框 + `[✓] 自动追加回车执行` 开关 + 动态占位符检测提示；
  - **等宽代码编辑器**：自适应填满剩余空间，采用 `Cascadia Code` 等宽字体；
  - **底部备注栏 (28px)**：单行紧凑备注说明输入。

### 2. 动态参数填报弹窗 (`SnippetRunModal`)

当执行含有 `{{var}}` 占位符的代码片段时，系统自动弹出动态表单：
- 动态生成输入框，预填默认值；
- 支持回车快速提交或 Esc 取消；
- 提交后自动渲染最终命令并定向注入活动终端窗格。

### 3. 左侧抽屉与右侧终端伴生抽屉

- **左侧 [`SnippetsDrawer`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/views/left_drawers/snippets_drawer.slint)**：完全复刻主机抽屉的树形层级、折叠展开与搜索能力；
- **右侧 [`SnippetToolDrawer`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/views/right_drawers/snippet_tool_drawer.slint)**：终端操作时的快捷侧栏，提供即插即用的紧凑分类与注入卡片。

---

## 📡 领域事件总线集成

代码片段系统的增删改查均触发相应领域事件：

```rust
// 执行代码片段时广播事件
core_state.event_manager().publish(&SnippetExecutedEvent {
    snippet_id: id.clone(),
    snippet_title: s.title.clone(),
    rendered_command: final_cmd.clone(),
    auto_executed: s.auto_execute,
    target_session_id: active_session_id,
});
```
