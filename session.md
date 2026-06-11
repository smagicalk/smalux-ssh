# smagicalssh 会话恢复基线

## 恢复目的

- 项目路径：`F:/code/rust/smagicalssh`
- 项目目标：Rust + Slint 跨平台 SSH 工作台，面向主机管理、终端、SFTP、隧道、凭据、片段、历史、主题和设置。
- 用户核心要求：继续完成核心，同时保持“模块化、功能化、单一化”；后续用户可能自己重写 UI，因此核心、view model、projection、Slint 页面装配必须清晰分层。
- 当前优先级：网络引用模型已经接到主机页，下一步优先补 Network 页资源库 CRUD 和 UI 预览；UI 可以继续打磨，但不要把核心逻辑写进 Slint。
- 回复要求：始终用简体中文；命令、标识符、日志和报错保留原文。

## 当前验证基线

- 最近验证通过：
  - `cargo fmt`
  - `cargo check --color never`
  - `cargo test --color never`，`440 passed`
  - `cargo test -p smagical-core --color never`，`36 passed`
  - `cargo test -p smagical-storage --color never`，`44 passed`
  - `cargo build --color never`
- 2026-06-10 网络内核最近验证：
  - 单主机单代理兼容测试：`host_deserializes_legacy_single_proxy_field` 通过。
  - SQLite 旧 `host_proxy` 单行表自动修复测试通过。
  - `proxy_assets`、`jump_chain_assets`、`jump_chain_steps` 新表已纳入 SQLite 往返与导入导出测试。
- 2026-06-10 文档同步：
  - `README.md` 已同步网络资产内核、SQLite 表、网络页第一版和当前边界。
  - `session.md` 已同步恢复基线、风险和下一步建议。
  - 本次只改文档，没有重新跑全量 Rust 测试。
- 2026-06-11 网络资源库改造验证：
  - 新增 `ForwardAsset`、`ForwardId`、`HostNetworkSelection`。
  - `Host` 已有 `network.proxy_ids`、`network.jump_chain_ids`、`network.forward_ids`。
  - SQLite 新增 `forward_assets`、`host_network_proxies`、`host_network_jump_chains`、`host_network_forwards`。
  - 网络页 view model 改为“运行中 + 网络资源”。
  - `cargo fmt` 通过。
  - `cargo test -p smagical-core --color never`，`36 passed`。
  - `cargo test -p smagical-storage --color never`，`45 passed`。
  - `cargo test --color never app_view_model_projects_network_workspace_items`，`1 passed`。
  - `cargo test --color never`，`440 passed`。
  - `cargo build --color never` 通过。
- 2026-06-11 主机页网络资源选择接入：
  - `QuickHostDraft` 已携带 `network`。
  - 主机创建/编辑弹窗已接上代理、跳板链、转发的多选入口。
  - `HostNetworkSelection` 在核心、projection、view model、Slint 链路中保持单独引用，不把资源细节复制进主机。
  - 重新验证通过：`cargo fmt`、`cargo check --tests --color never`、`cargo build --color never`、`cargo test -p smagical-core --color never`、`cargo test -p smagical-storage --color never`。
- 2026-06-11 网络资产删除保护：
  - `StorageManager` 新增 `proxy_asset_host_ids`、`jump_chain_asset_host_ids`、`forward_asset_host_ids` 等引用查询。
  - `remove_proxy_asset`、`remove_jump_chain_asset`、`remove_forward_asset` 在资产仍被主机引用时会直接拒绝删除。
  - `src/app/view_model/network.rs` 改为统一走存储层引用查询，不再重复扫主机集合。
  - 新增并通过测试：被引用资产不能删除，且引用查询返回的主机 ID 与实际数据一致。
  - 重新验证通过：`cargo fmt`、`cargo check --tests --color never`、`cargo test -p smagical-storage --color never network_assets`、`cargo test -p smagical-core --color never`、`cargo build --color never`。
- 最近预览：
  - 当前检测到 `smagicalssh.exe` 进程 `30672` 正在运行。
  - 直接运行现有 `target/debug/smagicalssh.exe` 可进入运行期日志阶段。
  - `cargo run --color never` 当前可能受 Windows 上已运行 `smagicalssh.exe` 文件锁影响，导致 `target/debug/smagicalssh.exe` 无法替换。
