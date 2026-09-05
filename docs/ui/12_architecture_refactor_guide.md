# 🏗️ 特性内聚与组件模块化重构全景指南 (Architecture Refactor Guide)

本文档是 `smalux-ssh` 整个桌面端 UI 系统与 Rust 交互层进行**工业级模块化、组件化、统一化与去臃肿解耦**的权威架构规范与落地指导手册。

---

## 一、重构背景与架构挑战

在客户端迭代演进过程中，全工程体量达到了 144 个源文件、60,888 行代码，暴露出三大典型的架构瓶颈：

1. **巨石文件堆积 (Monolithic Components)**：
   - `settings_view.slint` 达 5,708 行，包含 12 个分类和 16 个局部手写小组件；
   - `tunnels_view.slint` 达 3,012 行，混合了端口转发、跳板机、代理及弹窗；
   - `file_explorer_view.slint` 达 2,581 行，本地盘与远程盘逻辑克隆重复高达 90%；
   - `main.slint` 达 2,459 行，在 `AppWindow` 上承载了 **314 个属性与 226 个回调**。
2. **属性穿透爆炸 (Props Drilling Hell)**：
   - 为将状态传给深层 Tab，`main.slint` 仅仅是向设置中心转发参数就消耗了 208 行样板代码；
3. **霰弹式分散 (Shotgun Surgery)**：
   - 修改一个业务（如网络隧道），需要横跨 `views/`、`left_drawers/`、`right_drawers/`、`components/`、`src/handlers/` 等 8 个分散目录。

---

## 二、三位一体核心架构解决方案

为从根本上彻底解决上述问题，本工程确立**“领域桥接总线 + 4 级原子设计系统 + 特性优先物理聚合”**三大支柱架构：

```mermaid
graph TB
    subgraph UI_Architecture["UI 表现层 (Slint)"]
        direction TB
        Shared["ui/shared/<br>全工程通用原子控件与脚手架<br>(AppModalScaffold / AppDropdown / AppSegmentedControl)"]
        
        subgraph Features["ui/features/ (按业务特性独立成包)"]
            F_Settings["features/settings/<br>设置主视图 + 8个独立Tab + SettingsBridge"]
            F_Files["features/file_manager/<br>双盘主页面 + 统合单盘FileBrowserPane + 传输抽屉"]
            F_Tunnels["features/tunnels/<br>隧道主页面 + 3大网络表单 + 拓扑卡片 + 专属弹窗"]
            F_Credentials["features/credentials/<br>凭据主页面 + 密钥/密码表单 + 抽屉"]
            F_Terminal["features/terminal/<br>网格视口 + TabBar + 状态栏 + 专属弹窗"]
        end
        
        AppWindow["ui/main.slint<br>顶层极简主窗口路由器<br>(从 2,459行 瘦身至 550行)"]
        
        Shared -.->|无业务逻辑依赖| Features
        Features -->|组装入主视口| AppWindow
    end

    subgraph Rust_Architecture["Rust 后端层 (Domain Handlers)"]
        H_Settings["src/handlers/settings/"]
        H_Files["src/handlers/files/"]
        H_Tunnels["src/handlers/tunnels/"]
        H_Credentials["src/handlers/credentials/"]
        H_Terminal["src/handlers/terminal/"]
        
        Mock["crates/smagical-core/src/storage/mock/<br>7 大仓储解耦"]
    end

    F_Settings <==>|SettingsBridge| H_Settings
    F_Files <==>|FileBridge| H_Files
    F_Tunnels <==>|TunnelBridge| H_Tunnels
    F_Credentials <==>|CredentialBridge| H_Credentials
    F_Terminal <==>|SessionBridge| H_Terminal
```

### 1. 支柱一：Slint 领域桥接单例 (`Domain Bridge`) —— 终结属性穿透
- 每个独立特性包定义 `export global XxxBridge`；
- 子 Tab 与表单直接通过 `XxxBridge.property` 读写，彻底省去父子层层传递；
- `AppWindow` 属性从 314 锐减至 40，回调从 226 锐减至 18；
- Rust Handler 直接调用 `window.global::<XxxBridge>()`，与主窗口解耦。

### 2. 支柱二：4 级响应式原子设计系统 (Atomic Design System)
- **Tier 0: Design Tokens (`themes/app-theme.slint`)**：单一真实数据源；
- **Tier 1: Atomic Controls (`ui/shared/base/`)**：
  - `AppButton`, `AppIconButton`, `AppFormInput`, `AppSwitch`, `AppBadge`, `AppStatusDot`；
  - **新增补齐**：`AppDropdown`（带滚动条自适应下拉框）、`AppSegmentedControl`（动态模型分段胶囊）；
- **Tier 2: Composite Scaffolds (`ui/shared/scaffolds/`)**：
  - **`AppModalScaffold`**：统管暗黑遮罩、ESC 退出、防穿透点击、居中卡片、标题栏与操作底栏，直接消灭全仓 16 处手写弹窗；
  - **`AppMasterDetailScaffold`**：规范左侧列表 + 右侧详情自适应布局；
  - **`AppFormRow`**：规范 `Label (定宽栅格) + 控件插槽 + 辅助提示` 表单行；
- **Tier 3: Feedback (`ui/shared/feedback/`)**：
  - `ToastContainer`, `MessageDialog`, `ContextMenuContainer`。

