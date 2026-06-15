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

## 本轮进度（Network 跳板链主机选择）

- 已把 Network 页的跳板链弹窗从“手填主机 UUID”改成“选择已保存主机”：
  - 弹窗接入 `HostRow` 主机列表。
  - 每个主机显示图标、名称、endpoint 和分组/状态。
  - 点击主机卡片会追加或移除该主机，保存时仍提交核心层需要的 host id 列表。
- 已把主机选择字符串逻辑收敛在 Rust callback adapter：
  - `network-jump-host-selected` 只负责判断选中状态。
  - `toggle-network-jump-host` 负责解析、追加、移除和重新生成 id 列表。
  - Slint 只负责展示和触发，不直接解析 UUID。
- 已调整 i18n 文案：
  - `proxy.host_ids_label` 从“跳板主机 ID”改为“跳板主机”。
  - `proxy.host_ids_placeholder` 改为选择主机的提示。
- 已补轻量单元测试覆盖：
  - 主机 id 文本支持常见分隔符。
  - 点击可追加和移除主机。
  - 已选主机可以计算 1-based 跳板顺序。
  - 旧文本损坏时会恢复为当前点击主机。
- 已继续补弹窗可用性：
  - 已选主机会显示序号徽标，序号就是跳板链顺序。
  - 未选中主机仍显示分组/状态，不额外占用空间。
- 已补验证：
  - `cargo fmt` 通过。
  - `cargo check --tests --color never` 通过。
  - `cargo test -p smagical-storage --color never network_assets` 通过。
  - `cargo test -p smagical-core --color never` 通过。
  - `cargo build --color never` 通过。
- 未通过项说明：
  - `cargo test --color never app::callbacks::workspace::tests` 触发根 crate 链接，Windows `link.exe` 报 `LNK1140 / PDB` 限制；这是已知根测试链接限制，不是新增逻辑失败。
  - 新增序号后再次执行 `cargo build --color never` 超时退出，未留下 cargo/rustc/link 残留进程；此前同轮 `cargo check --tests --color never` 已通过。

## 本轮进度（Network 资源搜索）

- 已给 Network 页新增资源搜索：
  - 搜索词进入核心 UI 状态 `WorkspaceUiState.network_search_query`。
  - 新增 `UpdateNetworkSearchQuery` 消息，按现有 Host / Credential / Snippet 搜索路径分发。
  - `NetworkWorkspaceViewModel` 暴露 `search_query`，Slint 页面输入框直接绑定该值。
- 已在 view model 层过滤 Network 列表：
  - 运行态隧道和已保存资源都会按搜索词过滤。
  - 过滤字段覆盖标题、类型、subtitle、meta、note、统计值、详情值和使用位置。
  - 可以搜索代理名、跳板链名、转发名、主机名、端口、标签等。
- 已接入 UI：
  - Network 页左侧标题下新增搜索框。
  - 搜索框复用现有 `HostSearchField`，保持主题化输入框样式。
  - 新增 i18n 文案 `proxy.search_placeholder`。
- 已补测试：
  - `WorkspaceUiState` 默认 Network 搜索为空。
  - `UpdateNetworkSearchQuery` 会更新 UI 状态。
  - Network view model 会按搜索词过滤资源列表。
- 本轮验证：
  - `cargo fmt` 通过。
  - `cargo check --tests --color never` 通过。
  - `cargo test -p smagical-storage --color never network_assets` 通过。
  - `cargo test -p smagical-core --color never` 通过。

## 本轮进度（核心与 UI 分离）

- 核心门面继续成形：
  - `CoreState` 已直接承载后端队列执行链路。
  - 新增 `CoreState::drain_backend_queue_with_shared_executor()`，无 UI 场景可同步 drain 后端命令。
  - 新增 `CoreState::apply_and_drain_backend_queue()`，可在 CLI、测试工具或未来其他 UI 中提交消息后直接执行核心队列。
- 后端 pump 逻辑下沉到核心：
  - `drain_backend_queue`、`next_backend_command_for_worker`、`apply_backend_command_result` 的真实实现迁到 `CoreState`。
  - 后端事件归约、远程命令历史完成、过期命令清理、SFTP 传输失败收尾、Known Hosts 未信任记录都改为操作 `CoreState`。
  - `AppState` 只保留兼容包装，用于把核心错误同步到当前桌面 `UiState`。
- 桌面 Adapter 收敛：
  - `src/app/pump.rs` 不再临时组装 `AppState` 来处理后台事件，直接操作 `state.core`。
  - `src/app/callbacks.rs` 不再散落 `with_app_state` 调用，统一通过 `DesktopAppState::apply_message(s)` 和 `apply_messages_with_persistence()`。
  - `with_app_state` 只剩 `src/app/state.rs` 内部过渡 seam，后续拆分消息分发时只需改这一处。
- 注释和文档收敛：
  - 更新 `app`、`view_model`、`model/app_state`、`bootstrap`、`callbacks`、`pump` 头部说明。
  - 明确 `AppState` 是桌面过渡组合器，真正无 GUI 核心入口是 `CoreState`。
- 测试修复：
  - 补齐 `view_model` 子模块测试中遗漏的 `AppState` 导入。
  - 修复 3 个 SFTP 后端泵测试里的借用顺序问题。
- 本轮验证：
  - `cargo fmt` 通过。
  - `cargo check` 通过。
  - `cargo check --tests` 通过。
  - `cargo test core_state_can -- --nocapture` 已进入链接阶段，但 Windows MSVC 报 `LNK1140` PDB 超限；改用临时禁用 PDB 后重新编译超时，未完成运行。当前以 `cargo check --tests` 作为测试代码类型基线。

## 下一步建议

- 继续拆 `Message` 分发：把纯核心消息直接落到 `CoreState`，把输入框、弹窗、筛选这类桌面草稿消息留在 `DesktopAppState`。
- 之后再把 `AppState` 退化成测试兼容层或删除，让新 UI 只依赖 `CoreState + Message + 后端运行入口`。
- 根 crate 单测建议后续单独处理 Windows PDB 问题，例如给测试 profile 降低 debug 信息或拆小测试目标。

## 本轮进度（核心消息继续下沉）

- 继续把纯核心消息从桌面过渡层剥离：
  - `Message::BackendEventReceived`
  - `Message::CloseSessionTab`
  - `Message::ActivateTerminalTab`
  这三类消息现在会先在 `DesktopAppState` 被识别，再直接进入 `CoreState::apply_core_message()`。
- 会话和后台事件分发继续下沉到核心：
  - `dispatch/backend.rs` 的真实实现已在 `CoreState`
  - `dispatch/session.rs` 的真实实现已在 `CoreState`
  - `session_tabs/activate.rs`
  - `session_tabs/close.rs`
  - `session_tabs/pending.rs`
  - `session_tabs/sftp_cleanup.rs`
  - `session_tabs/tunnel_cleanup.rs`
  这些会话生命周期实现现在都直接操作 `CoreState`。
- 继续清理过渡残留：
  - 删除了迁移后不再使用的 `AppState` 包装方法和多余导入。
  - 保持单一真实入口，避免核心/桌面双实现漂移。
- 当前架构状态：
  - `CoreState` 已能独立承接：
    - 后端命令队列执行
    - 后端事件归约
    - 会话激活/关闭
    - SFTP 浏览器 owner 清理
    - 隧道关闭前检查与运行态回收
  - `DesktopAppState` 现在只在“仍依赖 `UiState` 草稿”的消息上回退到旧 `AppState` 过渡层。
- 本轮验证：
  - `cargo fmt` 通过。
  - `cargo check` 通过。
  - `cargo check --tests` 通过。

## 下一步建议

- 继续挑第二批纯核心消息下沉，优先：
  - `Launch` 中不依赖 `ui.*draft` 的分支
  - `Sftp` 中只读导航/刷新类分支
  - `Storage` 中不依赖确认弹窗草稿的直接保存类分支
- 等这三批继续下沉后，再重写 `DesktopAppState::apply_message()` 的分流策略，减少 `with_app_state()` 覆盖面。

## 本轮进度（Launch / SFTP 第二批下沉）

- Launch 路径继续拆分：
  - `open_shell`
  - `open_recent_connection`
  - `reconnect_shell`
  - `open_sftp`
  这些动作的“核心行为”已经迁到 `CoreState`。
- 对应桌面副作用被局部收口：
  - `WorkspacePage::Terminal`
  - `WorkspacePage::Sftp`
  现在只在 `AppState` 包装层补充页面切换，核心本体不再直接知道工作区页面。
- SFTP 运行期继续下沉到核心：
  - `refresh_sftp`
  - `navigate_sftp`
  - `select_sftp_entry`
  - `save_sftp_bookmark`
  - `open_sftp_bookmark`
  - `remove_sftp_bookmark`
  - `claim_sftp_session_id_for_host` 相关选择逻辑
  这些已经直接运行在 `CoreState`。
- `dispatch_sftp_message` 已拆成两段：
  - 纯核心的 SFTP 浏览/导航/书签消息直接走 `CoreState`
  - 仍依赖草稿的上传、下载、创建目录、删除文件、取消传输继续留在桌面过渡层
- `DesktopAppState::apply_message()` 已新增核心直通分流：
  - `BackendEventReceived`
  - `CloseSessionTab`
  - `ActivateTerminalTab`
  - `RefreshSftp`
  - `SaveSftpBookmark`
  - `OpenSftpBookmark`
  - `RemoveSftpBookmark`
  - `NavigateSftp`
  - `SelectSftpEntry`
  以上消息现在不再经过 `with_app_state()`。
- 继续清理过渡残留：
  - 删除了迁移后不再被生产路径使用的 `AppState` 包装和相关未使用导入。

## 当前评估

- `CoreState` 已经独立承接：
  - 后端事件
  - 会话激活/关闭
  - Shell / SFTP 打开核心流程
  - SFTP 浏览、导航、书签
- 当前真正还卡在桌面草稿上的主要是：
  - 上传/下载/创建目录这类依赖输入框草稿的 SFTP 操作
  - 凭据导入/生成后回填 quick host 的 Storage 分支
  - 主机/片段/设置页的弹窗与筛选消息

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。

## 本轮进度（继续把 UI 和核心彻底拆开）

- 当前目标仍然是：先把 UI 和核心代码分离干净，再继续把核心做到可以单独运行、未来可替换 UI。
- 这轮已经把结构继续压薄到两层主线：
  - `CoreState` 作为无 UI 依赖的核心运行态，负责配置、存储、会话、终端和后端队列。
  - `DesktopAppState` 作为 Slint 桌面适配层，只处理 UI 草稿、页面状态和桌面消息路由。
- 构建入口也已经按这个方向切开：
  - `build.rs` 只在 `desktop` feature 下编译 Slint UI。
  - `src/main.rs` 在桌面模式启动 Slint，在无桌面 feature 时直接跑 headless core。
  - `cargo check --no-default-features` 和 `cargo run --no-default-features` 已经能通过。
- 当前还没到 100%，剩下的主要是：
  - 继续清理旧 `AppState` 里的兼容包装，尤其是 `quick_host` 一类仍带较多草稿逻辑的地方。
  - 逐步把测试从旧 `AppState::default()` 迁到 `CoreState` / `DesktopAppState`。
  - 最终再考虑把根 crate 的桌面壳和核心进一步拆得更彻底。

## 当前结论

- 现在不是“完全分离 100%”，但已经明显进入收尾阶段。
- 核心已经具备脱离 UI 独立运行的骨架，下一步继续做的是把遗留的过渡层清干净。

## 本轮进度（quick_host 兼容壳基本收尾）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 这轮把 `quick_host` 这条旧兼容链进一步做到收尾：
  - `SaveQuickHost` 已经直接在 `dispatch/ui_quick_host.rs` 内联
  - `OpenEditHostDialog` 已经直接在 `dispatch/ui_quick_host.rs` 内联
  - `DuplicateHost` 已经直接在 `dispatch/ui_quick_host.rs` 内联
  - `OpenCreateHostDialogInGroup` 已经直接在 `dispatch/ui_quick_host.rs` 内联
  - `SaveQuickGroup` 已经直接在 `dispatch/ui_quick_host.rs` 内联
