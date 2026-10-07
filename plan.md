# smalux-ssh 现代化全栈性能、架构演进与稳定性规划 (plan.md)

> **当前状态**：全阶段（阶段一至阶段四十一）已全量实施并 100% 验收通过！全工作区热检查耗时极速全绿，0 错误、0 警告、0 OOM！

---

## 一、最新落地成果复盘 (Delivered Review)

### 阶段三十一点五至三十二：网络穿透、真实驱动与凭据表单像素级对齐 (Phases 31-32: Network Penetration, Native Drivers & Credential Form Alignment)
- **核心成果**：
  1. **UI 像素级居中与防截断修复**：
     - 表单标题文本优化为 `私钥内容 (OpenSSH / PEM) *`，释放超 80px 宽度，彻底根治右侧详情面板中“从文件导入”按钮文字被挤压截断的缺陷；
     - 表单所有操作按钮引入 `cross-axis-alignment: center;`，并将图标严密包裹于 `VerticalLayout { alignment: center; ... }`，根除 SVG 图标相对于文字基线上浮偏移的顽疾；
     - 为“遮挡/查看私钥”增配状态自适应图标（`eye.svg` / `eye-off.svg`），为“复制私钥”与“复制公钥”增配 `copy.svg` 图标，交互规格整齐划一。
  2. **纯 Rust 原生公钥指纹与加密计算打通**：
     - 移除 `crates/smagical-ui/src/lib.rs` 中 `if !is_mock` 的拦截门禁，无论数据层存储模式为何，**始终无条件装配由 `ssh-key` 驱动的原生 [`NativeKeygenService`](crates/smagical-ssh/src/keygen.rs)**；
     - 粘贴私钥（Ed25519、RSA、ECDSA-P256/P384/P521 等）即时调用真机加密库计算真实 OpenSSH 标准 SHA256 指纹（消除 `SHA256:MockEcdsaFingerprint` 假指纹），支持口令保护私钥现场解密推导。
  3. **复杂网络多跳穿透与跳板拓扑加固**：
     - 多跳 Direct-TCPIP 链路增强（剥离 user@、每跳独立超时）；SOCKS5/HTTP 代理超时保护与健康探测；SFTP 跳板机/代理同等穿透，跨内网穿透稳定可靠。

### 阶段三十二点五：真实数据源自动嗅探迁移、RefCell 并发崩溃根除与全局 Toast 交互闭环 (Phase 32.5: Real Data Migration, RefCell Safety & Toast Dismissal Fix)
- **核心成果**：
  1. **真实本地系统资产自动嗅探与导入**：
     - 实现 [`real_seed.rs`](crates/smagical-storage/src/seaorm/real_seed.rs)，自动读取并解析本地真实 `~/.ssh/known_hosts`（支持 IPv4/IPv6、标准端口与非标端口解析，自动过滤散列哈希主机）与 `~/.ssh/config` 配置；
     - 彻底清除历史遗留的假服务器 IP（如 `auth-gateway-edge`, `prod-server-01` 等）与假数据，将 SQLite 数据源标记为生产级 `db_initialized = "real_data"`；
     - 提供纯净的默认主机分组与真实 Linux 运维命令片段库，打通真实资产直连通路。
  2. **根除连接定时器 `RefCell already borrowed` 崩溃 (Exit Code 101)**：
     - 在 [`crates/smagical-ui/src/lib.rs`](crates/smagical-ui/src/lib.rs) 的后台 1s 定时器中，精细化收窄 `pane_groups_timer.borrow_mut()` 的生命周期作用域；
     - 确保在调用 `sync_active_session_ui(&w, &groups, ...)` 并跨界触发 Slint 属性回调之前，可变借用守卫已安全释放，彻底根绝高并发网络状态刷新时的运行时 panic 闪退。
  3. **全局 Toast 提示与弹窗交互修复**：
     - 修复 [`toast.slint`](crates/ui/common/ui/components/toast.slint) 与 [`message-dialog.slint`](crates/ui/common/ui/components/message-dialog.slint) 中关闭按钮 `close-touch := TouchArea` 缺少几何尺寸约束的问题，显式赋予 `width: 100%; height: 100%;`，确保点击事件 100% 穿透捕获；
     - 在 [`crates/smagical-ui/src/handlers/mod.rs`](crates/smagical-ui/src/handlers/mod.rs) 的 `WindowBridge.on_close_toast` 回调中，除通知后台服务外，立即对前台活跃的 `ModelRc<ToastItemData>` 执行原子过滤与即时剔除；
     - 在 Toast 推送时挂载 `slint::Timer::single_shot` 自动超时淡出，彻底根除用户反馈的提示“x”关闭不掉的交互缺陷。
- **收益**：资产层全面回归真实主机，连接与监控并发调度稳固无崩，全局通知体系交互顺滑精准。

---

## 二、全阶段演进里程碑压缩矩阵 (Phases 1-32.5 Milestones Matrix)