- 编码抽样正常：
  - `session.md` 首字节为 `35,32,115`，无 BOM
  - `README.md` 首字节为 `23,32,115`，无 BOM
  - 中文注释抽样正常
- 当前工作区很脏，包含大量历史修改和未跟踪文件；不要回滚非本轮改动。
- 本次没有 commit，除非用户明确要求，不要自动提交。

## 已完成的核心能力

- SSH 后端：真实连接、认证、交互 shell、远程命令、PTY、断开清理。
- SFTP：浏览、刷新、上传、下载、删除、创建目录、书签、传输状态、取消队列任务。
- 隧道：Local / Remote / Dynamic SOCKS5 的启动、停止、运行态管理和错误收敛。
- Known Hosts：未知主机密钥候选、信任、删除、错配保护。
- 本地终端：可多开，输入与后端事件链路已接入。
- 命令历史：远程命令退出码、耗时、失败和跳过场景会回写历史。
- 存储：已迁移到 SQLite / SeaORM 风格分层，后续加密密码在设置页配置；支持备份、导出、导入和快照思路。
- 设置页文件操作：主题导入/导出、SQLite 备份、快照导入/导出、SQLite 备份导入都会在核心层拒绝空路径；备份/快照导出拒绝覆盖，SQLite 备份导入拒绝当前库路径，并已有设置页消息层测试覆盖。
- 主题：内置主题、自定义主题、导入导出常见格式，主题颜色分块配置。
- i18n：文案从配置文件读取，缺失 key 回退英文再回退 key，不因缺失翻译阻断运行。

## 已完成的重要领域设计

### 主机

- 首页默认是主机页。
- 主机支持树形分组、卡片/树模式、搜索、右键菜单、复制、删除、编辑、创建主机、创建分组。
- 创建主机弹窗已支持分组选择、图标选择、凭据选择、Agent/OpenSSH/Pageant 等认证来源。
- 主机别名有长度限制，分组路径用可读路径展示，不直接暴露 id。

### 网络

- 主机内联网络配置已从单代理改为：
  - `Host.proxies: Vec<ProxyProfile>`
  - `Host.jumps: Vec<JumpProfile>`
- 网络纯领域模型已从 `Host` 拆到：
  - `crates/smagical-core/src/network.rs`
- 旧 `proxy` 单值字段仍可反序列化，兼容历史 TOML / 快照。
- 已新增可复用网络资产：
  - `ProxyAsset`
  - `JumpChainAsset`
  - `ForwardAsset`
  - `ProxyId`
  - `JumpChainId`
  - `ForwardId`
- 主机已新增网络资源引用：
  - `Host.network.proxy_ids`
  - `Host.network.jump_chain_ids`
  - `Host.network.forward_ids`
- SQLite 已新增并接通：
  - `proxy_assets`
  - `jump_chain_assets`
  - `jump_chain_steps`
  - `forward_assets`
  - `host_network_proxies`
  - `host_network_jump_chains`
  - `host_network_forwards`
- 旧 SQLite 单行 `host_proxy` 表会在连接时自动修复为多行结构。
- 删除主机时会同步清理跳板链中对该主机的引用，避免后续保存命中外键错误。
- 网络资产内存 CRUD 已拆到：
  - `crates/smagical-storage/src/network_assets.rs`
- SQLite 网络资产 mapper 已拆到：
  - `crates/smagical-storage/src/sqlite/mapper_network.rs`
- Network 页已有第一版双栏展示：
  - 运行态隧道
  - 网络资源库，混合展示代理、跳板链和转发资产
- 当前 Network 页只对运行态隧道暴露停止动作，其他对象先展示，不假装可直接执行。

### 凭据