- 对应结果：
  - `src/model/app_state/ui_drafts/quick_host.rs` 现在只剩 `CoreState` helper：
    - `save_host_record`
    - `save_group_record`
    - `duplicate_host_record`
  - 旧 `AppState` 在 `quick_host` 这条线上已经基本没有壳方法了

## 当前评估

- 到现在为止，`quick_host` 这一整块已经基本完成从旧 `AppState` 的脱离：
  - 草稿更新层已内联
  - 分组弹窗流转已内联
  - 编辑/复制/保存已内联
- 这说明旧 `AppState` 现在剩余的价值已经主要集中在：
  - 少量旧分发器/测试壳
  - 某些 `workspace` / `settings` / `storage_admin` 的历史入口
  - 还没有整体迁走的旧测试依赖

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。
- `cargo run --no-default-features` 通过。

## 本轮进度（quick_host 重逻辑继续内联）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 这轮继续收 `quick_host` 里剩下的一批兼容壳：
  - `open_edit_host_dialog`
  - `duplicate_host`
  - `open_create_host_dialog_in_group`
  - `save_quick_group`
- 现在这些逻辑都直接在 `dispatch/ui_quick_host.rs` 里完成：
  - 编辑：直接读已保存主机，填充 `UiState::edit_quick_host(...)`
  - 复制：直接调 `core.duplicate_host_record(...)`
  - 在分组中创建主机：直接校验分组、重置 quick host 草稿、打开弹窗
  - 保存分组：直接校验父节点、落到 `storage.upsert_group(...)`、清空 quick group 草稿
- 同时也顺手把 request/cancel remove 这组兼容壳的遗留清理掉了：
  - `storage_admin.rs` 中的 `request_remove_host/cancel_remove_host/request_remove_group/cancel_remove_group` 已删除
  - 这组逻辑已经由 `dispatch/ui_quick_host.rs` 直接处理

## 当前评估

- `quick_host` 这一条线上，旧 `AppState` 已经基本只剩最重的一个保存主机入口：
  - `save_quick_host`
- 这说明旧 `AppState` 真的已经被压到最后几层了：
  - 大多数 UI 草稿更新层已内联到旧分发器
  - 大多数 core 调用层已直连 `CoreState`
  - 剩下的更多是少量历史入口和测试壳

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。
- `cargo run --no-default-features` 通过。

## 本轮进度（quick_host 分组弹窗相关旧包装清理）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 这轮继续清 `ui_quick_host` 这一条线，把“创建分组/选择父节点”相关的旧 `AppState` 薄包装收掉：
  - `open_create_group_dialog`
  - `open_create_group_parent_dialog`
  - `select_create_group_parent`
  - `close_create_group_parent_dialog`
  - `confirm_create_group_parent`
  - `update_quick_group_name`
  - `select_quick_group_parent`
  - `close_create_group_dialog`
- 现在 `dispatch/ui_quick_host.rs` 自己直接处理这组消息：
  - 校验父节点是否存在
  - 更新 `ui.workspace.pending_create_group_parent_id`
  - 更新 `ui.quick_group`
  - 切换相关弹窗开关
- 对应地，`ui_drafts/quick_host.rs` 里这批方法已经删除，只保留更重的 `save_quick_group/save_quick_host/open_edit_host_dialog` 等逻辑。

## 当前评估

- `quick_host` 这条旧兼容链已经被切掉两层：
  - 前半段草稿更新已内联到旧分发器
  - 这轮分组弹窗流程也已内联到旧分发器
- 现在 `quick_host` 剩下更值得继续处理的，就是：
  - `open_edit_host_dialog`
  - `duplicate_host`
  - `save_quick_host`
  - `save_quick_group`
- 也就是说，这一条线已经进入最后的重逻辑收尾，而不是薄包装清理阶段了。

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。
- `cargo run --no-default-features` 通过。

## 本轮进度（ui_quick_host 前半段薄包装清理）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 这轮收掉了 `ui_quick_host` 里最薄的一批 `AppState` 包装：
  - `update_quick_host_draft`
  - `select_quick_host_group`
  - `update_quick_host_auth_kind`
  - `update_quick_host_auth_field`
  - `toggle_quick_host_network_proxy`
  - `toggle_quick_host_network_jump_chain`
  - `toggle_quick_host_network_forward`
- 现在 `dispatch/ui_quick_host.rs` 自己直接处理这些消息：
  - 直接写 `UiState` 草稿
  - 在需要时校验资源存在
  - 直接返回 `draft_changed()` 结果
- 对应地，`ui_drafts/quick_host.rs` 里这批方法已经删除，只保留更重的保存/创建/编辑相关逻辑。

## 当前评估

- 旧 `AppState` 现在被进一步压缩到两类职责：
  - 兼容旧分发器
  - 少量仍未迁出的保存/创建/编辑壳
- `quick_host` 这一条线上，最薄的草稿更新层已经挪走了；下一步如果继续做，就该轮到：
  - `open_edit_host_dialog`
  - `duplicate_host`
  - `save_quick_host`
  - `open_create_group_dialog`
  - `open_create_group_parent_dialog`
  - `save_quick_group`

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。
- `cargo run --no-default-features` 通过。

## 本轮进度（settings 旧包装退出编译路径）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 这轮把 `settings.rs` 里那批旧 `AppState` 包装收进了旧 `dispatch/ui_workspace.rs`：
  - `export_current_theme`
  - `copy_current_built_in_theme`
  - `import_theme`
  - `apply_theme_profile`
  - `remove_theme_profile`
  - `backup_storage`
  - `export_storage_snapshot`
  - `import_storage_snapshot`
  - `import_sqlite_backup`
- 现在旧 `ui_workspace` 分发器会直接：
  - 调 `CoreState::*_action(...)`
  - 在需要时同步 `ui.visual_settings`
  - 在导入后同步 `UiState` 的 workspace 偏好
- 相应地：
  - `src/model/app_state/settings.rs` 里的 `AppState` 包装已经删掉
  - 该文件现在更纯粹地只保留核心动作和辅助函数

## 当前评估

- 这一轮完成后，旧 `AppState` 在“设置页”这块也不再保留一层单独包装：
  - 旧分发器自己能完成兼容
  - 真实行为仍然只在 `CoreState`
- 现在继续往 100% 走，最适合收的还是：
  - `ui_quick_host` 里剩余的旧入口
  - `storage_admin` / `ui_drafts` 里还保留的兼容方法
  - 更关键的是把旧测试入口逐步迁离 `AppState::default()`

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。
- `cargo run --no-default-features` 通过。

## 本轮进度（workspace_ui 空壳退出编译链）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 这轮把已经只剩空壳的 `workspace_ui` 子模块从编译链里移出：
  - `background`
  - `command_palette`
  - `layout`
  - `page`
  - `tool_panel`
- 现在 `src/model/app_state/workspace_ui.rs` 只保留测试入口，不再包含这些子模块。
- 这意味着：
  - 这些文件里原本残留的兼容实现已经不再参与编译
  - 旧 `AppState` 的历史包袱又少了一层
  - 编译图更接近“只保留真正有实现的模块”

## 当前评估

- 旧 `AppState` 的编译面持续缩小：
  - launch / sftp / snippets / visual / storage confirm 已经持续被拆薄
  - `workspace_ui` 这组已空壳模块也退出编译链
- 现在最值得继续做的，不是再去挖同类小包装，而是：
  - 清掉更多旧测试对 `AppState` 的直接依赖
  - 继续收 `ui_quick_host` 和 `settings` 周边剩余的兼容入口
  - 最后再做 Cargo graph 的最终拆分

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。
- `cargo run --no-default-features` 通过。

## 本轮进度（launch 分发继续脱离旧 AppState 包装）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 这轮把旧 `dispatch/launch.rs` 也从那批已经删掉的 `AppState` 薄包装里彻底脱离出来：
  - `OpenShell`
  - `OpenRecentConnection`
  - `ReconnectShell`
  - `OpenSftp`
  - `RunRemoteCommand`
  - `StartTunnel`
  - `StopTunnel`
- 现在这些消息在旧分发器里直接：
  - 调 `CoreState`
  - 必要时切 `WorkspacePage`
- 这意味着：
  - launch 主路径不再依赖旧 `AppState` 里那批已经删除的中转方法
  - 旧 `AppState` 的历史职责继续往外剥离

## 当前评估

- 旧 `AppState` 现在已经越来越像兼容壳，而不是当前系统的运行核心：
  - launch 已直连 core
  - snippets 已直连 core
  - storage confirm 已直连 core
  - visual 已直连 core / UiState
- 接下来更值得继续削的，是：
  - `ui_quick_host` 里还剩的创建/编辑/保存相关旧入口
  - `workspace_ui` 里还剩的兼容方法
  - 以及更多旧测试对 `AppState::default()` 的直接依赖

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。
- `cargo run --no-default-features` 通过。

## 本轮进度（ui_quick_host 脱离旧 create-host 包装）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 这轮收掉了 `ui_quick_host` 对旧 `workspace_ui/layout.rs` 两个包装的直接依赖：
  - `open_create_host_dialog`
  - `close_create_host_dialog`
- 现在 `dispatch/ui_quick_host.rs` 里直接处理：
  - `OpenCreateHostDialog`
    - `ui.reset_quick_host()`
    - 打开创建主机弹窗
    - 关闭创建分组弹窗
  - `CloseCreateHostDialog`
    - 直接关闭创建主机弹窗
- 这样做的结果：
  - 旧 `workspace_ui/layout.rs` 不再承载这两个真实方法
  - 该文件现在只剩兼容注释占位
  - `ui_quick_host` 这一支更自包含，不再绕旧 workspace_ui 包装

## 当前评估

- 旧 `AppState` 的兼容层现在在继续按域被挖空：
  - visual 已清
  - snippets 参数包装已清
  - storage confirm 包装已清
  - create-host 这两个 workspace_ui 兼容点也清掉了
- 现在剩余最明显的收敛目标还是：
  - `workspace_ui/page/tool_panel/command_palette/background` 的旧文件本身
  - 以及更多旧测试对 `AppState::default()` 的直接依赖

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。
- `cargo run --no-default-features` 通过。

## 本轮进度（visual 清理完成，workspace_ui 收缩一大块）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 这轮完成了 `visual_settings` 这一块旧 `AppState` 包装的清理：
  - `dispatch/visual.rs` 现在直接：
    - 写 `UiState` 草稿
    - 调 `CoreState::apply_visual_profiles_action(...)`
    - 调 `CoreState::apply_host_visual_profiles_action(...)`
    - 调 `CoreState::clear_host_visual_profiles_action(...)`
  - 对应删除了旧 `AppState` 包装：
    - `update_visual_settings_draft`
    - `set_visual_background_enabled`
    - `apply_visual_settings`
    - `update_host_visual_settings_draft`
    - `set_host_visual_background_enabled`
    - `apply_host_visual_settings`
    - `clear_host_visual_settings`
- 同时开始收缩 `workspace_ui`：
  - `dispatch/ui_workspace.rs` 已经内联了 page/layout/tool-panel/command-palette/background 这一批 UI 状态逻辑
  - 对应删除了大部分旧 `workspace_ui/*.rs` 包装
  - 但保留了两个必要兼容点：
    - `open_create_host_dialog`
    - `close_create_host_dialog`
  - 原因是旧 `ui_quick_host` 分发还在直接调用它们，先保持兼容，避免一次动太大

## 当前评估

- 这一轮后，旧 `AppState` 的“纯 UI 状态薄包装”又少了一整块：
  - visual 已经基本清完
  - workspace_ui 已经收掉一大半
- 现在更适合继续处理的剩余点是：
  - `ui_quick_host` 里还直调的 create-host 相关旧入口
  - `workspace_ui` 里保留的两个兼容方法
  - 以及更多旧测试对 `AppState::default()` 的依赖

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。
- `cargo run --no-default-features` 通过。