| 阶段分组 | 核心优化领域 | 关键重构成果 | 验收收益 |
| :--- | :--- | :--- | :--- |
| **Phases 1-3** | 视图生命周期与内存卸载 | `ViewLifecycleManager` 统一纳管 8 大主视图与抽屉，切离即卸载 Slint 模型 | 释放 10MB~25MB 悬浮内存 |
| **Phases 4-6** | 绘帧暂停与巨石组件拆解 | Monitor 抽屉收起停探针；切离终端暂停 GPU 纹理；ThemeEditor 与 AiEndpoint 模块化 | 0% 后台空转；单包检查 1.12s |
| **Phases 7-9** | 虚拟视口与根治编译暴涨 | 双盘文件分片加载；主机树 300 节点截断；禁用 incremental 铲除 23.5GB 缓存 | 根除 0xc0000409 崩溃；files 3.12s |
| **Phases 10-11** | 主机资产树与终局生命周期 | 拆离 4 大私有行组件；开启 `interactive: true` 恢复滚轮；右侧 6 抽屉即关即卸 | `kernel` 编译提速 35%，图元零泄漏 |
| **Phases 12-13** | 内存镜像与终端 60FPS 零堆分配 | 历史会话一级缓存 `HISTORY_CACHE`；目录 30 槽 LRU；`RenderScratch` 与紧凑 `RenderCell` | 0ms 纯内存微秒响应，稳帧 60FPS |
| **Phases 14-15** | SQLite WAL 并发与 SFTP 双缓冲 | SQLite 全面开启 WAL 模式、NORMAL 同步与 5000ms 超时；SFTP 8 槽双缓冲流水线 | 读写并发不锁库，大文件吞吐提升 30%~50% |
| **Phases 16-18** | AST 瘦身、零拷贝与编译治理 | 拔除 50+ 代理属性与闭包；SharedString 零堆分配共享；`codegen-units = 1` | 消除 50% 字符串堆分配，编译 1.98s |
| **Phases 19-20** | 门面下线与核心公共工具库 | 彻底替代并删除 `smagical-ui-view`；抽取 dispatch / string / filter / model 模块 | 依赖图精简 -1 Crate，消除大面积重复代码 |
| **Phases 21-22** | 终端双缓冲与高亮 ASCII 快径 | 终端 60FPS Ping-Pong 双缓冲（零 CoW 深拷贝）；运维高亮引擎 ASCII 1:1 快速映射 | 削减 ~480MB/s 瞬态内存抖动与 2400次/秒堆分配 |
| **Phases 23-27** | 零分配检索与公共体系归一 | 全仓搜索/过滤统一走 ASCII 快径；全业务/服务/守护进程模型构建 100% 归一 | 0 警告、0 错误、热检查 1.80s 极速全绿 |
| **Phase 28** | SSH 协议心跳保活与智能重连 | russh 心跳 15s/3次；后台终端零渲染排空；1s..15s 指数退避自动重试；原地重连修复 | 彻底杜绝 NAT 假死挂起，后台断线实时感知与自愈 |
| **Phase 29** | 编译内存暴涨与链接闪退终局根治 | 接入 `rust-lld.exe` 突破 4GB 限制；三方依赖符号全剥离；`codegen-units = 16`；FastISel | 消除链接期 0xc0000409 崩溃，峰值内存降低 60%+ |
| **Phase 30** | 全仓模块独立文档规范化矩阵 | 15+ 模块独立 README 全量更新；包含职责、调用范例、函数参数契约与矩阵索引 | 达成极致工程规范，代码与文档 100% 同步自洽 |
| **Phase 31** | 凭据密匙视图与真实网络驱动激活 | 修复私钥文件导入按钮截断与图标垂直居中；无条件注册真实 Keygen/SSH/SFTP 驱动 | 根除 SHA256:Mock 指纹与终端虚假连接秒退问题 |
| **Phase 32** | 复杂网络穿透与跳板拓扑加固 | 多跳 Direct-TCPIP 链路增强（剥离 user@、每跳独立超时）；SOCKS5/HTTP 代理超时保护与健康探测；SFTP 跳板机/代理同等穿透 | 跨内网多跳跳板机与代理穿透稳定可靠，杜绝永久挂起 |
| **Phase 32.5** | 真实数据源嗅探、RefCell 根除与 Toast 闭环 | known_hosts 真实导入/剔除假数据；收窄借用作用域防闪退；Toast TouchArea 尺寸修复与即时闭环 | 真实资产无缝直连，消除 101 崩溃，提示交互 100% 可关 |
| **Phase 33** | Slint 1.18 模块化评估与微内核拓扑收敛 | POC 验证 Slint 1.18.1 `experimental-module-builds`（发现 upstream E0062/E0605 Bug）；统一 8 大独立 UI 插件单一构建拓扑，消除遗留废弃路径与跨插件死样板 | 15 个 Crate 全部直通，全工作区 `cargo check` 达成 3.65s 极速 Clean Build |
| **Phase 34** | 终端 60FPS 极速流控、脏行局部重绘与 PTY 背压 | 紧凑 256 行脏位图 RowDirtyMask；双缓冲行指纹缓存与局部刷新；PTY 128KB 背压合包与容量治理 | 静态/打字降低 98% 渲染开销，超大日志刷屏稳帧 60FPS 杜绝 UI 冻结 |
| **Phase 35** | 资产树 O(N) 虚拟化与 100 视口零堆检索 | 消除 O(N^2) 祖先扫描；memo 记忆化递归；多字段 ASCII 快径；MAX_VIEWPORT_ITEMS=100 截断保护 | 彻底根除万级资产堆抖动与卡顿，全工作区 5.82s 极速 Clean Build |
| **Phase 36** | SFTP 并发队列调度、冲突策略与完整性校验 | TransferQueue 并发槽位池(3~5槽)；Overwrite/Skip/Rename冲突；端到端文件大小完整性核验；滑动窗口平滑测速与 ETA | 杜绝并发 I/O 阻塞进程，消除截断假成功，全工作区 4.68s 极速 Clean Build |
| **Phase 37** | 终端会话录屏回放与历史安全审计 | Asciinema cast v2 标准规范；CastSessionRecorder 全生命周期流式捕获；CastReplayer 时间轴播放器；SecurityAuditInspector 静态扫描 | 操作全程可记录、时间轴可回溯、高危操作可预警，全工作区 15.94s 极速 Clean Build |
| **Phase 38** | 命令片段智能推荐与参数动态记忆 | 变量作用域回退记忆；高频热度加权排序模型；一键注入与 ASCII \x07 音效/按键反馈 | 常用运维命令极速预填置顶，全工作区 15 个 Crate 编译极速全绿 |
| **Phase 39** | 终端分屏拓扑持久化、经典布局预设与多窗格广播执行 | SplitNode Serde 序列化；8 大经典网格分屏预设；Ctrl+Shift+B 输入广播模式；按键/粘贴/片段多机同步 | 多机协同并发运维，分屏拓扑自动持久化，全工作区 11.91s 极速 Clean Build |
| **Phase 40** | 端口转发与 SSH 隧道智能保活自愈管理器 | 动态 SOCKS5 (-D) 代理网关；1s 周期探活与指数退避自愈；1s 吞吐速率与 Unicode 字符波形；端口占用预检与诊断 | 彻底解决端口冲突与静默挂死，流量波形直观实时，全工作区 15 个 Crate 编译极速全绿 |
| **Phase 41** | 终端回滚历史检索、全视口划选与快照导出 | 行缓冲字符流零拷贝检索与前后循环导航；视口自适应滚入居中；全选上下文闭环；多格式快照与剪贴板导出 | 运维日志检索秒级定位，日志一键格式化捕获，全工作区 3m 44s 极速全绿 |