- 左侧栏“凭据”包含私钥、证书、密码等，不展示主机关联。
- 凭据页面使用树形分组，右侧展示选中凭据或选中分组内容。
- 已实现创建、导入、复制、删除、编辑、导出、查看内容、密码可见/隐藏、复制内容。
- 私钥导入会保存到数据库，不依赖原文件继续存在。
- 私钥算法下拉和导入算法识别已接入。
- 证书和密码详情页使用各自文案，不再全部显示“编辑密匙”。
- 认证代理不再放在凭据模块，后续计划放到网络/代理类模块。

### 片段

- 片段核心已重做为“逻辑片段 + 脚本实现 + 支持目标”：
  - 一个逻辑片段可以支持多个系统/环境目标。
  - 多个目标可以共享同一份脚本实现。
  - 目标也可以拆分为独立实现。
- 片段 scope 只保留 `Global` 和 `Host`，文件夹只负责整理。
- SQLite 已有 `snippet_implementations`、`snippet_support_targets`，参数绑定到 implementation。
- 片段树结构：分组/文件夹 -> 逻辑片段 -> 支持目标。
- 支持目标节点可以运行和复制命令；目标级删除、共享、拆分已有核心和 UI 闭环。
- 片段目标标记选择已从下拉改为独立多选弹窗：
  - 点击目标标记展示区打开弹窗。
  - 弹窗内使用图标化预设标签，支持多选。
  - 取消不回写，保存后才更新原标签区域。
  - 创建目标支持多标签创建；编辑目标时第一个标签更新当前目标，后续标签创建为共享同一份脚本的目标。
- 片段树最近 UI 状态：
  - 目标变体树节点显示逻辑片段名称，不再把 `linux`、`debian-ubuntu` 等目标标签当节点名称。
  - 目标标签仍放在右侧 badge 和详情区域。
  - 左侧片段树已有右键菜单：分组可新建/新建分组/编辑/移动/删除；片段可新建目标/编辑/移动/运行/删除；目标变体可运行/编辑/独立编辑/删除。
  - 片段节点支持拖动到其他分组或根分组；分组移动暂时保留右键“移动”对话框，避免拖拽循环父子关系处理不完整。

## 已完成的模块化拆分

### Rust 回调和分发

- `host_actions.rs` 已拆分：
  - `host_actions_quick_host.rs`
  - `host_actions_credentials.rs`
  - `host_actions_helpers.rs`
- `settings_actions.rs` 已拆分：
  - `settings_theme_actions.rs`
  - `settings_storage_actions.rs`
- `callbacks/workspace/layout.rs` 已拆出：
  - `workspace/snippet_actions.rs`
  - `workspace/snippet_helpers.rs`
- `dispatch/ui.rs` 已拆分：
  - `ui_quick_host.rs`
  - `ui_workspace.rs`
  - `ui_terminal.rs`
- `dispatch/target.rs` 已抽出 `is_visual_message`、`is_workspace_message`、`is_ui_message`、`is_storage_message`、`is_session_message`、`is_sftp_message`、`is_launch_message`、`is_snippet_message` 等 helper。

### Rust view model / projection

- 已新增 Rust 侧页面参数模型：
  - `TerminalWorkspaceViewModel`
  - `SecurityWorkspaceViewModel`
  - `SnippetWorkspaceViewModel`
  - `SettingsWorkspaceViewModel`
- `AppViewModel` 中和页面模型重复的旧根字段已按页面删除：
  - Settings：移除根 `settings`
  - Terminal/SFTP：移除根 `terminal`、`sftp`、`tabs`、`history`、`tunnels`、`known_hosts`、`tool_panel_*`
  - Snippets：移除根 `snippet_search_query`、`snippets`、`snippet_rows`、`snippet_target_options`
  - Security：移除根 `credential_search_query`、`credentials`、`credential_rows`、`credential_group_contents`、`credential_detail_fields`
- projection 已经从页面模型读取：
  - `projection.rs` 整窗终端同步读 `model.terminal_workspace.terminal`
  - `projection/sftp.rs` 读 `model.terminal_workspace.sftp`
  - `projection/collections.rs` 按 terminal/security/snippet/settings 页面模型写列表
  - `projection/workspace.rs` 的设置页、凭据搜索、片段搜索和工具面板状态读页面模型