## 本轮进度（visual_settings 旧包装继续收缩）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 这轮重点收掉了 `visual_settings` 相关的一批旧 `AppState` 包装：
  - `update_visual_settings_draft`
  - `set_visual_background_enabled`
  - `apply_visual_settings`
  - `update_host_visual_settings_draft`
  - `set_host_visual_background_enabled`
  - `apply_host_visual_settings`
  - `clear_host_visual_settings`
- 同时把旧 `dispatch/visual.rs` 改成直接处理消息：
  - 直接写 `UiState` 草稿
  - 直接调用 `CoreState::apply_visual_profiles_action(...)`
  - 直接调用 `CoreState::apply_host_visual_profiles_action(...)`
  - 直接调用 `CoreState::clear_host_visual_profiles_action(...)`
- 这让视觉配置这块的职责更清楚：
  - 草稿和页面相关状态留在 `UiState`
  - 真正配置写入留在 `CoreState`
  - 旧 `AppState` 不再保留这层薄包装
- 顺手清掉了这批旧 helper 里留下的 warning：
  - `host_visual_fallbacks`
  - `invalid_visual_settings`

## 当前评估

- 旧 `AppState` 已经越来越接近纯兼容层：
  - launch / sftp / snippets / storage confirm / visual settings 这几类典型旧包装都在持续减少
  - 现在更多剩下的是 `workspace_ui`、少量 settings 和旧测试入口
- 结构上已经很稳定：
  - `CoreState` 负责真实行为
  - `UiState` 负责纯 UI 草稿
  - `DesktopAppState` 负责当前 Slint Adapter
  - `AppState` 主要是历史兼容和测试壳

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。
- `cargo run --no-default-features` 通过。

## 本轮进度（storage confirm 包装继续压平）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 这轮继续收掉一小块旧 `AppState` 的确认包装：
  - 删除 `confirm_remove_host`
  - 删除 `confirm_remove_group`
- 对应兼容调整：
  - `src/model/app_state/dispatch/storage.rs`
  - `ConfirmRemoveHost / ConfirmRemoveGroup` 现在直接：
    - 从 `ui.workspace.pending_delete_*` 取出待删目标
    - 调 `core.remove_host_record_action(...)`
    - 调 `core.remove_group_record_recursive_action(...)`
- 这样做的结果是：
  - “待确认 id 属于 UI 草稿，真正删除属于核心动作”这一分层更直接
  - 旧 `AppState` 不再额外保留一层 confirm 包装

## 当前评估

- 旧 `AppState` 继续在被掏空：
  - launch / sftp / snippet / storage confirm 这些典型路径的薄包装都在减少
  - 现在剩下更多的是 `workspace_ui` / `visual_settings` 那些历史兼容入口
- 结构已经越来越稳定：
  - `CoreState`：真实行为与状态
  - `UiState`：纯桌面草稿
  - `DesktopAppState`：当前 Slint Adapter
  - `AppState`：越来越接近旧测试和兼容容器

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。
- `cargo run --no-default-features` 通过。

## 本轮进度（snippet 参数路径的旧包装继续收掉）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 这轮继续清理 `AppState` 中一类已经没有生产价值的 snippet 薄包装：
  - 删除 `run_snippet_with_arguments`
  - 删除 `run_snippet_target_with_arguments`
- 对应兼容改动：
  - `src/model/app_state/dispatch/snippets.rs`
  - `RunSnippetWithArguments` 和 `RunSnippetTargetWithArguments` 现在直接：
    - 调 `core.run_snippet_*_action(...)`
    - 在分发器里按结果决定是否切到 `WorkspacePage::Terminal`
- 这样做的结果是：
  - 页面切换副作用继续保留
  - 但旧 `AppState` 不再额外保留一层只做“core 调用 + 页面切换”的包装
- 这一轮收掉的不是大块业务，而是继续清除“历史中转层”：
  - 旧分发器自己就能完成兼容
  - 真正业务实现继续只留在 `CoreState`

## 当前评估

- 旧 `AppState` 现在越来越像“旧测试语义容器”：
  - 典型 launch/sftp/snippet 参数运行路径的单层包装已经基本清掉
  - 剩下更值得继续清理的是：
    - `workspace_ui` 那些只操纵 `UiState` 的旧入口
    - `visual_settings` 那些 desktop 已经自己处理的旧入口
    - 以及更多旧测试对 `AppState::default()` 的直接依赖

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。
- `cargo run --no-default-features` 通过。

## 本轮进度（再删一批旧 AppState 薄包装）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 这轮又删掉一批已经没有生产价值的旧 `AppState` 包装：
  - `run_command_history`
  - `upload_sftp`
  - `download_sftp`
  - `create_sftp_dir`
- 这些方法的共同点和上一轮一致：
  - 生产路径已经在 `DesktopAppState` 或旧 dispatch 里直接读草稿后调核心 action
  - 旧 `AppState` 包装只是在做单层转发
- 对应修正：
  - `src/model/app_state/dispatch/snippets.rs`
    - `RunCommandHistory` 直接调 `core.run_command_history(...)`
  - `src/model/app_state/dispatch/sftp.rs`
    - `UploadSftp / DownloadSftp / CreateSftpDir` 直接在分发器里读取 UI 草稿后调核心 action
    - 不再依赖被删除的旧包装
- 这轮顺手把一处 SFTP 残余 helper 也清掉了：
  - `src/model/app_state/sftp_transfer/path.rs` 中未再使用的 `basename_local_path` 已删除
  - 相关 unused import / dead_code warning 一并消失

## 当前评估

- 旧 `AppState` 继续接近“纯兼容层”：
  - 典型的 launch/sftp/snippet 生产路径包装已经被持续挖空
  - 现在剩下的更多是旧测试入口、workspace_ui 兼容、storage_admin 兼容层
- 结构上已经越来越清楚：
  - `CoreState` 负责真实状态与行为
  - `UiState` 负责纯 UI 草稿
  - `DesktopAppState` 负责当前 Slint Adapter 的组合与副作用
  - 旧 `AppState` 主要是历史兼容债

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。
- `cargo run --no-default-features` 通过。

## 本轮进度（旧 AppState 薄包装再删一批）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 这轮直接删除了一批已经完全没有生产价值的 `AppState` 薄包装：
  - `open_shell`
  - `open_recent_connection`
  - `reconnect_shell`
  - `open_sftp`
  - `run_remote_command`
  - `start_tunnel`
  - `stop_tunnel`
  - `save_host_command_snippet`
- 这些方法之前的共同特点是：
  - 生产桌面路径已经全部改走 `DesktopAppState`
  - 它们只是在旧 `AppState` 里补页面切换或读一下 UI 草稿
  - 核心行为本身早就已经在 `CoreState`
- 删除后补了一处旧分发兼容：
  - `src/model/app_state/dispatch/snippets.rs`
  - `SaveHostCommandSnippet` 现在直接读 `ui.remote_command_for(host_id)` 后调用 `core.save_host_command_snippet_action(...)`
  - 不再依赖已经删除的 `AppState::save_host_command_snippet(...)`

## 当前评估

- 到这一轮为止，旧 `AppState` 又更接近“只给旧测试/兼容模块用”的状态：
  - shell / sftp / tunnel / remote command / host command snippet 这些典型生产路径已经不再有旧包装残留
  - `DesktopAppState` 和 `CoreState` 的职责边界更直接了
- 下一步更适合继续清理的，是同类的薄包装和测试入口：
  - 看看 `AppState` 下还剩哪些 `self.core.xxx(...)` 或 `self.ui.xxx(...)` 的单层转发
  - 把旧测试逐步迁到 `CoreState` / `UiState` / `DesktopAppState` 的真实入口

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。
- `cargo run --no-default-features` 通过。

## 本轮进度（工作区偏好同步继续下沉到 UiState）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 这轮继续把旧 `AppState` 的一块真实职责剥离出去：
  - 新增 `UiState::apply_workspace_preferences_from_config(&AppConfig)`
  - `host_list_mode / language / built_in_theme` 的配置同步现在集中到 `UiState`
- 对应收敛后：
  - `AppState::apply_workspace_preferences()` 只剩薄包装
  - `src/app/bootstrap.rs` 不再保留自己那份 `apply_workspace_preferences(...)` helper
- 这次还顺手把测试面收窄了一点：
  - `src/model/ui_state/tests.rs` 新增 `UiState` 级别的偏好同步测试
  - `src/model/app_state/workspace_ui/tests.rs` 删除那两条偏好同步测试
  - 让这类 UI 偏好行为更明确地归属到 `UiState`
- 这一步继续在削弱旧 `AppState` 的“真实实现”面积：
  - 桌面偏好同步越来越像 `UiState` 的职责
  - `AppState` 只剩迁移兼容和旧测试入口的意义

## 当前评估

- 现在旧 `AppState` 的生产价值已经很弱：
  - 启动不需要它
  - 消息分发不需要它
  - 这轮连工作区偏好同步也已经下沉到 `UiState`
- 下一步如果继续推进，最自然的收尾顺序是：
  - 再收一批旧 `AppState` 的薄包装
  - 把更多旧测试逐步换成 `CoreState` / `UiState` 真实入口
  - 最后再动 Cargo graph 的最终拆包

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。
- `cargo run --no-default-features` 通过。

## 本轮进度（工作区偏好同步收敛到 UiState）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 已把一段仍挂在旧 `AppState` 身上的纯 UI 同步逻辑继续下沉：
  - 新增 `UiState::apply_workspace_preferences_from_config(&AppConfig)`
  - 负责统一同步：
    - `host_list_mode`
    - `language`
    - `built_in_theme`
- 现在这条逻辑的分工变成：
  - `UiState`：知道“配置如何映射成桌面 UI 偏好”
  - `AppState::apply_workspace_preferences()`：只剩薄包装，转发到 `UiState`
  - `src/app/bootstrap.rs`：不再保留自己那份重复的 `apply_workspace_preferences(...)` helper
- 这一步虽然小，但很关键：
  - 又减少了一块旧 `AppState` 的真实实现
  - 消除了 bootstrap 和 AppState 之间的重复偏好同步逻辑
  - 让“桌面 UI 草稿同步”更明确地归属到 `UiState`

## 当前评估

- 到这一轮为止，旧 `AppState` 的角色继续被压扁：
  - 生产启动不依赖它
  - 生产消息分发不依赖它
  - 部分 UI 偏好同步逻辑也已经挪到 `UiState`
- 当前最主要的剩余结构工作不再是 message routing，而是：
  - session.md 历史记录的重复整理
  - Cargo graph 的最终拆分
  - 是否把当前根 crate 再收敛成真正的 app-core/headless 层

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。
- `cargo run --no-default-features` 通过。

## 本轮进度（默认执行器策略去 desktop 命名耦合）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 当前核心默认运行时虽然已经可执行，但还有一个语义耦合点：
  - `CoreState::try_default_runtime()` 里直接依赖 `DesktopBackendExecutor`
  - 这会把“桌面”概念继续暴露到核心运行时装配语义里
- 已做的收敛：
  - 在 `src/backend/local_pty.rs` 引入中性 seam：
    - `type RuntimeBackendExecutor<R> = DesktopBackendExecutor<R>`
    - `default_runtime_backend_executor(remote)`
  - `src/core.rs` 改为只依赖这个中性 seam，而不再在核心层显式写 `DesktopBackendExecutor`
- 这次没有改执行器行为，只改了依赖命名和装配入口的语义方向：
  - 本地 PTY fallback + 远程 SSH/SFTP 组合执行器仍然存在
  - 但核心默认运行时不再直接说“桌面后端执行器”
- 当前意义：
  - 这是在为下一步彻底拆 crate 做准备
  - 先把运行时 Interface 的名字变中性，再拆包会更顺，不会把旧桌面名词带到新的核心 crate 接口上

## 当前评估