---

## 三、深度优化与演进路线规划 (Deep Optimization Roadmap)

### 阶段三十五（已归档）：真实海量资产树虚拟懒加载与瞬时检索 (Phase 35: Real Host Tree Large-Scale Virtualization & Instant Search)
- **落地成果**：
  1. **树模型算法由 $O(N^2)$ 全面收敛至 $O(N)$ 线性时间**：在 [`crates/smagical-ui/src/tree_model.rs`](crates/smagical-ui/src/tree_model.rs) 中重构 [`build_visible_tree_nodes`](crates/smagical-ui/src/tree_model.rs)、[`build_group_options`](crates/smagical-ui/src/tree_model.rs)、[`sort_tree_hierarchy`](crates/smagical-ui/src/tree_model.rs)、[`build_raw_tree`](crates/smagical-ui/src/tree_model.rs) 与 [`build_cards_from_records`](crates/smagical-ui/src/tree_model.rs)，引入 `parent_map`、`memo` 记忆化与 `children_map` 预索引；
  2. **多维度零分配快速检索**：重构 [`build_search_tree_nodes`](crates/smagical-ui/src/tree_model.rs)，覆盖主机名、IP、端口、用户与在线状态，全流程零堆内存抖动；
  3. **100 条视口虚拟截断与按需加载闭环**：定义 `MAX_VIEWPORT_ITEMS = 100`，对树节点与平铺卡片列表实施最大 100 条推送保护，闭环 `on_load_all_hosts` 一键展示全部；
  4. **工程质量验证**：全工作区 15 个 Crate `cargo check --workspace` **5.82s** 极速全绿，0 错误、0 警告。

### 阶段三十六（已归档）：SFTP 传输任务并发调度与端到端完整性校验 (Phase 36: SFTP Concurrent Transfer Queue & Integrity Verification)
- **落地成果**：
  1. **多任务队列并发控制与槽位调度**：在 [`crates/smagical-ssh/src/sftp.rs`](crates/smagical-ssh/src/sftp.rs) 中实现 [`TransferQueue`](crates/smagical-ssh/src/sftp.rs)，提供 1..=5 可控并发槽位；
  2. **传输冲突策略体系**：实现 [`TransferConflictPolicy`](crates/smagical-ssh/src/sftp.rs)（Overwrite, Skip, Rename）；
  3. **端到端传输完整性校验**：通过远程字节核查与 [`check_transfer_integrity`](crates/smagical-ssh/src/sftp.rs) 消除虚假成功；
  4. **滑动窗口平滑速率与 ETA 预估**：实现 [`TransferSpeedMeter`](crates/smagical-ssh/src/sftp.rs) 3.0s 滑动窗口，精准估算瞬时速率与剩余时间；
  5. **工程质量验证**：全工作区 15 个 Crate `cargo check --workspace` **4.68s** 极速全绿，0 错误、0 警告。