- `sync_terminal_pane` 仍保留局部刷新路径，继续直接从 `active_terminal(state)` 同步终端，不依赖整窗 `AppViewModel`。
- `app_view_model_projects_workspace_page_models` 已改为验证页面模型自身字段，不再依赖旧根字段对照。
- `src/app/projection/models/tools.rs` 已收敛为薄入口，拆出：
  - `tools_common.rs`
  - `tools_credentials.rs`
  - `tools_snippets.rs`
- `src/app/view_model/root/builder.rs` 已把工作区静态文案移到：
  - `src/app/view_model/root/workspace_text.rs`
- `builder.rs` 继续保留 App 根展示模型装配职责。
- `pending_delete_group_caption` 的 `tr` 导入遗漏已修复。
- 凭据 view model 已拆成 credentials common/detail/group/tree 等多个文件。

### Storage / SQLite

- SQLite mapper 和 migration 已按领域拆分：
  - `mapper_common.rs`
  - `mapper_credentials.rs`
  - `mapper_hosts.rs`
  - `migration_common.rs`
  - `migration_credentials.rs`
  - `migration_extensions.rs`
  - `migration_history.rs`
  - `migration_hosts.rs`
  - `migration_settings.rs`
  - `migration_snippets.rs`
- 网络资产层已补入 SQLite：
  - `proxy_assets`
  - `jump_chain_assets`
  - `jump_chain_steps`
- 旧单代理 `host_proxy` 表的自动修复逻辑已接入 `connect_and_migrate`。
- 后续加密存储预留：密码在设置页配置，目前暂无。

### Slint UI

- 已有组件/页面拆分：
  - 主机：`host_card.slint`、`host_tree.slint`、`host_form_dialog.slint`、`host_dialog_parts.slint` 等
  - 凭据：`credential_panel.slint`、`credential_panel_parts.slint`、`credential_controls.slint`、`security_credential_detail_page.slint` 等
  - 片段：`snippet_panel.slint`、`snippet_detail_page.slint`、`snippet_dialogs.slint`、`snippet_detail_parts.slint` 等
  - 设置：`settings_page.slint`、`settings_page_parts.slint`
  - 工作区：`workspace_common.slint`、`workspace_tabs.slint`、`terminal_pane.slint`、`sftp_pane.slint`
- 最近新增页面 wrapper：
  - `ui/workspace_snippet_page.slint`
  - `ui/workspace_security_page.slint`
  - `ui/workspace_settings_page.slint`
  - `ui/workspace_terminal_page.slint`
  - `ui/workspace_network_page.slint`
- `ui/main_workspace.slint` 现在偏页面装配入口，外部 property/callback 名称保持不变，避免影响 Rust 生成绑定。
- `ui/main.slint` 已增加中文分区注释，标明全局导航、主机、凭据、片段、设置、终端/SFTP、集合数据和回调职责。
- `ui/components.slint` 已补强为稳定组件聚合出口，供后续重写 UI 复用；当前不强制现有页面从 facade import，避免循环依赖。

## 当前风险和注意事项

- `ui/main.slint` 仍是最大文件，主要因为 Rust 生成绑定需要大量 property/callback。
- `ui/main_workspace.slint` 仍有较长桥接，但页面内容已经下沉到 wrapper。
- 不建议继续机械搬 Slint 代码；Rust 侧页面参数模型重复字段已经收敛，下一步若继续收窄 Slint 接口，应先设计分页面 facade，而不是直接大批量改 `main.slint` property。
- 本轮 Slint 接口评估结论：`main_workspace.slint` 仍暴露大量扁平 property/callback，但当前继续机械搬运会影响生成绑定和页面装配稳定性；推荐等人工 UI 回归后，再按页面做 facade。
- 当前网络资产层已经存在两套表达：
  - 新主路径：`Host.network` 引用代理、跳板链和转发资产 ID。
  - 旧兼容字段：主机内联 `proxies/jumps` 仍保留给旧数据和旧执行链路，UI 主路径不再展示它。
- 当前 Network 页已经有第一版双栏展示，但还不是最终 UI：
  - 资产 CRUD 未完成。
  - 主机页选择网络资源控件未完成。
  - 部分展示文案仍在 Rust 或 Slint 中硬编码，需要继续收敛到 i18n。