- 到这一轮为止，核心分离已经具备这几个稳定点：
  - `cargo check --no-default-features` 通过
  - `cargo run --no-default-features` 通过
  - 默认运行时装配不再由 Slint bootstrap 独占
  - 默认执行器策略在核心层不再显式写 `DesktopBackendExecutor`
- 剩下最大的结构工作已经不再是“状态分发”，而是“Cargo graph 拆包”：
  - 当前根 crate 还是同时承载 core model 和 desktop adapter
  - 真正彻底时，应把 Slint 桌面入口拆成独立 crate，把根 crate 收敛成 app-core/headless 角色

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。
- `cargo run --no-default-features` 通过。

## 本轮进度（headless core 真正可执行）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 已把默认运行时装配从“只能 panic 的桌面启动助手”推进成“核心可直接使用的启动入口”：
  - 新增 `CoreState::try_default_runtime() -> std::io::Result<CoreState>`
  - 保留 `CoreState::with_default_runtime()` 作为桌面启动用的 panic 包装
- 当前分工变成：
  - `CoreState::try_default_runtime()`：
    - 构造真实 SSH 执行器
    - 包装 `DesktopBackendExecutor`
    - 处理 SQLite 加载
    - 处理旧 redb 迁移
    - 注入默认本地终端
  - `src/app/bootstrap.rs`：
    - 调用 `try_default_runtime().unwrap_or_else(...)`
    - 从 `core.config` 生成 `UiState`
    - 同步桌面 UI 偏好
- 已把 `src/main.rs` 的 `#[cfg(not(feature = "desktop"))]` 分支从“直接报错”改为真正启动 headless core：
  - 调用 `CoreState::try_default_runtime()`
  - 输出当前核心摘要：
    - host 数量
    - group 数量
    - session 数量
    - terminal tab 数量
    - 是否存在排队后端命令
- 已完成关键运行验证：
  - `cargo run --no-default-features` 成功执行
  - 当前输出为：
    `headless core ready: hosts=0, groups=0, sessions=1, terminal_tabs=1, has_queued_commands=true`
- 这说明当前无 UI 模式已经不只是“能编译”，而是：
  - 能启动核心
  - 能装配默认执行器和默认存储
  - 能种出默认本地终端运行态

## 当前评估

- 到这一轮为止，第一目标已经推进到三个层面都成立：
  - 运行期：桌面消息分发不依赖旧 `AppState.apply(...)`
  - 构建期：`cargo check --no-default-features` 通过
  - 启动期：`cargo run --no-default-features` 可以真正启动 headless core
- 当前还没有完全做到“最佳分层”：
  - `CoreState::try_default_runtime()` 里仍然用了 `DesktopBackendExecutor`
  - 这意味着默认运行时策略还混着“桌面本地 PTY fallback”的命名和依赖
- 下一步如果继续深挖，最值得做的是：
  - 把“默认运行时执行器策略”从 `DesktopBackendExecutor` 命名中抽出来
  - 或直接把当前 Slint 桌面 crate 再拆出去，让根 crate 只保留 app-core/headless 角色

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。
- `cargo run --no-default-features` 通过。

## 本轮进度（核心默认运行时装配下沉）

- 继续沿“核心可单独运行”推进，这一轮把默认运行时装配从桌面 bootstrap 下沉到 `CoreState`。
- 已新增：
  - `CoreState::with_default_runtime()`
  - `CoreState::ensure_default_local_terminal()`
- `with_default_runtime()` 现在直接负责：
  - 创建默认真实 SSH 执行器
  - 包装成 `DesktopBackendExecutor`
  - 处理 SQLite 默认存储加载
  - 处理旧 redb 到 SQLite 的迁移
  - 加载 `app_config` 回写到 `core.config`
  - 在没有任何标签页时注入默认本地终端
- `src/app/bootstrap.rs` 现在只做桌面 Adapter 自己的事：
  - 调用 `CoreState::with_default_runtime()`
  - 由 `core.config` 生成 `UiState`
  - 同步 `host_list_mode / language / built_in_theme` 到桌面 UI 草稿
- 这意味着：
  - 默认执行器和默认存储装配已经不属于 Slint bootstrap 私有逻辑。
  - 未来 CLI、测试工具、另一套原生 UI 都可以直接复用同一个核心运行时入口。
- 已补核心测试：
  - 验证 `CoreState::with_default_runtime()` 在无 UI 状态下会种出默认本地终端，并排入 `OpenLocalShell` 后端命令。

## 当前评估

- 到这一轮为止，分离已经进入“可运行核心”阶段：
  - 运行期：桌面消息分发不再依赖旧 `AppState.apply(...)`
  - 构建期：`cargo check --no-default-features` 已通过
  - 启动期：核心已经有自己的默认运行时装配入口
- 下一步更大的价值点仍然是 Cargo graph 彻底拆分：
  - 当前 `CoreState::with_default_runtime()` 还依赖桌面场景里的 `DesktopBackendExecutor`
  - 真正完全独立时，应再把“桌面本地 PTY fallback”与“核心默认执行器策略”分开命名或拆层

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。

## 本轮进度（Cargo/构建层加入 desktop feature）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 已在根 crate 引入最小 `desktop` feature 边界：
  - `default = ["desktop"]`
  - `desktop = ["dep:slint", "dep:rfd", "dep:sys-locale", "dep:slint-build"]`
- 已把桌面依赖改为可选：
  - `slint`
  - `rfd`
  - `sys-locale`
  - `slint-build`
- 已修改 `build.rs`：
  - i18n catalog 仍然总是生成。
  - Slint UI 编译改为 `compile_desktop_ui()`。
  - 只有启用 `desktop` feature 时才真正调用 `slint_build::compile("ui/main.slint")`。
  - 无 `desktop` feature 时 build script 不再解析 Slint build 依赖。
- 已修改 crate 暴露面：
  - `src/lib.rs` 中 `app` / `desktop` 改为 `#[cfg(feature = "desktop")]`。
  - `src/desktop.rs` 仅在 `desktop` feature 下编译。
  - `src/main.rs` 在无 `desktop` feature 时返回清晰错误，而不是编译失败。
- 已完成关键验证：
  - `cargo check --no-default-features` 通过。
  - 这说明当前核心路径已经可以在不编译 Slint / rfd / sys-locale 的情况下独立完成编译。
- 已顺手收敛 `no-default-features` 下的噪音：
  - 对只供桌面 Adapter 使用的若干 `*_action` 入口增加了
    `#[cfg_attr(not(feature = "desktop"), allow(dead_code))]`
  - 避免核心无 UI 构建时充满无意义 warning。

## 当前评估

- 这是一个真正的里程碑：
  - 之前只是运行期上把 UI 和核心分开。
  - 现在构建期也开始分开，核心路径已经可以无桌面 UI 依赖通过编译。
- 当前还没有完全做到“核心独立 crate”：
  - 根 crate 里仍然同时放着 core model 和 desktop adapter。
  - 只是通过 feature 把 desktop seam 切了出来。
- 下一步如果继续往前推，最自然的方向是：
  - 把当前根 crate 继续收敛成“app-core”职责。
  - 再把 Slint 桌面入口拆成独立 crate，彻底把 Cargo graph 分开。

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check --no-default-features` 通过。

## 本轮进度（删除桌面主路径旧 AppState 兜底）

- 继续推进第一目标：核心单独运行，未来换 UI 仍能继续运行。
- 这轮完成了一个关键里程碑：
  - `DesktopAppState::apply_message()` 已经覆盖全部 `Message` 分支。
  - 删除了最后的 `_ => self.with_app_state(|state| state.apply(message))` 兜底。
  - 删除了 `DesktopAppState::with_app_state(...)` 过渡方法。
  - 当前桌面运行主路径不再临时组装旧 `AppState` 来分发消息。
- 继续拆出更多核心动作：
  - 确认删除主机/分组：桌面层只持有 pending id，核心层执行 `remove_host_record_action(...)` / `remove_group_record_recursive_action(...)`。
  - 工作区快照：保存、恢复、清除都迁到 `CoreState`，并补 `dispatch_workspace_message` 的核心实现。
  - 启动路由：补 `CoreState::dispatch_launch_message(...)`，让 `CoreState::apply(...)` 能直接处理 shell/sftp/remote command/tunnel 启动消息。
  - 视觉配置：草稿仍留在桌面 `UiState`，应用后的 `ThemeProfile` / `BackgroundProfile` 写入迁到核心动作。
  - SFTP 上传/下载/创建目录：桌面层只读取本地路径、远程文件名、新目录名等草稿，核心层负责校验会话并排后端命令。
- `CoreState::apply(...)` 已改为直接调用 `apply_core_message(...)`：
  - 不再临时构造 `AppState::from_core_state(...)`。
  - 核心入口现在明确只接受不依赖桌面草稿的消息。
  - UI 草稿消息由具体 UI Adapter 处理，而不是在核心里隐式丢弃或伪装执行。
- 清理了 workspace 快照迁移后的旧 `AppState` 包装，避免死代码继续留在核心路径旁边。

## 当前评估

- 这是当前分离工作的关键节点：
  - `DesktopAppState` 已经不再依赖旧 `AppState.apply(...)` 作为生产兜底。
  - `CoreState::apply(...)` 已经不再反向依赖 `AppState`。
  - 旧 `AppState` 还存在，但主要是兼容旧测试和过渡模块，不再是当前桌面主路径必须经过的运行核心。
- 下一步继续做：
  - 收缩旧 `AppState` 的公开价值，逐步把它降级为测试兼容层。
  - 检查 `AppState` 中仍有真实实现但核心/桌面已经有替代实现的模块，删除重复包装。
  - 为 `CoreState::apply(...)` 补更直接的核心入口测试，覆盖 launch/workspace/sftp/storage/snippet 等已核心化消息。

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo check` / `cargo check --tests` 中有 file lock 等待，最终均成功。

## 本轮进度（继续压薄 AppState，桌面主路径直通 CoreState）

- 继续沿“核心单独运行，未来换 UI 也能继续运行”的第一目标推进。
- 已把隧道启动/停止拆成“核心动作 + 桌面副作用”：
  - `CoreState::start_tunnel_action(...)`
  - `CoreState::stop_tunnel_action(...)`
  - 核心负责主机查询、隧道规则校验、标签页/运行态创建、最近连接、后端命令排队。
  - `DesktopAppState` 只在启动成功后切换到 `WorkspacePage::Tunnels`。
- 已把本地终端启动拆到核心：
  - `CoreState::open_local_terminal_action(...)`
  - 核心负责创建本地 session/tab/terminal 和排 `OpenLocalShell`。
  - 桌面层只负责切换终端页和展开主机侧栏。
- 已把终端输入发送拆成两层：
  - `CoreState::send_terminal_input_action(session_id, input)`
  - 核心负责校验会话、记录历史、排 `SendShellInput`、本地终端回显。
  - 桌面层负责从 `UiState` 读取输入草稿，并在成功后清空草稿。
- 已把终端输入草稿和主机/SFTP 草稿更新收进 `DesktopAppState`：
  - `UpdateTerminalInputDraft`
  - `AppendTerminalInputDraft`
  - `BackspaceTerminalInputDraft`
  - `UpdateHostCommandDraft`
  - `UpdateHostSftpInitialDirDraft`
  - `UpdateSftpActionDraft`
- 已把 quick host 主路径继续从旧 `AppState` 兜底迁到桌面适配层：
  - 主机字段草稿、认证草稿、分组选择、网络资产选择。
  - 创建/编辑/复制主机入口。
  - 保存主机时由桌面草稿构造 `Host`，再交给 `CoreState::save_host_record(...)`。
  - 保存分组时由桌面草稿构造 `HostGroup`，再交给 `CoreState::save_group_record(...)`。
- 已清理 `launch/host.rs` 里不再使用的旧 `AppState` 主机 helper 包装，避免继续留下废弃入口。

## 当前评估

- `CoreState` 现在承接了更多真实业务动作：
  - shell/sftp/snippet/run-command 主路径
  - settings/storage/theme 动作
  - tunnel 启停
  - local terminal 启动
  - terminal input 发送
  - host/group 持久记录写入