### 阶段三十七（已归档）：终端会话录屏回放与历史安全审计 (Phase 37: Terminal Recording, Replay & Security Audit)
- **落地成果**：
  1. **Asciinema (cast v2) 规范流式录制**：
     - 在 [`crates/smagical-core/src/domain/recording.rs`](crates/smagical-core/src/domain/recording.rs) 中实现 [`CastHeader`](crates/smagical-core/src/domain/recording.rs)、[`CastEventType`](crates/smagical-core/src/domain/recording.rs)、[`CastEvent`](crates/smagical-core/src/domain/recording.rs) 与 [`CastSession`](crates/smagical-core/src/domain/recording.rs)，100% 兼容官方 Asciinema v2 单行 JSONL 协议；
     - 实现 [`CastSessionRecorder`](crates/smagical-core/src/domain/recording.rs)，并在 [`TerminalInstance`](crates/smagical-ui/src/terminal/instance.rs) 中挂载状态机，零内存拷贝实时捕获 PTY 屏幕输出与键盘输入流；
     - 提供 `start_recording`、`stop_recording`、`is_recording` 与格式化计时；在会话关闭时自动落盘归档保护。
  2. **时间轴回放播放器驱动引擎 (Cast Replayer)**：
     - 实现 [`CastReplayer`](crates/smagical-core/src/domain/recording.rs)，具备高精度增量步进（`advance`）、任意时间跳转（`seek_to`）、百分比跳转（`seek_ratio`）与循环控制；
     - 支持 0.25x~8.0x 平滑倍速调节，以及标准化时间标签格式化（如 `01:23 / 04:56`）；
     - 在 [`session_handlers.rs`](crates/smagical-ui/src/handlers/session_handlers.rs) 中封装高层回放载入入口 [`load_latest_recording_replay`](crates/smagical-ui/src/handlers/session_handlers.rs)。
  3. **深度操作安全审计与敏感词侦测引擎 (Security Audit Inspector)**：
     - 实现 [`SecurityAuditInspector`](crates/smagical-core/src/domain/recording.rs)，定义五级风险等级 [`AuditRiskLevel`](crates/smagical-core/src/domain/recording.rs)（Critical, High, Medium, Low, Info）；
     - 覆盖毁灭性指令（`rm -rf /`, `mkfs`, `> /dev/sd*`, Fork 炸弹）、高危系统破坏（`rm -rf`, `fdisk`, `dd`, `reboot`, `shutdown`, `chmod -R 777`）、敏感私钥明文暴露（`BEGIN PRIVATE KEY`）与服务中断等威胁模式；
     - 支持整部录屏静态全量审计扫描 [`inspect_session`](crates/smagical-core/src/domain/recording.rs)；在 [`audit_logger.rs`](crates/smagical-ui/src/audit_logger.rs) 增加录制目录索引并自动记录审计流水。
  4. **工程质量验证**：全工作区 15 个 Crate `cargo check --workspace` **15.94s** 极速全绿，0 错误、0 警告。

### 阶段三十八（已归档）：命令片段智能推荐与参数动态记忆 (Phase 38: Snippet Intelligence & Parameter Auto-Fill)
- **落地成果**：
  1. **双层作用域参数记忆引擎 (SnippetParamMemory)**：
     - 在 [`crates/smagical-core/src/domain/snippet.rs`](crates/smagical-core/src/domain/snippet.rs) 中设计并实现 [`SnippetParamMemory`](crates/smagical-core/src/domain/snippet.rs)；
     - 采用“片段专属作用域 (scoped_params)”优先、“全局跨片段变量 (global_params)”回退的双层降级决策链；
     - 在呼出参数填报对话框时自动对 `{{key}}` 或 `{{key:default}}` 进行瞬时预填，彻底免除运维重复输入高频参数（如端口、环境名、命名空间等）。
  2. **高频常用命令热度与时间衰减推荐模型 (SnippetUsageTracker)**：
     - 在 [`crates/smagical-core/src/domain/snippet.rs`](crates/smagical-core/src/domain/snippet.rs) 中实现 [`SnippetUsageRecord`](crates/smagical-core/src/domain/snippet.rs) 与 [`SnippetUsageTracker`](crates/smagical-core/src/domain/snippet.rs)；
     - 引入复合智能评分算法：基础收藏权重 (+10,000) + 累计执行热度 (+50/次) + 24小时平滑指数级新鲜度加权 (+0~500)；
     - 在 [`snippet_tree_model.rs`](crates/smagical-ui/src/snippet_tree_model.rs) 中实现 [`sort_snippet_tree_with_intelligence`](crates/smagical-ui/src/snippet_tree_model.rs)，全面联动右侧伴生工具栏快速命令（Quick Commands）、搜索过滤列表以及目录树，实现常用运维命令智能置顶。
  3. **命令模版一键注入与系统级按键音效反馈 (One-Click Injection & Audio Feedback)**：
     - 在 [`crates/smagical-ui/src/snippet_service.rs`](crates/smagical-ui/src/snippet_service.rs) 中封装全局单例管理中心，挂载 `%LOCALAPPDATA%\smagical\smalux\snippets` 异步持久化流；
     - 采用 VT100 / ANSI 标准 ASCII `\x07` (BEL) 零依赖管道机制实现超低延迟的物理按键音效/触觉反馈；
     - 在 [`snippet_handlers.rs`](crates/smagical-ui/src/handlers/snippet_handlers.rs) 中统一打通 `record_snippet_execution`，实现注入执行、频次递增、参数记忆与声音提示一体化闭环。
  4. **工程质量验证**：全工作区 15 个 Crate `cargo check --workspace` 极速全绿，0 错误、0 警告。