- 当前 `TunnelRule` 不携带宿主 `host_id`，而 `Message::StartTunnel` 必须显式传入 `host_id`；因此 Network 页不能诚实地把“已保存隧道模板”表现为可一键启动的已绑定规则。
- 当前工作区大量未跟踪文件是前面拆分和功能开发结果，不要删除；提交时要记得包含网络相关新文件：
  - `crates/smagical-core/src/network.rs`
  - `crates/smagical-storage/src/network_assets.rs`
  - `crates/smagical-storage/src/sqlite/mapper_network.rs`
  - `src/app/view_model/network.rs`
  - `ui/workspace_network_page.slint`
- 如果要提交，先让用户确认 commit message；用户之前提醒过 commit 要求，但本轮没有要求 commit。
- 最近预览：当前检测到 `smagicalssh.exe` 进程 `30672` 正在运行。

## 推荐下一步

1. 继续完善 Network 页资源库 CRUD：
   - 补创建、编辑、删除的 UI 触发入口。
   - 把硬编码展示文案继续收敛到 i18n 配置。
   - 删除资源时，继续沿用存储层引用保护，并把使用位置展示出来。
2. 做人工 UI 预览回归：
   - 首页主机页
   - 新建主机弹窗
   - 凭据页树和详情
   - 片段页目标变体
   - Network 页双栏展示
   - 设置页主题/存储动作
3. 评估 Slint 接口是否继续收窄：
   - 当前 Rust view model 已按页面分组
   - Slint 仍保留大量扁平 property/callback，是生成绑定和现有页面装配的现实成本
   - 推荐下一阶段只做有明确收益的分页面 facade，不做机械搬运
4. 若继续其他核心：
   - 凭据加密密码设置页
   - 文件/SFTP 独立页面完善
5. 若准备提交：
   - 先跑 `cargo fmt`
   - 再跑 `cargo check --color never`
   - 再跑 `cargo test --color never`
   - 检查 `git status --short`
   - 按用户要求 commit/push

## 常用验证命令

```powershell
cargo fmt
cargo check --color never
cargo test --color never app::view_model::tests
cargo test --color never
git diff --check
git status --short
```

## 建议使用的技能

- `diagnose`：遇到闪退、卡住、UI 点击失效、拖拽失效时使用。
- `improve-codebase-architecture`：继续模块化和深层架构收敛时使用。
- `tdd`：新增核心行为、存储迁移、凭据/片段规则时使用。
- `browser:browser`：如果用户要求预览或检查界面，可以用于本地 UI/网页类检查；Slint 原生窗口仍主要用 `cargo run`。

## 本轮进度（Network 管理页）

- 已把 `Proxy` 页从展示页接成管理页，左侧能新建代理、跳板和转发，右侧能编辑和删除。
- 已补核心消息：
  - `SaveProxyAsset`
  - `SaveJumpChainAsset`
  - `SaveForwardAsset`
  - `RemoveProxyAsset`
  - `RemoveJumpChainAsset`
  - `RemoveForwardAsset`
- 已补存储引用保护：
  - 删除代理 / 跳板链 / 转发时，会检查主机引用。
  - 被主机使用时会阻止删除，并返回使用位置。
- 已补 i18n：
  - 网络资源相关文案已走 `assets/i18n/zh-CN.json` / `en-US.json`
  - 代理协议、删除错误提示已收敛到配置文件
- 已补验证：
  - `cargo check --tests --color never` 通过
  - `cargo test -p smagical-storage --color never network_assets` 通过
  - `cargo test -p smagical-core --color never` 通过
  - `cargo build --color never` 通过
- 已知限制：
  - 根 crate 的 `cargo test` 在 Windows 上仍可能撞到 `LNK1140 / PDB` 限制，这是链接器容量问题，不是业务失败。

## 下一步

- 预览 Network 页实际 UI。
- 再检查主机页里选择代理 / 跳板 / 转发的联动。
- 继续把剩余硬编码文案收敛到 i18n。