- `DesktopAppState` 现在更明确承担桌面 Adapter 职责：
  - 页面切换
  - 输入框/弹窗/搜索/树展开等 UI 草稿
  - 成功后同步或清理 UI 草稿
  - 错误文本统一写入 `ui.last_error`
- `AppState` 仍未完全移除，当前主要作为兼容层和旧测试入口存在。
- 下一步最值得继续拆：
  - visual/settings 草稿应用路径
  - workspace UI 快照/局部 UI 包装
  - 继续减少 `DesktopAppState::_ => with_app_state(...)` 的兜底范围

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- 两次 `cargo check` 中出现过短暂 file lock 等待，最终均成功，不是逻辑失败。

## 本轮进度（Launch 主路径继续脱离旧 AppState）

- 继续沿“核心单独运行、未来换 UI 仍能继续运行”推进，这一轮把一批启动主路径直接接到了核心门面：
  - `OpenShell`
  - `OpenRecentConnection`
  - `ReconnectShell`
  - `OpenSftp`
  - `RunRemoteCommand`
- 做法不是直接放大底层私有方法可见性，而是在对应核心模块补稳定 action：
  - `CoreState::open_shell_action(...)`
  - `CoreState::open_recent_connection_action(...)`
  - `CoreState::reconnect_shell_action(...)`
  - `CoreState::open_sftp_action(...)`
  - `CoreState::run_remote_command(...)` 继续复用已有核心入口
- `DesktopAppState::apply_message()` 现在对这批消息直接走核心：
  - 成功后只在桌面层补 `WorkspacePage::Terminal`
  - 或补 `WorkspacePage::Sftp`
  - 不再需要先回退旧 `AppState::dispatch_launch_message(...)`
- 这一轮的结果：
  - 启动 Shell / SFTP / 一次性远程命令 这条主路径更像真正的“核心动作 + 桌面副作用”
  - `dispatch/launch.rs` 继续被压缩，旧 `AppState` 在 Launch 领域里的职责进一步变薄

## 当前评估

- 现在分层状态比上一轮更清晰：
  - `CoreState`：启动远程 Shell、重连、打开 SFTP、运行远程命令
  - `DesktopAppState`：页面切换和活动上下文
  - `AppState`：仍保留兼容壳，但 Launch 主路径已经不是默认入口
- 当前还值得继续拆的 Launch 余量主要是：
  - `StartTunnel`
  - `StopTunnel`
  这两条还在旧 `AppState` 中，因为它们同时依赖会话标签匹配、运行态状态检查和页面切换。

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。

## 本轮进度（凭据元数据与分组继续下沉）

- 继续沿“先分离 UI 和核心代码”推进，把凭据相关 Storage 中不依赖 `UiState` 的纯核心部分下沉到 `CoreState`：
  - `create_credential_group`
  - `rename_credential_group`
  - `remove_credential_group`
  - `move_credential_group`
  - `update_credential_metadata`
  - `update_credential_secret`
  - `export_credential_secret`
  - `duplicate_credential`
  - `remove_credential`
  - `move_credential`
  - `local_private_key_from_secret_ref`
- `dispatch_storage_message` 已继续扩大核心直通范围：
  - 凭据分组增删改移
  - 凭据元数据更新
  - 凭据内容替换
  - 凭据导出
  - 凭据复制 / 删除 / 移动
- `DesktopAppState::apply_message()` 已同步把上面这些消息直接送入 `CoreState`
- 本轮明确保留在桌面过渡层的凭据消息：
  - `create_credential_metadata`
  - `generate_private_key_credential`
  - `save_password_credential`
  - `import_private_key_credential`
  - `import_private_key_text_credential`
  - `import_certificate_credential`
  - `import_certificate_text_credential`
  - `generate_certificate_credential`
  这些分支仍会回填 quick-host 草稿，因此还不能直接归入纯核心。

## 当前评估

- 现在 `CoreState` 已经覆盖：
  - 会话和后端事件
  - Shell / SFTP 打开核心流程
  - SFTP 浏览/书签/删除/取消
  - 网络资源 / known hosts
  - 凭据分组和大部分凭据元数据维护
- UI 和核心代码的第一层分离已经越来越清晰：
  - 核心负责参数化动作和持久化规则
  - 桌面层只负责读取草稿、页面切换、确认状态
- 当前剩余最主要的 UI 耦合点，已经收敛到：
  - quick-host 回填相关凭据创建/导入/生成
  - 主机创建编辑弹窗草稿
  - 片段 / 设置页的纯桌面状态

## 本轮验证

- `cargo check` 通过。
- `cargo check --tests` 通过。

## 本轮进度（远程命令与历史重放继续核心化）

- 继续拆片段运行期上下文相关路径：
  - `CoreState::run_remote_command(...)` 现在承接真正的一次性远程命令启动逻辑
  - `CoreState::run_command_history(...)` 现在承接历史命令重放逻辑
- 当前分层结果：
  - 核心负责：
    - 生成一次性远程命令会话
    - 记录命令历史
    - 排队连接和 `RunCommand`
  - 桌面层负责：
    - 成功后切到 `WorkspacePage::Terminal`
    - 读取当前活动远程主机上下文
- 这一步的意义：
  - `RunCommandHistory` 已经可以在桌面层只做“调用核心 + 切页”
  - `run_remote_command` 本体不再依赖 `UiState`

## 当前评估

- 现在片段/远程命令这条线剩余最大的耦合点，已经缩小到：
  - `run_snippet*` 最后还是通过 `AppState` 桥接到 `run_remote_command`
  - 以及少量仍依赖运行期上下文的执行入口
- 但从整体看，第一目标“UI 和核心代码分离”已经进一步逼近：
  - `CoreState` 负责真正业务动作
  - `DesktopAppState` 只补页面切换、活动上下文和草稿读取

## 本轮验证

- `cargo check` 通过。
- `cargo check --tests` 通过。

## 本轮进度（片段运行期主机上下文本地化）

- 继续拆片段运行期上下文，把“当前活动远程主机”的读取从 `AppState` 挪到 `DesktopAppState`：
  - `DesktopAppState::active_remote_host_id()`
  - `RunSnippetOnActiveHost`
  - `RunSnippetTargetOnActiveHost`
- 当前分层结果：
  - `DesktopAppState` 负责读取“当前活动远程 tab 对应的 host_id”
  - 之后再通过过渡包装调用现有片段执行逻辑
- 这一步虽然还没有把 `run_remote_command(...)` 核心化，但已经把“桌面运行时上下文选择”从 `AppState` 里拿走了一层。

## 当前评估

- 现在 `AppState` 中剩下的片段运行耦合，更多是：
  - 片段渲染后调用 `run_remote_command(...)`
  - `RunCommandHistory`
  - 记录并复用运行参数
- 从“第一目标：UI 和核心代码分离”的角度看，纯桌面上下文已经基本都在往 `DesktopAppState` 收口了。

## 本轮验证

- `cargo check` 通过。
- `cargo check --tests` 通过。

## 本轮进度（主机命令草稿保存片段改为核心动作）

- 继续沿“UI 草稿触发核心动作”的方向推进，把 `SaveHostCommandSnippet` 拆成：
  - `DesktopAppState` 读取 `ui.remote_command_for(host_id)` 草稿
  - `CoreState::save_host_command_snippet_action(host_id, command)` 负责真正保存片段
- 这意味着：
  - 生产路径里保存主机命令片段，不再需要临时组装 `AppState`
  - `AppState::save_host_command_snippet()` 只保留为兼容壳，用于旧测试和过渡调用

## 当前评估

- 片段模块目前已经分成两块：
  - 维护类消息：已核心化
  - 运行期上下文类消息：仍留在 `AppState`
- 现在片段线剩下最明显的 UI/运行期耦合就是：
  - `RunSnippetOnActiveHost`
  - `RunSnippetTargetOnActiveHost`
  - `RunCommandHistory`
  - 以及 `run_remote_command(...)` 这条真正触发执行的路径
- 也就是说，从“UI 草稿触发核心动作”的角度看，主机命令草稿保存片段这条已经收干净了。

## 本轮验证

- `cargo check` 通过。
- `cargo check --tests` 通过。

## 本轮进度（主题/语言/背景纯偏好本地化）

- 继续收口纯桌面偏好，把以下消息直接在 `DesktopAppState` 本地处理，不再通过 `AppState`：
  - `NextTheme`
  - `SetLanguage`
  - `SetBuiltInTheme`
  - `NextBackground`
- 当前分层结果：
  - `DesktopAppState` 负责：
    - 当前页面 / 面板 / 搜索 / 命令面板
    - 删除确认 pending
    - 创建弹窗开关
    - 主题 / 语言 / 背景轮播这类纯桌面偏好
  - `CoreState` 继续负责真正的业务和持久化规则
- 虽然这几条消息仍会同步写 `config.workspace.*` 和 `storage.app_config`，但它们已经不需要走 `AppState` 过渡分发。

## 当前评估

- 到这一轮，第一目标“把 UI 和核心代码分离”已经接近一个比较稳定的形态：
  - `CoreState`：业务内核
  - `DesktopAppState`：桌面 Adapter 运行态
  - `UiState`：草稿值容器
  - `model::host_draft`：转换逻辑
- 现在明显还留在 `AppState` 里的，大多已经不是纯 UI 状态，而是：
  - 主题导入/导出/复制
  - 存储备份/导入/恢复
  - 片段运行期上下文
  - 从主机命令草稿保存片段

## 本轮验证

- `cargo check` 通过。
- `cargo check --tests` 通过。

## 本轮进度（删除确认与创建弹窗状态继续旁路）

- 继续收口纯桌面运行态，把以下消息直接在 `DesktopAppState` 本地处理，不再走 `AppState` 过渡分发：
  - `OpenCommandPalette`
  - `UpdateCommandPaletteQuery`
  - `CloseCommandPalette`
  - `RequestRemoveHost`
  - `CancelRemoveHost`
  - `RequestRemoveGroup`
  - `CancelRemoveGroup`
  - `OpenCreateHostDialog`
  - `CloseCreateHostDialog`
  - `OpenCreateHostDialogInGroup`
  - `OpenCreateGroupParentDialog`
  - `SelectCreateGroupParent`
  - `CloseCreateGroupParentDialog`
  - `ConfirmCreateGroupParent`
  - `OpenCreateGroupDialog`
  - `UpdateQuickGroupName`
  - `SelectQuickGroupParent`
  - `CloseCreateGroupDialog`
- 当前效果：
  - 命令面板状态已经完全归属于桌面 Adapter
  - 删除确认的 `pending_delete_*` 状态不再需要通过 `AppState`
  - 主机/分组创建流程中的弹窗开关和父级选择也开始集中到 `DesktopAppState`

## 当前评估

- 到这一轮，`AppState` 中最“像桌面状态”的那一层已经被持续抽空。
- 现在剩余还明显需要进一步收口的主要是：
  - 主题/背景切换偏好与设置页交互
  - 少量 still-through-`AppState` 的 UI 草稿更新路径
  - 运行期上下文相关动作（比如片段运行、从主机命令草稿保存片段）
- 第一目标“把 UI 和核心代码分离”已经非常接近一个清晰的双层结构：
  - `CoreState`：业务和持久化
  - `DesktopAppState + UiState`：纯桌面运行态与草稿

## 本轮验证

- `cargo check` 通过。
- `cargo check --tests` 通过。

## 本轮进度（纯桌面工作区状态从 AppState 旁路）

- 继续沿“先分离 UI 和核心代码”推进，把一批最纯的工作区 UI 运行态从 `AppState` 路由里旁路掉，直接在 `DesktopAppState` 本地处理：
  - `DismissUiError`
  - `SetWorkspacePage`
  - `NavigateWorkspacePage`
  - `ToggleHostListMode`
  - `ToggleHostTreeGroup`
  - `ToggleCredentialTreeNode`
  - `ToggleSnippetTreeNode`
  - `UpdateHostSearchQuery`
  - `UpdateCredentialSearchQuery`
  - `UpdateSnippetSearchQuery`
  - `UpdateNetworkSearchQuery`
  - `UpdateNewSessionSearchQuery`
  - `ResizeHostsPanel`
  - `ResizeActivityPanel`
  - `ResizeToolPanel`
  - `OpenToolPanel`
  - `CloseToolPanel`
  - `ToggleRightSidebar`