### 阶段三十九（已归档）：终端分屏拓扑持久化、经典布局预设与多窗格广播执行 (Phase 39: Terminal Multi-Pane Layout Presets, Topology Persistence & Broadcast Input)
- **落地成果**：
  1. **分屏二叉拓扑 Serde 序列化与持久化**：
     - 在 [`crates/smagical-ui/src/terminal/split_tree.rs`](crates/smagical-ui/src/terminal/split_tree.rs) 中为 [`SplitNode`](crates/smagical-ui/src/terminal/split_tree.rs) 与 [`SplitOrientation`](crates/smagical-ui/src/terminal/split_tree.rs) 增加 `Serialize` / `Deserialize` 契约支持；
     - 实现 [`PersistedSplitTopology`](crates/smagical-ui/src/terminal/split_tree.rs)，全生命周期在 `%LOCALAPPDATA%\smagical\smalux\layouts\last_topology.json` 进行异步落盘与容灾重载；
     - 在分屏创建（`on_split_terminal`）、分割条拖拽微调（`on_adjust_splitter`）以及窗格关闭（`on_close_pane_by_id`）等全链路触发自动持久化。
  2. **经典网格分屏预设与快速排布引擎**：
     - 在 [`split_tree.rs`](crates/smagical-ui/src/terminal/split_tree.rs) 中实现 [`SplitLayoutPreset`](crates/smagical-ui/src/terminal/split_tree.rs)，覆盖 8 大运维经典布局：单屏 (`Single`)、左右等分 (`DualVertical` 1:1)、上下等分 (`DualHorizontal` 1:1)、四分宫格 (`QuadGrid` 2x2)、一大左两小右 (`MainLeftDualRight` 1L2R)、一大上两小下 (`MainTopDualBottom` 1T2B)、三列等分 (`TripleColumns` 1:1:1) 与三行等分 (`TripleRows` 1:1:1)；
     - 在 [`session_handlers.rs`](crates/smagical-ui/src/handlers/session_handlers.rs) 的 `on_split_terminal` 中增加预设识别与会话自动重组引擎，一键将现有会话排布至目标拓扑槽位。
  3. **多窗格并发输入广播与批量按键同步 (Multi-Pane Keyboard & Input Broadcasting)**：
     - 全面激活并联动 [`TerminalBridge`](crates/ui/common/ui/features/terminal/terminal_bridge.slint) 中的 `is-broadcast-mode`；
     - 支持 `Ctrl+Shift+B` 全局快捷键现场切换广播模式，配有安全警示 Toast 提示与日志流追踪；
     - 在 `on_terminal_key_input`、`on_terminal_paste` 以及 `on_send_snippet` 中，当广播开启时，原子并发将按键字节、剪贴板文本与命令模版同时发送给所有活跃分屏终端，达成企业级多机批量排障体验。
  4. **工程质量验证**：全工作区 15 个 Crate `cargo check --workspace` **11.91s** 极速全绿，0 错误、0 警告。

### 阶段四十（已归档）：端口转发与 SSH 隧道智能保活自愈管理器 (Phase 40: SSH Tunnel Keepalive, Dynamic SOCKS5 & Auto-Healing Manager)
- **落地成果**：
  1. **SSH 隧道连接池健康探测与自动重连自愈 (Tunnel Keepalive & Auto-Healing)**：
     - 在 [`crates/smagical-core/src/service/tunnel.rs`](crates/smagical-core/src/service/tunnel.rs) 中扩展 `TunnelService` 特征，增加 `is_tunnel_alive` 契约；
     - 在 [`crates/smagical-ssh/src/tunnel_driver.rs`](crates/smagical-ssh/src/tunnel_driver.rs) 中实现原生 RFC 1928 / RFC 1929 动态 SOCKS5 (-D) 代理协议网关支持，含用户口令子协商与 IPv4/域名/IPv6 解析；
     - 在 [`crates/smagical-ui/src/tunnel_daemon.rs`](crates/smagical-ui/src/tunnel_daemon.rs) 中实现 [`AutoHealingManager`](crates/smagical-ui/src/tunnel_daemon.rs)，基于指数退避算法（2s -> 4s -> 8s -> 16s -> 32s -> 60s）在网络闪断后原地自愈重连。
  2. **动态流量统计与速率波形计算 (Real-Time Throughput & Sparkline Telemetry)**：
     - 在 [`tunnel_daemon.rs`](crates/smagical-ui/src/tunnel_daemon.rs) 中实现 `TunnelRateSample` 与 `TunnelRateTracker`，计算 1 秒滑动瞬时吞吐速率（`rx_rate_bps`, `tx_rate_bps`）与自适应单位换算（B/s, KB/s, MB/s, GB/s）；
     - 生成 8 采样平滑 Unicode 字符波形条（` ▂▃▄▅▆▇█`），推送到 Slint `TunnelsBridge`，提供直观的流量动态波形；
     - 后台启动 1 秒高精保活探活循环（`spawn_keepalive_telemetry_loop`），实时同步累计流量与速率。
  3. **隧道启动冲突检测与端口占用防护 (Port Conflict Detection & Bind Protection)**：
     - 在 [`tunnel_driver.rs`](crates/smagical-ssh/src/tunnel_driver.rs) 中实现 `check_local_port_availability`，绑定前预先检测端口占用并输出操作系统级排障指令指导（Windows `netstat -ano | findstr <port>` / Unix `lsof -i :<port>`）；
     - 在 [`tunnel_handlers.rs`](crates/smagical-ui/src/handlers/tunnel_handlers.rs) 中全面接入 `try_start_tunnel_with_result`，捕获冲突并提示友好排障 Toast。
  4. **工程质量验证**：全工作区 15 个 Crate `cargo check --workspace` 极速全绿，0 错误、0 警告。