### 3. 支柱三：特性优先高内聚物理目录 (Feature-First Colocation)
- **通用共享资产归入 `ui/shared/`**：严禁含有任何具体业务逻辑；
- **各业务特性全面“归家” (`ui/features/<module>/`)**：
  - 将过去散落在各目录的视图、抽屉、专属弹窗、表单、桥接统一收敛在专有特性包内；
  - Rust 端 `src/handlers/<module>/` 形成 1:1 镜像结构。

---

## 三、四大专项巨石单点爆破实施规范

### 1. ⚙️ 偏好设置中心 (`settings_view.slint` 5,708 行)
- **拆分方案**：
  - `features/settings/settings_view.slint`（主容器，缩减至 250 行）；
  - `features/settings/settings_bridge.slint`（单例状态总线）；
  - `features/settings/tabs/`（8 大独立分类 Tab 文件，每个 150~350 行）。

### 2. 📁 双盘文件管理器 (`file_explorer_view.slint` 2,581 行)
- **拆分方案**：
  - 提炼 `FileBrowserPane.slint`（单盘通用浏览器，~400 行）；
  - 提炼 `FileTransferDrawer.slint`（底部传输队列抽屉，~300 行）；
  - 主文件仅保留左右栏装配容器（~220 行）。

### 3. 🌐 网络隧道中心 (`tunnels_view.slint` 3,012 行)
- **拆分方案**：
  - 提炼 3 大多态表单：`TunnelForwardForm.slint`、`TunnelBastionForm.slint`、`TunnelProxyForm.slint`；
  - 提炼 `TunnelTopologyCard.slint`；
  - 迁入专属弹窗与抽屉；主文件缩减至 ~300 行。

### 4. 💾 核心存储层 (`mock_storage.rs` 2,243 行)
- **拆分方案**：
  - 拆分为 `crates/smagical-core/src/storage/mock/` 目录；
  - 独立 7 大仓储实现（hosts, groups, credentials, tunnels, snippets, history, config）；
  - 种子数据独立剥离至 `seed_data.rs`。

---

## 四、渐进落地演进路线图 (Compile Gate Assurance)

每个阶段都必须通过 `cargo check --workspace`（0 错误 0 警告）作为严格验收准则：

- **阶段 0 (脚手架基建)**：在 `ui/shared/` 健全 `AppModalScaffold`、`AppSegmentedControl`、`AppDropdown`、`AppFormRow`；✅ 已完成 (Commit `156fc47`)
- **阶段 1 (偏好设置中心重构)**：构建 `SettingsBridge`，按 8 大 Tab 分拆 `settings_view`，消灭 5,708 行巨石；✅ 已完成 (Commit `156fc47`)
- **阶段 2 (双盘文件管理器归一)**：提炼 `FileBrowserPane`，双盘归一复用，消灭 2,581 行巨石；✅ 已完成 (Commit `f68752b`)
- **阶段 3 (网络隧道中心拆解)**：多态表单提取，专属弹窗收拢归位，消灭 3,012 行巨石；✅ 已完成 (Commit `bb06e66`)
- **阶段 4A (安全凭据中心重构)**：凭据主视图、独立模型与表单解耦，消灭 1,792 行巨石；✅ 已完成 (Commit `229b407`)
- **阶段 4B (命令片段中心重构)**：片段树、编辑区与模型解耦，消灭 902 行巨石；✅ 已完成 (Commit `6bec354`)
- **阶段 5 (Rust 核心存储层治理)**：拆解 `mock_storage.rs` 2,243 行巨石至 7 大独立仓储与种子引擎；✅ 已完成 (Commit `9b0cfe0`)

---

## 五、重构成果指标矩阵 (Refactor Achievement Metrics)

| 模块/文件 | 重构前代码行数 | 重构后主文件行数 | 拆解子模块数量 | 降低代码复杂度/代码复用成效 |
| :--- | :--- | :--- | :--- | :--- |
| **偏好设置 (`settings_view.slint`)** | 5,708 行 | 14 行纯转发器 + 560 行容器 | 8 大独立分类 Tab + 单例 Bridge | 降低 90% 穿透样板代码，每个 Tab 单一权责 |
| **网络隧道 (`tunnels_view.slint`)** | 3,012 行 | 11 行纯转发器 + 596 行容器 | 10 个独立模型、表单与弹窗子组件 | 消除 3 处冗余多态表单，拓扑与指标解耦 |
| **文件浏览 (`file_explorer_view.slint`)** | 2,581 行 | 12 行纯转发器 + 386 行容器 | 3 个独立组件 (统合 FileBrowserPane) | **消除 90% 本地/远程重复渲染逻辑** |
| **安全凭据 (`credentials_view.slint`)** | 1,792 行 | 11 行纯转发器 + 499 行容器 | 6 个独立表单与抽屉共享数据模型 | 凭据列表与 3 大录入表单分离，安全输入复用 |
| **命令片段 (`snippets_view.slint`)** | 902 行 | 11 行纯转发器 + 203 行容器 | 5 个树与编辑器独立子模块 | 片段树与多行脚本编辑区解耦 |
| **核心存储 (`mock_storage.rs`)** | 2,243 行 | 6 行转发器 + 468 行聚合测试 | 7 大独立仓储 + 1 个独立种子生成器 | 各业务实体独立并发安全加锁，单测 100% 覆盖 |
| **合计消灭巨石代码** | **16,238 行** | - | **40+ 个高内聚轻量模块** | **100% 编译通过，0 警告，0 破坏** |