- 这批消息现在不再需要临时组装 `AppState` 才能运行。
- 这一步的实际效果：
  - `AppState` 不再承担那么多纯桌面布局/筛选/页面状态变更
  - `DesktopAppState` 开始真正像“桌面 Adapter 状态容器”
  - `CoreState` 只保留业务和持久化相关职责

## 当前评估

- 目前分层已经更接近最终形态：
  - `CoreState`：业务状态、后端、持久化、核心动作
  - `DesktopAppState`：页面状态、面板、搜索、删除确认、quick-host 回填等纯桌面运行态
  - `UiState`：值存储
  - `model::host_draft`：草稿和值对象转换
- 剩余明显还可继续收口到桌面层的，主要是：
  - 命令面板相关状态
  - 主题/背景切换 UI 偏好
  - 主机/分组创建弹窗开关与 pending 确认状态

## 本轮验证

- `cargo check` 通过。
- `cargo check --tests` 通过。

## 本轮进度（片段维护类下沉到核心）

- 继续沿“先分离 UI 和核心代码”推进，把片段维护类逻辑从 `AppState` 下沉到 `CoreState`：
  - `create_snippet`
  - `update_snippet`
  - `create_snippet_group`
  - `rename_snippet_group`
  - `remove_snippet_group`
  - `remove_snippet_group_recursive`
  - `move_snippet_group`
  - `move_snippet`
  - `remove_snippet`
  - `create_snippet_targets`
  - `update_snippet_target`
  - `sync_snippet_target_implementation_targets`
  - `remove_snippet_target`
  - `split_snippet_target_implementation`
  - `update_snippet_argument`
- `dispatch_snippet_message` 已拆成两段：
  - 片段维护 / 目标变体 / 分组调整这批消息直接走 `CoreState`
  - 片段运行、历史重放、从主机命令草稿保存片段这批仍留在桌面过渡层
- `DesktopAppState::apply_message()` 也已同步把这批片段维护消息直通到核心

## 当前评估

- 现在片段模块的分界也更清楚了：
  - 核心：片段模型维护、目标变体、分组、变量记录
  - UI/桌面：从主机命令草稿保存片段、运行期上下文、活动终端选择
- 到这一轮为止，核心已经覆盖：
  - 会话 / 后端 / SFTP
  - 网络资源 / known hosts / 大部分凭据
  - 主机持久化动作
  - 片段维护动作
- 当前最主要的剩余 UI 区域，已经比较集中：
  - 工作区页面状态 / 面板开关 / 筛选
  - 删除确认这类 pending UI 状态
  - 片段运行期上下文
  - 少量从 UI 草稿读取再触发核心动作的流程

## 本轮验证

- `cargo check` 通过。
- `cargo check --tests` 通过。

## 本轮进度（QuickHostDraft 与 Host 转换抽离）

- 新增独立模块：
  - `src/model/host_draft.rs`
- 已从 `ui_state/quick_host` 中抽出的业务转换：
  - `quick_host_draft_from_host(...)`
  - `build_host_from_draft(...)`
  - `build_quick_host_auth(...)`
  - 标签解析 / icon 归一化 / 名称截断 / auth draft 转 `AuthProfile`
- 当前结构变化：
  - `UiState.quick_host` 现在更接近“纯值”
  - `ui_state/quick_host/draft.rs` 只保留数据类型和对新模块的薄转发
  - `ui_state/quick_host/auth.rs` 只保留认证方式显示标签，不再持有业务转换
- 主机保存链路现在进一步清晰：
  - UI/桌面层：
    - 保存 quick-host 草稿值
    - 调用 `QuickHostDraft::build_host_with_existing(...)`
    - 关闭弹窗、清草稿
  - 核心层：
    - `save_host_record(...)`
    - `duplicate_host_record(...)`
- 这一步的实际意义：
  - 主机草稿与领域主机之间的转换逻辑，不再埋在 `UiState` 目录里
  - 未来换 UI 时，这套转换可以直接复用，不需要跟 Slint 或当前桌面状态结构耦合

## 当前评估

- 到目前为止，“UI 和核心代码分离”的第一目标已经形成比较清楚的层次：
  - `CoreState`：会话、SFTP、网络资源、凭据、主机持久化
  - `model::host_draft`：主机草稿和值对象转换
  - `UiState`：输入框、弹窗、页面、筛选等纯桌面运行态
- 剩下还明显偏 UI 的主要区域：
  - 打开编辑主机弹窗时的 `Host -> Draft` 装配调用点
  - 片段编辑与目标变体草稿
  - 设置页和工作区页面状态

## 本轮验证

- `cargo check` 通过。
- `cargo check --tests` 通过。

## 本轮进度（主机保存核心动作下沉）

- 继续沿“先分离 UI 和核心代码”推进，在 quick-host 路径里把真正的主机保存动作下沉到核心：
  - `CoreState::save_host_record(...)`
  - `CoreState::duplicate_host_record(...)`
- 当前分层已经变成：
  - `UiState.quick_host` 继续负责表单草稿
  - `QuickHostDraft::build_host_with_existing(...)` 继续负责把草稿转换成 `Host`
  - `CoreState::save_host_record(...)` 负责：
    - 编辑对象是否仍存在
    - 最终 `storage.upsert_host(...)`
  - `CoreState::duplicate_host_record(...)` 负责复制主机本体
- `save_quick_host()` 现在不再自己直接写 `storage.upsert_host(...)`，而是把构造好的 `Host` 交给核心保存。
- `duplicate_host()` 也不再在 UI 草稿模块里直接改存储，而是调用核心动作。

## 当前评估

- 主机模块这一步虽然还没有把 `SaveQuickHost` 从 UI 消息域移走，但真正的“保存主机记录”已经是核心职责。
- 也就是说，主机创建/编辑现在已经拆成：
  - UI：草稿输入、弹窗开关、表单转 `Host`
  - 核心：主机持久化与编辑存在性校验
- 目前还留在 UI 侧的主机耦合主要是：
  - quick-host 草稿本身
  - 创建/编辑/分组弹窗开关
  - 打开编辑弹窗时从已保存主机生成草稿

## 本轮验证

- `cargo check` 通过。
- `cargo check --tests` 通过。

## 本轮进度（凭据创建/导入/生成与 quick-host 回填分离）

- 继续围绕“先分离 UI 和核心代码”推进，把最后一批凭据创建类动作拆成：
  - `CoreState` 负责真正的创建 / 导入 / 生成
  - `DesktopAppState` 负责成功后回填 quick-host 草稿
- 当前已变成核心动作接口的凭据创建类操作：
  - `create_credential_metadata_action`
  - `generate_private_key_credential_action`
  - `save_password_credential_action`
  - `import_private_key_credential_action`
  - `import_private_key_text_credential_action`
  - `import_certificate_credential_action`
  - `import_certificate_text_credential_action`
  - `generate_certificate_credential_action`
- `DesktopAppState::apply_message()` 现在对这批消息采用统一模式：
  - 先调用 `CoreState` 动作
  - 成功后按凭据类型把 `secret_ref` 回填到 `ui.quick_host.auth`
- 这意味着：
  - 核心不再直接写 quick-host 草稿
  - UI 副作用被集中到桌面 Adapter，而不是散落在 Storage 业务实现里
- `dispatch_storage_message` 已把这批凭据创建类消息一并纳入核心直通

## 当前评估

- 到这一步，Storage 里的大多数业务逻辑已经从 UI 草稿中剥离出来。
- 现在核心/桌面职责分界更明确：
  - `CoreState`：凭据、网络资源、known hosts、SFTP、会话、后端队列
  - `DesktopAppState`：页面切换、quick-host 回填、删除确认、输入框草稿、筛选状态
- 目前还没有完全拆开的，主要就剩：
  - 主机创建/编辑弹窗草稿
  - 片段编辑/目标变体/参数草稿
  - 设置页和工作区级页面状态

## 本轮验证

- `cargo check` 通过。
- `cargo check --tests` 通过。

## 本轮进度（Storage 纯保存类下沉）

- 继续拆 Storage，把不依赖 quick-host 回填和确认弹窗草稿的纯保存类下沉到 `CoreState`：
  - `trust_known_host`
  - `remove_known_host`
  - `save_proxy_asset`
  - `save_jump_chain_asset`
  - `save_forward_asset`
  - `remove_proxy_asset`
  - `remove_jump_chain_asset`
  - `remove_forward_asset`
- `dispatch_storage_message` 已拆成两段：
  - 上面这批纯保存类消息直接在 `CoreState` 中处理
  - 其余仍依赖待确认状态或 quick-host 回填的凭据类消息继续留在桌面过渡层
- `DesktopAppState::apply_message()` 已同步扩展核心直通：
  - `SaveProxyAsset`
  - `SaveJumpChainAsset`
  - `SaveForwardAsset`
  - `RemoveProxyAsset`
  - `RemoveJumpChainAsset`
  - `RemoveForwardAsset`
  - `TrustKnownHost`
  - `RemoveKnownHost`
- 配套清理：
  - `credential_refs::next_secret_ref(...)` 已改为依赖 `CoreState`
  - 删除迁移后不再被生产路径使用的 `AppState` 包装

## 当前评估

- 现在 `CoreState` 已经可以独立承接三大类核心动作：
  - 会话和后端事件
  - SFTP 浏览/部分传输动作
  - 网络资源和 known hosts 持久化
- 仍未拆完的主要阻塞点：
  - 凭据生成/导入/更新后回填 quick-host 的那批 Storage 分支
  - 主机创建/编辑、片段编辑、设置页等大量弹窗草稿消息

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。

## 本轮进度（SFTP 草稿读取与核心执行分离）

- 继续把 SFTP 草稿依赖拆成“桌面读草稿，核心做事”：
  - `upload_sftp` 现在只负责从 `UiState` 读取：
    - 本地路径
    - 远程文件名草稿
  - 真正的上传执行已经落到 `CoreState::upload_sftp_with_paths(...)`
- 下载同样拆开：
  - `download_sftp` 只负责从 `UiState` 读取本地路径草稿和默认值推断
  - 真正的下载执行已经落到 `CoreState::download_sftp_to_path(...)`
- 创建目录同样拆开：
  - `create_sftp_dir` 只负责从 `UiState` 读取新目录名草稿
  - 真正的创建动作已经落到 `CoreState::create_sftp_dir_named(...)`
- 继续扩大核心直通 SFTP 消息：
  - `CancelSftpTransfer`
  - `RemoveSftpFile`
  这两类消息现在也直接进入 `CoreState`
- 当前 SFTP 分层状态：
  - 纯核心已直通：
    - 刷新
    - 导航
    - 选择条目
    - 书签保存/打开/删除
    - 删除文件
    - 取消传输
  - 仍在桌面层读草稿，但核心已参数化：
    - 上传
    - 下载
    - 创建目录

## 当前评估

- `CoreState` 现在已经不只是“能跑消息”，而是开始具备可复用的参数化动作接口。
- 这对以后换 UI 很关键：新 UI 不需要复制上传/下载/创建目录逻辑，只需要把自己的表单值传给核心。
- 还没拆掉的主要是：
  - 凭据导入/生成后回填 quick host 的 Storage 分支
  - 片段/主机/设置页的大量弹窗与筛选草稿消息

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。

## 本轮进度（Network 参数补齐第一轮）

- 核心模型已补齐网络资源常用参数：
  - `ProxyProfile` 增加 `auth`、`remote_dns`。
  - `ProxyAuth` 新增 `None`、`UserPassword`。
  - `JumpProfile` 增加 `username_override`、`port_override`、`alias`。
  - `JumpChainAsset` 增加 `stop_on_failure`。
  - `TunnelRule` 增加 `exit_on_failure`。
  - `ForwardAsset` 增加 `exit_on_failure`。