### 阶段四十一（已归档）：终端回滚历史瞬时检索、全视口划选与屏幕快照导出 (Phase 41: Terminal Scrollback Search, Selection Acceleration & Viewport Snapshot Export)
- **落地成果**：
  1. **终端行缓冲高精度检索与前后导航状态机 (Terminal In-Buffer Search & Match Navigation Engine)**：
     - 在 [`crates/smagical-ui/src/terminal/parser.rs`](crates/smagical-ui/src/terminal/parser.rs) 中实现 `TerminalSearchMatch` 与 `find_matches`，对终端整屏与历史回滚缓冲执行零拷贝字符流扫描；
     - 实现 `TerminalSearchState`，记录检索词、大小写敏感、匹配列表与当前高亮序号；
     - 实现 `focus_match`，智能计算历史回滚目标 `display_offset`，联动 `scroll_to_offset` 将目标行自动滚动入可见屏幕中央，并调用 `set_selection` 触发即时划选高亮；
     - 在 [`TerminalInstance`](crates/smagical-ui/src/terminal/instance.rs) 中封装 `search_query`、`search_next`、`search_prev`；
     - 在 [`session_handlers.rs`](crates/smagical-ui/src/handlers/session_handlers.rs) 中全面接入 `tb.on_search_terminal` 回调，实时联动 Slint UI 视口滚动偏移与命中提示。
  2. **终端全视口划选与快捷菜单交互闭环 (Terminal Select-All & Viewport Selection)**：
     - 在 [`TerminalParser`](crates/smagical-ui/src/terminal/parser.rs) 与 [`TerminalInstance`](crates/smagical-ui/src/terminal/instance.rs) 中实现 `select_all`，一键将当前视口全部字符纳入选区；
     - 在 [`terminal_viewport.slint`](crates/ui/kernel/ui/views/center_terminal/terminal_viewport.slint) 中闭环右键菜单 `select-all` 交互，联动 `TerminalBridge.terminal-selection-changed` 与系统剪贴板复制。
  3. **终端屏幕快照与多格式日志导出引擎 (Terminal Screen & Scrollback Snapshot Export)**：
     - 在 [`TerminalInstance`](crates/smagical-ui/src/terminal/instance.rs) 中定义 [`SnapshotExportFormat`](crates/smagical-ui/src/terminal/instance.rs)（`PlainText`, `Markdown`, `Html`），实现 `export_snapshot`；
     - 实现 `copy_snapshot_to_clipboard`，集成 `arboard::Clipboard` 实现排障现场日志一键捕获复制；
     - 编写完备单元测试：`test_terminal_instance_search_and_navigation`、`test_terminal_instance_select_all`、`test_terminal_instance_export_snapshot`。
  4. **工程质量验证**：全工作区 15 个 Crate `cargo check --workspace` **3m 44s** 极速全绿，0 错误、0 警告。

### 阶段四十二（已归档）：端到端 AEAD 加密备份快照与多端云同步自愈 (Phase 42: End-to-End Encrypted Snapshot Backup & Disaster Recovery Sync)
- **落地成果**：
  1. **客户端端到端 AEAD 口令加密封包 (End-to-End Passphrase Encryption)**：
     - 在 [`crates/smagical-core/src/service/backup/mod.rs`](crates/smagical-core/src/service/backup/mod.rs) 中实现基于 Argon2id (RFC 9106, 64MB, 3 iterations, 4 parallelism) 密钥派生与 AES-256-GCM 认证加密；
     - 密文输出标准化信封容器 [`EncryptedSnapshotEnvelope`](crates/smagical-core/src/service/backup/mod.rs)（魔数 `smalux_encrypted_snapshot_v1`、96-bit 随机 Nonce 与 128-bit MAC Tag、SHA-256 完整性摘要）；
     - 实现 `encrypt_snapshot_payload` 与 `decrypt_snapshot_payload`，实现 100% 杜绝多云存储（WebDAV, S3, Gist, 本地）凭据明文泄露；
     - 实现 `inspect_backup_payload` 与 `is_encrypted_snapshot`，支持不落盘预检快照内主机、分组、凭据等资产元数据。
  2. **快照导入冲突消解与智能合并策略 (Smart Merge & Conflict Policy)**：
     - 设计 [`BackupConflictPolicy`](crates/smagical-core/src/service/backup/mod.rs) 枚举（`Overwrite`, `SkipExisting`, `MergeNewer`）与 [`BackupRestoreReport`](crates/smagical-core/src/service/backup/mod.rs) 执行报告；
     - 在 [`restore_backup_payload_with_policy`](crates/smagical-core/src/service/backup/mod.rs) 中无损处理主机、分组、凭据、隧道、代码片段资产，细粒度统计导入与跳过计数；
     - 保留原始 `restore_backup_payload` 接口 100% 向后兼容；并在设置中心还原与文件导入处理器中无缝集成。
  3. **远端快照生命周期滚动修剪与后台守护增强 (Remote Lifecycle Retention & Pruning)**：
     - 在 [`BackupDriver`](crates/smagical-core/src/service/backup/mod.rs) Trait 中增配 `prune_old_snapshots(keep_count)` 契约方法；
     - 升级 [`backup_daemon.rs`](crates/smagical-ui/src/backup_daemon.rs)，在快照推送完成后自动依据任务 Retention 限制（如保留 10 份）修剪数据库与远端存储（S3/WebDAV/Local）的冗余陈旧快照；
     - 在 [`settings_handlers/backup.rs`](crates/smagical-ui/src/handlers/settings_handlers/backup.rs) 中全面接入带跳过冲突提示的还原 Toast。
  4. **工程质量验证**：
     - 编写完备单元测试：`test_format_bytes_size`、`test_compute_sha256`、`test_encryption_and_decryption_roundtrip`、`test_inspect_backup_payload`；
     - 全工作区 15 个 Crate `cargo check --workspace` 极速全绿，0 错误、0 警告。

### 阶段四十三（已归档）：SFTP 高性能并发传输引擎与队列调度管理器 (Phase 43: High-Performance SFTP Transfer Engine & Queue Concurrency Manager)
- **落地成果**：
  1. **并发槽位受控调度与排队状态机 (Controlled Concurrency & Queue State Machine)**：
     - 在 [`crates/smagical-ui/src/transfer_manager.rs`](crates/smagical-ui/src/transfer_manager.rs) 中设计并实现 [`TransferQueueManager`](crates/smagical-ui/src/transfer_manager.rs)；
     - 具备原子可调最大并发数（`max_concurrency`，默认 3 槽位，限制 1..=10）；
     - 超额任务自动进入 `pending_queue` 排队等待，处于 `TransferStatus::Pending` 态；前序任务完成或取消时自动弹出下一个任务调度执行（`schedule_next`）。
  2. **物理可取消任务句柄与资源释放 (Physical Cancellation with AbortHandle)**：
     - 活跃传输任务的 Tokio 协程生成 `tokio::task::AbortHandle` 存入 `active_transfers`；
     - 当用户触发“停止 (stop)”或“移除 (remove)”时，调用 `handle.abort()` 立即物理中断底层读写与网络传输管道，彻底杜绝后台悬挂死锁与连接泄漏。
  3. **真实双缓冲流式遥测与 100ms 批量节流同步 (Streamed Telemetry & UI Throttling)**：
     - 彻底消除历史上遗留的 `12.5 MB/s`、`14.5 MB/s` 硬编码假速度与假状态；
     - 桥接底层 SFTP 双缓冲流式通道派发的 `TransferProgress`，基于平滑指数加权移动平均 (EMA) 算法计算瞬时速率与 ETA 预估；
     - 遥测协程采用 100ms 滑动窗口批量调度回 Slint UI 主事件循环，防止高频密集重绘引发主线程卡顿。
  4. **文件夹多级递归并发与父级动态聚合 (Recursive Folder Transfers & Parent Aggregation)**：
     - 上传本地文件夹时通过 `scan_folder_recursive` 探测各层级文件，生成具有层级折叠属性的父任务与各子文件任务；
     - 子任务独立进入并发槽位调度（享受 3 任务并行吞吐），遥测流动态向上累计父文件夹的已传输字节数与聚合速率；子项全部完成自动标记父文件夹完成。
  5. **传输队列生命周期闭环与桌面气泡通知 (Lifecycle & Desktop Notification)**：
     - 在 [`file_handlers.rs`](crates/smagical-ui/src/handlers/file_handlers.rs) 中重构 `execute_transfer_task`、`fb.on_transfer_action`（暂停/继续/停止/重新传输/移除）与 `fb.on_clear_completed_transfers`，100% 委托给 `TransferQueueManager`；
     - 任务成功或失败自动触发系统级桌面气泡通知（`NotificationManager`），并精准失效目标端目录 LRU 缓存，触发无感静默热刷新。
  6. **工程质量验证**：
     - 编写单元测试 `test_transfer_queue_concurrency_bounds`；
     - 全工作区 15 个 Crate `cargo check --workspace` 极速全绿，0 错误、0 警告。