- SQLite schema / entity / mapper 已同步：
  - `proxy_assets`、`host_proxy` 增加认证与 `remote_dns` 列。
  - `jump_chain_steps` 增加跳板步骤覆盖字段。
  - `forward_assets`、`tunnel_rules` 增加 `exit_on_failure` 列。
- AppState / Message / 回调链路已同步：
  - `SaveProxyAsset` 增加认证与 `remote_dns` 参数。
  - `SaveForwardAsset` 增加 `exit_on_failure` 参数。
- Network 页 UI 已接入第一版编辑能力：
  - 代理弹窗支持无认证 / 用户名密码、密码引用、SOCKS5 远端 DNS。
  - 转发弹窗支持 `exit_on_failure` 开关。
  - 详情区可展示代理认证摘要和转发失败策略摘要。
- i18n 已补：
  - 新增代理认证、远端 DNS、转发失败策略相关中英文 key。
- 本轮验证：
  - `cargo fmt` 通过。
  - `cargo check` 通过。
  - `cargo check --tests` 通过。
  - `cargo test -p smagical-core` 通过。
  - `cargo test -p smagical-storage network_assets --config profile.test.debug=0` 通过。
- 当前剩余：
  - 主 crate 精确单测在 Windows 上会受 `link.exe` / PDB 体量影响，出现 `LNK1140` 或超时；代码层面已经能通过 `cargo check --tests`。
  - 跳板链“逐节点高级编辑”目前只有核心模型和落库，UI 还没做节点级表单。
  - 代理认证目前保存的是 `SecretRef` 文本引用，尚未接凭据选择器。

## 本轮进度（Network 页面去 tab 与布局重构）

- 页面定位调整：
  - Network 页不再显示工作区 `WorkspaceTabStrip`，该 tab 条只保留给主机/终端/SFTP 等会话类页面。
  - Network 页按配置中心处理，不再混入终端会话导航。
- Network 页面布局重构：
  - 左侧改成资源树/资源栏：运行中、跳板链、代理池、端口转发分区展示。
  - 每个分区显示数量，列表项改成更像文件树的行式结构，减少卡片堆叠感。
  - 左侧保留搜索和新建代理/跳板/转发入口。
  - 右侧改成详情页结构：头部身份区、操作区、统计、备注、字段、使用位置分块。
- i18n/主题收敛：
  - 右侧详情标题复用已有 `proxy.resources_section`、`proxy.field_note`、`proxy.field_used_by` 等配置文案。
  - 没有新增硬编码展示文字。
- 参考方向：
  - MobaXterm 将 SSH tunnels 作为独立工具入口管理。
  - Bitvise SSH Client 将端口转发作为可保存配置/profile 的一部分。
  - OpenSSH 本身也把 `ProxyJump`、`LocalForward`、`RemoteForward`、`DynamicForward` 作为网络连接配置能力，而不是终端 tab。
- 本轮验证：
  - `cargo fmt` 通过。
  - `cargo check --tests --color never` 通过。
  - `cargo test -p smagical-storage --color never network_assets` 通过。
  - `cargo test -p smagical-core --color never` 通过。
  - `cargo build --color never` 通过。
- 预览：
  - 已重启预览进程，当前 `smagicalssh` PID `28316`。
  - `cargo build --color never` 通过。
- 预览：
  - 已重启预览进程，当前 `smagicalssh` PID `19584`。
  - `cargo build --color never` 通过。

## 下一步

- 再检查主机页里选择代理 / 跳板 / 转发的联动。
- 检查 Network 页删除/编辑后选中态是否需要自动刷新或清空。
- 继续把剩余硬编码文案收敛到 i18n，并预览 Network 页实际 UI。

## 归档计划（Network Page Redesign Plan）

### Goal

将当前 Network 页面从“纵向堆叠四类列表”重做成与 Security / Snippets 风格一致的双栏工作区：左侧分组导航，右侧选中项详情卡。

### Why

当前页面虽然已经接入真实数据，但仍存在几个问题：

- 运行态、模板、资产、主机内联路由混在同一视觉层级
- 详情信息只能靠 subtitle / meta 猜语义
- 页面缺少稳定的焦点区和真实空态
- 一些文案仍带临时性质，如 `live`、`Host Routes`、`direct`、`{n} hops`

### Constraints

- `Host` 仍保存内联 `proxies: Vec<ProxyProfile>` 和 `jumps: Vec<JumpProfile>`
- `ProxyAsset` / `JumpChainAsset` 已存在，但主机尚未通过 ID 引用它们
- `TunnelRule` 不带 `host_id`
- `Message::StartTunnel` / `AppState::start_tunnel` 必须显式传入 `host_id`
- 因此：已保存隧道模板不能被设计成可一键启动的已绑定规则

### Information Architecture

左侧分组：

- Runtime Tunnels
- Saved Tunnel Templates
- Proxy Assets
- Route Definitions

其中 Route Definitions 下再区分：

- Jump Chain Assets
- Host Inline Routes

右侧详情按选中类型切换：

- Runtime Tunnel：状态、关联主机、endpoint、启动时间/运行中、最近错误；动作仅 `Stop`
- Saved Tunnel Template：类型、endpoint、auto_start、需先选宿主主机的说明；无直接启动动作
- Proxy Asset：协议、host:port、tags、说明这是可复用资产
- Jump Chain Asset：hop 数量与顺序
- Host Inline Route：主机名、jump/proxy 摘要、tags、说明这是 host 内联定义

### Files

优先修改：

- `src/app/view_model/root/types.rs`
- `src/app/view_model/root/builder.rs`
- `src/app/view_model/network.rs`
- `src/app/projection/collections.rs`
- `src/app/projection/workspace.rs`
- `src/app/projection/models/tools_common.rs`
- `ui/main_workspace.slint`
- `ui/models.slint`
- `ui/workspace_network_page.slint`
- `ui/workspace_common.slint`

如继续保留停止运行态隧道动作：

- `src/app/callbacks/workspace.rs`
- `src/model/app_state/launch_tunnel/stop.rs`

### Verification

- `cargo fmt`
- `cargo check --color never`
- `cargo test --color never`
- 手动验证：
  - Network 页已切成双栏工作区
  - 左侧分组可区分 runtime tunnel、saved template、proxy asset、route definitions
  - 右侧详情会随选中对象类型切换
  - Runtime tunnel 可停止
  - Saved tunnel template 不会出现误导性启动按钮
  - 页面内不再保留 `live`、`Host Routes`、`direct`、`{n} hops` 这类临时硬编码文案

## 本轮进度（Network 多轮优化）

- 修复 Network 页 Slint 编译错误：
  - `NetworkWorkspacePage` 不再直接修改 `in property`。
  - 清空右侧选中态改为子页面发 `clear-selection`，父级 `MainWorkspace` 统一清理 `network-selected-*` 临时 UI 状态。
- 收敛 Network 页状态刷新：
  - 搜索时先清空右侧详情，再更新 `network_search_query`。
  - 保存代理、跳板链、转发成功后清空旧详情，避免继续显示过期内容。
  - 删除网络资源改为返回 `bool`，只有删除成功才清空详情；如果资源仍被主机引用而删除失败，会保留当前详情并显示错误。
- 拆分 Network 展示模型：
  - `NetworkWorkspaceViewModel` 从单一 `resources` 拆成 `proxy_assets`、`jump_chain_assets`、`forward_assets`。
  - projection 分别写入 `network-proxy-assets`、`network-saved-tunnels`、`network-route-definitions`。
  - UI 分区语义修正为：运行中、跳板链、代理池、端口转发。
- 当前验证：
  - `cargo fmt` 通过。
  - `cargo check --tests --color never` 通过。
  - `cargo test -p smagical-storage --color never network_assets` 通过。
  - `cargo test -p smagical-core --color never` 通过。
- 已知验证限制：
  - 根 crate 聚焦测试 `cargo test --color never app_view_model_projects_network_workspace_items` 仍触发 Windows `link.exe LNK1140 / PDB` 限制。
  - 尝试用 `RUSTFLAGS='-C debuginfo=0'` 后遇到 `target/debug/.cargo-lock` / 备用 target 目录 `os error 5`，未能完成根 crate 单测运行；当前以 `cargo check --tests` 和子 crate 测试作为基线。

## 下一步建议

- 先启动预览，手动检查 Network 页四个分区、搜索、编辑、删除失败提示和跳板链主机选择弹窗。
- 如果 UI 可接受，继续补 Network 页主机引用联动：从主机编辑页选择代理 / 跳板 / 转发后，在 Network 页详情里确认“使用位置”即时正确。
- 再做一次轻量清理：把 Network 页中 `saved-tunnels` 这类历史命名改成更准确的 `jump-chain-assets`，减少后续维护误解。

## 本轮进度（Network 继续多轮优化）

- 命名收敛：
  - Slint 层 `network-saved-tunnels` / `saved-tunnels` 已改为 `network-jump-chain-assets` / `jump-chain-assets`。
  - projection 现在调用 `set_network_jump_chain_assets`，与 `NetworkWorkspaceViewModel.jump_chain_assets` 对齐。
  - 代码层不再保留 `saved-tunnels` 旧命名。
- 搜索空态优化：
  - 新增 i18n 文案 `proxy.search_empty`。
  - Network 页搜索词不为空且运行中、跳板链、代理、转发四个列表都为空时，显示统一的“没有匹配的网络资源”。
  - 正常无数据时仍保留各分区空态，不影响初始页面。
- 弹窗互斥：
  - Network 页新增 `close-dialogs()`。
  - 新建代理、跳板、转发、编辑、删除前会先关闭其他 Network 弹窗，避免叠层状态残留。
- 本轮验证：
  - `cargo fmt` 通过。
  - `cargo check --tests --color never` 通过。
  - `cargo test -p smagical-storage --color never network_assets` 通过。
  - `cargo test -p smagical-core --color never` 通过。

## 下一步建议

- 启动预览，重点检查 Network 页搜索无结果空态和四分区标题是否正确。
- 如果 UI 通过，继续做主机页与 Network 页的引用联动验证和小修：主机编辑选择资源后，Network 详情“使用位置”应即时变化。
- 后续可继续把 Network 页弹窗里的主机选择列表加搜索或按分组展示，提升跳板链选择效率。

## 本轮进度（Network 命名和跳板选择优化）

- 命名继续收敛：
  - Slint 层 `network-route-definitions` / `route-definitions` 已改为 `network-forward-assets` / `forward-assets`。
  - projection 现在调用 `set_network_forward_assets`，与 `NetworkWorkspaceViewModel.forward_assets` 对齐。
  - 代码层不再保留 `route-definitions` 旧命名。
- 跳板链主机选择弹窗新增搜索：
  - 弹窗内增加搜索输入框，复用 `HostSearchField`。
  - 新增 pure callback `network-jump-host-matches(query, host)`。
  - Rust helper `host_row_matches_query` 按主机名、endpoint、认证方式、分组、标签、状态匹配。
  - 已补 helper 单测覆盖空查询、名称、endpoint、分组和不匹配场景。
- 本轮验证：
  - `cargo fmt` 通过。
  - `cargo check --tests --color never` 通过。
  - `cargo test -p smagical-storage --color never network_assets` 通过。
  - `cargo test -p smagical-core --color never` 通过。

## 下一步建议

- 启动预览检查跳板链弹窗搜索是否顺滑，以及搜索后已选主机序号是否仍正确。
- 如果 UI 可接受，继续做跳板链弹窗按主机分组展示，或给已选主机增加“已选列表/清空”入口。
- 继续做 Network 页和主机编辑页联动的手动预览验证。

## 本轮进度（Network 跳板选择继续优化）

- 跳板链主机选择弹窗继续增强：
  - 搜索后没有匹配主机时显示统一空态。
  - 增加已选主机数量显示。
  - 增加“清空”按钮，可一次清空跳板链已选主机。