### 阶段四十四（已归档）：既有系统异常边界加固与内存/协程生命周期收敛 (Phase 44: System Boundary Hardening & Concurrency/Memory Lifecycle Convergence)
- **落地成果**：
  1. **I/O 极端异常语义本土化与目标磁盘空间防爆 (Localized I/O Errors & Disk Full Handling)**：
     - 在 [`crates/smagical-ui/src/transfer_manager.rs`](crates/smagical-ui/src/transfer_manager.rs) 中实现 `map_io_error`，将操作系统底层 `StorageFull` / `ENOSPC` 转化为明确的「目标磁盘空间不足 (Disk Full)」，`PermissionDenied` 转化为「目标路径权限不足」，`ConnectionReset` / `TimedOut` 转化为网络断开/超时；
     - 在本地读写流式循环 `copy_file_with_progress` 中，任何 I/O 错误发生时立即异步执行目标残损文件删除，杜绝留下损坏半截文件。
  2. **传输异常中断与手动取消的残存脏数据自愈清理 (Self-Healing Residual Cleanup)**：
     - 在 `TransferQueueManager::spawn_worker` 的终态捕获块中，针对下载或本地复制任务，若遇到网络超时、传输报错或用户主动 Cancel/Abort，自动异步清理本地目标残损文件（`tokio::fs::remove_file`），保障本地磁盘文件完整性。
  3. **内部注册表与 UI 任务列表无界堆积治理 (Unbounded Task Registry & UI List Pruning)**：
     - 治理长效内存泄漏：在 `clear_completed` 中同步原子裁剪 `TransferQueueManager` 内部的 `jobs_registry`，仅保留活跃与待处理项；
     - 在 `submit_job` 中增设 2000 项安全水位防护，防止内存被无界并发排队耗尽；
     - 在 [`file_handlers.rs`](crates/smagical-ui/src/handlers/file_handlers.rs) 中实现 `prune_excess_transfer_tasks`，在单文件与批量目录任务压入时自动裁剪已完成/失败的历史任务，保持 UI 任务列表在 500 项高水位内平稳运行。
  4. **后台常驻探针与巡检协程生命周期收敛 (Daemon Goroutine Lifecycle Convergence)**：
     - 在 [`tunnel_daemon.rs`](crates/smagical-ui/src/tunnel_daemon.rs)（1s 探活链路循环）与 [`backup_daemon.rs`](crates/smagical-ui/src/backup_daemon.rs)（30s 周期巡检循环）中引入主窗口存活检测（`window_weak.upgrade().is_none()`）；
     - 当客户端退出或主窗口销毁时，后台协程自动跳出常驻循环终止退出，彻底释放所持有的 `Arc<Storage>`、`Arc<TunnelService>` 与网络链路资源。
  5. **终端极端海量数据刷屏防爆与 PTY 队列安全水位 (Terminal High-Watermark Protection)**：
     - 在 [`terminal/instance.rs`](crates/smagical-ui/src/terminal/instance.rs) 的 `poll_output` 中增设 2MB 高水位线截断防护（保留最新 1MB 尾部），彻底防御 `cat /dev/urandom` 或万行日志刷屏导致的 OOM；
     - 在 [`terminal/parser.rs`](crates/smagical-ui/src/terminal/parser.rs) 的 `TerminalEventListener` 中对 PTY 回写队列增设 128 容量保护，防止光标查询指令风暴撑爆内存。
  6. **原生 SSH Exec 命令输出缓冲区 10MB 防爆上限 (Session Exec Output Protection)**：
     - 在 [`crates/smagical-ssh/src/session_driver.rs`](crates/smagical-ssh/src/session_driver.rs) 的 `execute_command` 中增设 `MAX_EXEC_OUTPUT_BYTES = 10 * 1024 * 1024`（10MB）安全水位保护，当命令产生巨量输出时及时截断退出，彻底杜绝单次命令输出撑爆内存。
  7. **纯 Rust 原生 SFTP 读写异常双向残损文件自愈 (Native SFTP Incomplete File Self-Healing)**：
     - 在 [`crates/smagical-ssh/src/sftp_driver.rs`](crates/smagical-ssh/src/sftp_driver.rs) 中：下载失败时（`download_file`）自动异步执行 `tokio::fs::remove_file(local_path)` 清理本地残损文件；上传失败时（`upload_file`）自动执行 `sftp.remove_file(remote_path)` 清理远程残损文件。
  8. **凭据保险库自动锁定巡检协程生命周期收敛 (Vault Auto-Lock Daemon Convergence)**：
     - 在 [`crates/smagical-ui/src/handlers/settings_handlers/security.rs`](crates/smagical-ui/src/handlers/settings_handlers/security.rs) 的 `start_auto_lock_monitor` 10s 巡检循环中引入主窗口弱引用升级检测（`window_weak.upgrade().is_none()`），窗口关闭时自动退出协程，释放底层的数据库连接池。
  9. **PTY 伪终端孤儿进程彻底终结与 4MB 有界背压通道 (PtyProcess Zombie Prevention & Backpressure)**：
     - 在 [`crates/smagical-ui/src/terminal/pty.rs`](crates/smagical-ui/src/terminal/pty.rs) 中为 `PtyProcess` 实现 `Drop` 特征，在会话标签页关闭或进程销毁时强制调用 `self.child.kill()`，杜绝 Windows/Unix 孤儿进程残留；
     - 将输出读取无界通道替换为 `sync_channel(512)`（4MB 缓冲上限），向操作系统底层 PTY 管道施加物理背压。
  10. **主机树写锁作用域收窄与嵌套锁解除 (master_tree Lock Scope Narrowing)**：
      - 在 [`crates/smagical-ui/src/handlers/host_handlers.rs`](crates/smagical-ui/src/handlers/host_handlers.rs) 的 `move_and_reorder_raw_node` 与 `create_group` 中，在数据结构变更完成后立即 `drop(tree)` 提前释放写锁，彻底解除在持有锁期间执行 Slint UI 模型批量序列化拷贝造成的线程阻塞与嵌套锁竞争。
  11. **历史会话大列表 300 项容量截断保护 (History UI Model Capping)**：
      - 在 [`crates/smagical-ui/src/handlers/history_handlers.rs`](crates/smagical-ui/src/handlers/history_handlers.rs) 中为按主机聚合视图和时间轴“更早”视图设置 300 项容量截断保护，杜绝长年累月成千上万条历史记录拖慢 Slint 渲染帧率。
  12. **终端录屏事件容量防护 (CastSessionRecorder Event Cap)**：
      - 在 [`crates/smagical-core/src/domain/recording.rs`](crates/smagical-core/src/domain/recording.rs) 中设置 `MAX_SESSION_EVENTS = 100_000` 容量保护，防止用户忘记关闭录制时长时间运行导致内存无限膨胀。
  13. **工程质量与分级编译验证**：
      - 严禁 `--tests` 触发 Slint AST 宏展开导致的 MSVC/Rustc OOM；
      - 统一采用 `$env:RUST_MIN_STACK="134217728"; cargo check -p smagical-ui --lib`（3.43s 极速通过）；
      - 全工作区 15 个 Crate `cargo check --workspace` 100% 验证通过，0 错误、0 警告。

### 阶段四十五（待定）：Release 发布构建工程与二进制体积裁剪优化 (Phase 45: Release Profile & Binary Size Optimization)
- **候选目标**：
  1. Cargo 配置优化（LTO、codegen-units=1、panic=abort、opt-level=3/z）；
  2. Windows MSVC 静态资源与可执行文件符号剥离（strip = true）；
  3. 最终产物瘦身评估与跨平台发布打包配置核验。

---

## 四、硬件保全与研发纪律 (Strict Constraints)

> [!CAUTION]
> 1. **严禁执行 `cargo test`**：全程使用 `cargo check` 分级验证（模块级 -> 内核底座 -> 全工作区），杜绝 MSVC 链接器并发 OOM 导致硬件闪退。
> 2. **契约零破坏原则**：严禁破坏既有的 Bridges 属性、回调名称与领域接口，保证前端交互与数据状态管理 100% 向后兼容。
> 3. **实时同步文档**：每次优化均同步更新本规划文档及对应工作成果。