- i18n 和回调链路：
  - 新增 `proxy.clear_selection` 文案，中文为“清空”，英文为 `Clear`。
  - 新增 `has-network-jump-host-match(query)`，从当前 Slint `hosts` model 中判断搜索是否有匹配。
  - 新增 `selected-network-jump-host-count(host-ids)` 和 `clear-network-jump-hosts()`。
  - `host_id_count` 已补单测。
- 本轮验证：
  - `cargo fmt` 通过。
  - `cargo check --tests --color never` 通过。
  - `cargo test -p smagical-storage --color never network_assets` 通过。
  - `cargo test -p smagical-core --color never` 通过。

## 下一步建议

- 启动预览重点检查：搜索无匹配提示、已选数量、清空按钮、已选序号是否一致。
- 如果交互可接受，再做分组展示或“已选主机置顶”，进一步提升长列表选择效率。

## 本轮进度（Network 弹窗收敛与预览）

- Slint 内部结构继续收敛：
  - `NetworkWorkspacePage` 新增 `reset-draft()`。
  - 新增 `open-new-proxy-dialog()`、`open-new-jump-dialog()`、`open-new-forward-dialog()`、`open-edit-dialog()`。
  - 新建和编辑入口不再重复手写 draft 初始化逻辑。
- 跳板链弹窗交互继续打磨：
  - `NetworkJumpDialog` 新增 `selected-count` 派生属性。
  - “清空”按钮在未选择主机时禁用。
  - 关闭、取消、保存成功时会清空弹窗内搜索词，避免下次打开残留筛选。
- 本轮验证：
  - `cargo fmt` 通过。
  - `cargo check --tests --color never` 通过。
  - `cargo test -p smagical-storage --color never network_assets` 通过。
  - `cargo test -p smagical-core --color never` 通过。
- 预览：
  - 已执行 `cargo run -- --seed-preview-data` 生成预览数据。
  - 已直接启动 `target/debug/smagicalssh.exe`。
  - 当前预览进程：`smagicalssh`，PID `9492`。

## 本轮进度（错误提示 toast 收敛）

- 错误提示改为右上角 toast：
  - 取消了右下/底部横幅式错误提示。
  - `last-error` 统一作为 toast 内容来源，不再维护并行状态。
  - toast 默认右上角显示，带手动关闭按钮。
  - 3.2 秒后自动淡出，随后自动清理错误状态。
- Slint 结构收敛：
  - `MainWorkspace` 只负责展示 toast。
  - 自动关闭逻辑放到 `main.slint` 顶层 `Timer`。
  - 修掉了两处 Slint 语法问题：`Rectangle` 不能用 `vertical-alignment`，以及组件内不能直接引用 `parent.width`。
- 本轮验证：
  - `cargo fmt` 通过。
  - `cargo check --tests --color never` 通过。
  - `cargo test -p smagical-storage --color never network_assets` 通过。
  - `cargo test -p smagical-core --color never` 通过。

## 本轮进度（片段执行链继续下沉到核心）

- 继续沿“先把 UI 和核心代码分离”推进，这一轮把片段执行链再往下切了一层：
  - `CoreState::run_snippet_action(...)`
  - `CoreState::run_snippet_with_arguments_action(...)`
  - `CoreState::run_snippet_target_action(...)`
  - `CoreState::run_snippet_target_with_arguments_action(...)`
- 这几条路径现在由核心直接负责：
  - 查找主机和片段
  - 校验片段作用域是否适用于主机
  - 过滤无效参数
  - 选择默认实现或目标实现
  - 渲染命令模板
  - 回写最近一次参数
  - 最终调用 `run_remote_command(...)`
- `src/model/app_state/snippets/run.rs` 也顺手做了内部收敛：
  - 抽出 `validated_host_snippet(...)`
  - 抽出 `filtered_snippet_arguments(...)`
  - 抽出 `rendered_snippet_command(...)`
  - 抽出 `snippet_implementation_for_target(...)`
  这样片段执行规则不再散落重复分支。
- `dispatch/snippets.rs` 已同步收口：
  - `RunSnippet`
  - `RunSnippetWithArguments`
  - `RunSnippetTargetWithArguments`
  - `RunCommandHistory`
  这些消息现在可以直接进入 `CoreState`，不再被统一判定为“必须依赖桌面运行期上下文”。
- `DesktopAppState` 这一轮也修正了一个更关键的结构问题：
  - `apply_messages()` 之前整批消息仍旧回退到 `AppState.apply(...)`
  - 现在改为逐条走 `DesktopAppState::apply_message(...)`
  - 这意味着前面已经拆好的核心直通分支终于真正生效，而不是被批量入口重新绕回旧组合器
- 桌面层本轮保留的职责更清晰了：
  - 读取“当前活动远程 tab 对应的 host_id”
  - 片段/历史执行成功后切到 `WorkspacePage::Terminal`
  - 把错误文本同步到 `ui.last_error`
- 当前这一轮后的分层状态：
  - `CoreState`：片段执行规则、远程命令启动、命令历史重放
  - `DesktopAppState`：活动上下文、页面切换、错误 UI 出口
  - `AppState`：仍保留兼容包装，但片段执行这条主路径已经明显变薄

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- 尝试运行根 crate 定向测试：
  - `cargo test run_snippet_on_active_host_uses_current_remote_tab --lib`
  - `cargo test run_command_history_reuses_recorded_host_and_command --lib`
  - `cargo test run_snippet_with_arguments_records_arguments_and_runs --lib`
  仍然在 Windows MSVC 链接阶段撞到 `LNK1140 / PDB` 限制，这个限制和本轮逻辑无关，当前仍以 `cargo check` / `cargo check --tests` 作为稳定验证基线。
- 中途出现过 `target/debug/.cargo-lock` 被占用，原因是超时后的 `cargo/rustc` 构建进程残留；随后已自行退出，之后 `cargo check` 与 `cargo check --tests` 再次通过。

## 本轮进度（设置页动作开始从 AppState 下沉）

- 继续沿“核心单独运行、未来可换 UI”这条线推进，这一轮先处理设置页里最成片的一批副作用：
  - `ExportCurrentTheme`
  - `CopyCurrentBuiltInTheme`
  - `ImportTheme`
  - `ApplyThemeProfile`
  - `RemoveThemeProfile`
  - `BackupStorage`
  - `ExportStorageSnapshot`
  - `ImportStorageSnapshot`
  - `ImportSqliteBackup`
- `src/model/app_state/settings.rs` 已重构为两层：
  - `CoreState::*_action(...)` 负责真正的主题资料和存储 IO 动作
  - `AppState` 只保留兼容包装和 UI 回填
- 当前这批设置动作里，核心层已经直接承接：
  - 当前主题文档解析与导出
  - 内置主题复制为资料
  - 主题文件导入
  - 已保存主题资料应用与删除
  - SQLite 备份 / 快照导出
  - 快照导入 / SQLite 备份导入后重载核心存储
- 桌面层当前只保留必要的 UI 同步：
  - `ui.visual_settings` 从 `core.config.theme/background` 回填
  - 存储导入后把 `host_list_mode / language / built_in_theme` 同步回 `ui.workspace`
- `DesktopAppState::apply_message()` 也已经开始直通这批设置消息，不再统一回退旧 `AppState.apply(...)`。

## 当前评估

- 这一步的意义不是“把设置逻辑搬了个文件”，而是继续建立稳定 seam：
  - `CoreState` 负责真实设置动作和 IO
  - `DesktopAppState` 负责 UI 草稿同步
  - `AppState` 继续变薄为兼容层
- 现在剩余还明显卡在旧 `AppState` 过渡层里的，主要是：
  - 仍依赖大量草稿读写的 quick-host / terminal input / 局部工作区 UI 路由
  - 少量设置页展示偏好和旧 workspace_ui 包装

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。

## 本轮进度（核心入口测试和核心门面收窄）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 已更新 `CoreState` 的说明：
  - `CoreState` 不再描述为可以和旧 `AppState` 互转。
  - 明确当前桌面 UI 是 Adapter，核心不反向依赖具体界面状态。
- 已删除旧迁移口：
  - `AppState::from_core_state(...)`
  - `AppState::into_core_state(...)`
  这两个方法已经没有调用者，保留只会诱导后续重新把核心塞回旧 UI 组合状态。
- 已修正核心测试语义：
  - `CoreState::apply(...)` 直接执行核心消息。
  - UI 草稿消息会被核心拒绝，而不是偷偷构造 UI 状态执行。
- 已收窄 `core` 门面：
  - 移除 `pub use crate::model`。
  - 避免通过 `core::model` 把 `AppState` / `UiState` 重新暴露到核心门面。
  - 新 UI 应直接使用 `CoreState` 与顶层 `model::Message`，不从核心门面间接拿 UI 类型。
- 已修正文档旧表述：
  - `src/lib.rs` 从 `core::model::AppState` 改为 `core::CoreState` + `model::Message`。
  - `src/model.rs` 移除旧的 Iced 表述。
  - `src/model/app_state/state.rs` 明确 `AppState` 是过渡兼容状态，当前桌面主路径已经使用 `DesktopAppState`。

## 当前评估

- 核心和 UI 的方向进一步明确：
  - `CoreState` 不再依赖 `AppState`。
  - 当前桌面 Adapter 不再通过旧 `AppState.apply(...)` 兜底。
  - `core` 门面不再重新导出包含 UI 草稿的整个 `model` 模块。
- 旧 `AppState` 仍存在，主要价值是旧测试和迁移期兼容。下一步应继续把旧 `AppState` 中已经有核心/桌面替代实现的包装减少，或者把测试逐步迁到 `CoreState` / `DesktopAppState` 的真实入口。

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
- `cargo test core_state_apply --lib` 仍在 Windows MSVC 链接阶段报 `LNK1140 / PDB` 限制；测试代码已通过 `cargo check --tests` 编译检查，这个链接限制和本轮逻辑无关。

## 本轮进度（桌面 bootstrap 脱离旧 AppState）

- 继续推进第一目标：核心单独运行，未来换 UI 也能继续运行。
- 已把当前 Slint 桌面启动路径从旧 `AppState` 上拆开：
  - `src/app/bootstrap.rs::boot_state()` 现在直接返回 `DesktopAppState`。
  - 启动时直接构造 `CoreState + UiState`，不再先 `AppState::default()` 再转换。
  - `src/app.rs` 直接使用 `bootstrap::boot_state()` 初始化共享状态。
  - `DesktopAppState::new(AppState)` 已删除。
- 加载 SQLite 存储后的 UI 偏好同步现在由 desktop bootstrap 内部完成：
  - `host_list_mode`
  - `language`
  - `built_in_theme`
  这些仍属于当前桌面 Adapter 的 UI 运行态，不进入 `CoreState`。
- 默认本地终端初始化仍写入核心的 session/terminal/backend queue：
  - session 和 terminal tab 是核心运行态。
  - 是否在启动时默认创建本地终端属于当前桌面启动策略。
- 已清理 app 层旧注释：
  - 文件选择、设置回调、导航、projection 不再写“核心 AppState”。
  - 文档统一指向 `CoreState` 或桌面 `UiState`/Adapter。

## 当前评估

- 当前生产路径进一步清晰：
  - 启动：`bootstrap` 直接产出 `DesktopAppState`。
  - 核心：`CoreState` 持有配置、存储、会话、终端、后端队列。
  - 桌面：`UiState` 持有输入框、弹窗、筛选、页面等 UI 草稿。
  - 旧 `AppState` 不再参与当前 Slint 桌面启动和消息分发主路径。
- 下一步更大的分离点是 Cargo/构建层：
  - 当前根 crate 仍总是编译 `app`/Slint，因为 `build.rs` 总是运行 `slint_build::compile`。
  - 要做到真正“核心 crate 可无 Slint 编译”，需要继续设计 feature 或拆出新的 app-core crate；这个改动面比本轮大，需要单独做。

## 本轮验证

- `cargo fmt` 通过。
- `cargo check` 通过。
- `cargo check --tests` 通过。
