# smagicalssh 会话恢复基线

## 项目目标

- 项目：`smagicalssh`
- 目标：Rust 跨平台桌面 SSH 工作台，参考 XTerminal / Termora / Termius / Tabby。
- 当前优先级：继续完成核心，UI 先不扩展。
- 核心范围：SSH shell/PTY、远程命令、SFTP、端口转发/隧道、命令历史、主机/分组/标签页/最近连接、Known Hosts、凭据安全存储、Snippets、工作区恢复。
- 工程要求：模块化、功能化、单一职责，小文件，中文注释，完整测试。
- 本轮推进：继续收紧真实 SSH 边界，优先补纯离线测试，再把真实网络烟测后置。
- 本轮新增：继续收口 SSH 客户端核心，`smagical-ssh-client-core` 继续承载 tunnel running 判断 helper、host key/connection 错误分流 helper、主机密钥策略选择 helper、SSH channel/executor operation 常量、SFTP operation 常量、远程 shell drain 停止判断 helper、ssh-agent 身份缺失认证错误 helper、SSH/SFTP 错误分类 helper、SSH 未连接会话错误 helper、SSH 连接错误 helper、SSH 连接生命周期事件 helper、SSH executor 隧道停止事件 helper、SFTP 目录事件 helper、SSH channel 生命周期事件 helper、SSH 隧道状态事件 helper、SSH 隧道轮询 tick 常量、SSH 隧道内部名常量、SSH 隧道双向复制 helper、SSH 隧道错误 helper、SSH 认证拒绝 helper、SSH channel request helper、SSH 认证 helper、SSH 错误映射 helper、SSH PTY 尺寸 helper、SFTP 传输复制 helper、RemoteTunnel 运行句柄、SOCKS5 握手解析、SFTP 纯映射 helper、channel 消息映射、handler、ssh-agent 身份选择、主机密钥校验策略和 `russh` 客户端配置。

## 当前进度

- 已实现真实 SSH 后端：连接、认证、shell、远程命令、SFTP、隧道。
- 已实现本地终端、命令历史、远程命令退出码和耗时回写。
- 已实现 SFTP 浏览、上传、下载、删除、创建目录、书签、传输状态、取消队列任务。
- 已实现隧道 Local / Remote / Dynamic SOCKS5 的启动、停止、状态管理。
- 已实现 Known Hosts、凭据引用、安全解析、redb 快照持久化。
- 最近核心新增：SFTP 传输任务绑定 `session_id`，会话断开/失败时只收敛对应会话的非终态传输。
- 最近核心新增：隧道运行态终态保护，避免停止/失败后被迟到事件重新拉回运行态。
- 最近核心新增：真实后端 SFTP 请求失败后丢弃对应缓存子系统句柄，下一次请求重新打开 SFTP 会话。
- 最近核心新增：远程 shell 输入写入失败后丢弃对应缓存 shell 句柄，避免后端继续保留失效交互通道。
- 最近核心新增：真实后端同 `session_id` 重连成功时先清理旧 shell/SFTP/connection 资源，避免重连后复用旧子系统句柄。
- 最近核心新增：启动隧道前清理同 `session_id` 的旧 shell/SFTP 子资源，避免 connection 交给 tunnel 后仍残留交互子系统句柄。
- 最近核心新增：断开连接时先取出全部后端资源并尽力关闭，关闭失败只记录警告，最终仍回传 Disconnected。
- 最近核心新增：后端隧道句柄记录所属 `session_id`，停止隧道时必须同时匹配会话和规则名，避免迟到停止命令影响同名新隧道。
- 最近核心新增：后端启动同名隧道成功后会停止被覆盖的旧隧道句柄，避免旧监听循环泄漏。
- 最近核心新增：真实后端同 `session_id` 重连成功时会停止该会话遗留的后端隧道句柄，避免重连后旧监听循环继续运行。
- 最新核心新增：真实后端断开连接时会统一取出 shell/SFTP/connection/tunnel 运行态，并停止该会话遗留的后端隧道。
- 最新核心新增：远程 shell drain 收到 `CommandExited` / `Failed` / `Disconnected` 后都会丢弃缓存 shell 句柄，避免 shell 退出后继续复用失效通道。
- 最新核心新增：真实后端重复打开同一 `session_id` 的 shell 时会取出被替换的旧 shell，并尽力关闭旧输入通道。
- 最新核心新增：真实后端缓存 SFTP 会话替换时会取出旧 SFTP，并复用统一关闭逻辑，避免旧子系统句柄被静默覆盖。
- 最新核心新增：SFTP 操作失败后会裁剪同会话后续 pending 上传/下载命令并标记传输失败，同时保留浏览/刷新类 SFTP 命令和会话连接态。
- 最新核心新增：`BackendEvent::SftpFailed` 只收敛 SFTP 浏览器错误，不再把同会话未关联传输一并失败；传输失败继续由传输事件和连接级失败负责。
- 最新核心新增：SFTP `TransferProgress` 更新必须同时匹配 `transfer_id` 与 `session_id`，避免迟到或错会话事件污染新传输状态。
- 最新核心新增：SFTP 传输取消和关闭标签页清理 pending 传输时同样校验 `session_id`，避免同 ID 错会话任务或命令被误取消。
- 最新核心新增：隧道运行态绑定 `session_id`，前端停止、失败和关闭标签页清理均按 `session_id + rule_name` 收敛，避免同名错会话隧道运行态互相污染。
- 最新核心新增：SFTP 传输入队替换同样按 `session_id + transfer_id` 收敛，避免同 ID 错会话传输任务互相覆盖。
- 最新核心新增：取消 SFTP 传输时若 `transfer_id` 命中多个会话任务会 fail-fast 报错，不再按列表顺序误取消第一条。
- 最新核心新增：停止隧道时如果规则级运行态存在但当前会话运行态缺失，会返回明确错误，不再触发不可达分支 panic。
- 最新核心新增：取消旧 SFTP 上传时清理浏览器 loading 必须匹配当前浏览器 owner，避免旧会话任务误清同主机新会话加载态。
- 最新核心新增：SFTP 选择目录项前会先 claim 可用浏览器 owner，当前 owner 已断开时转交给同主机可用会话，否则明确拒绝选择。
- 最新核心新增：SFTP 刷新和上传开启 loading 时改用会话归属 API，claim owner 后只允许匹配会话写入浏览器加载态。
- 最新核心新增：SFTP 上传/下载会先完成输入校验，再 claim 浏览器 owner，避免失败请求改变同主机浏览器归属。
- 最新核心新增：取消 SFTP 上传时 pending refresh 判断必须匹配当前浏览器 owner，旧会话刷新命令不再阻塞当前 loading 清理。
- 最新核心新增：关闭当前 SFTP 浏览器 owner 并转交给同主机标签页时会清理 loading 和错误态，避免已关闭会话状态泄漏给新 owner。
- 最新核心新增：关闭当前 SFTP owner 时只会转交给可用同主机 SFTP 标签页，剩余标签页均断开/失败时移除浏览器运行态。
- 最新核心新增：SFTP 面板投影优先绑定当前 active tab 的主机，当前主机没有浏览器时展示该主机空态，不再回退展示其他主机浏览器。
- 最新核心新增：激活 SFTP 标签页时会同步转交同主机浏览器 owner，避免后续刷新、上传等按 host 调度的命令继续落到旧会话。
- 最新核心新增：激活断开/失败的 SFTP 标签页不会抢占同主机可用浏览器 owner，避免不可用会话重新接管 SFTP 浏览器。
- 最新核心新增：当前 SFTP owner 收到连接级断开/失败事件后会立即转交给同主机可用会话，并清理 loading 和错误态，避免 stale owner 长时间残留。
- 最新核心新增：底层 SFTP owner 转交 API 会校验目标 session 必须是同主机 SFTP 标签页，避免误把 shell、错主机或缺失 session 设为 owner。
- 最新核心新增：底层 SFTP owner 转交 API 同时要求目标 session 处于可接管状态，断开/失败的 SFTP 标签页不会被设为 owner。
- 最新核心新增：SFTP 目录结果按 session 写入时会校验来源仍是可用 SFTP owner，断开/失败 owner 或同 ID 非 SFTP 标签页的迟到目录结果不再覆盖浏览器状态。
- 最新核心新增：普通 `SftpFailed` 操作错误只写入仍可用的当前 SFTP owner，迟到操作错误不会覆盖连接级断开/失败留下的浏览器错误。
- 最新核心新增：按 session 开启 SFTP loading 时要求当前 owner 仍可用，断开/失败 owner 不能重新进入 loading；关闭 loading 仍允许用于清理失效 owner。
- 最新核心新增：会话终态判断集中到 `SessionStatus::is_terminal()`，SFTP owner、SFTP 命令、标签关闭和终端缓冲更新复用同一生命周期口径。
- 最新核心新增：保存 SFTP 书签前会先 claim 同主机可用浏览器 owner，唯一 owner 已断开/失败时不会把旧目录保存为书签。
- 最新核心新增：SFTP 传输和隧道运行态生命周期判断集中到 `TransferStatus` / `TunnelStatus`，队列取消、迟到进度、隧道停止和标签关闭复用同一终态口径。
- 最新核心新增：后端 pump 收到 SFTP 操作错误后只保留同会话浏览读取，裁剪后续上传/下载/删除/建目录写操作，避免失败后继续执行排队写请求。
- 最新核心新增：远程 shell drain 终止事件必须匹配当前 `session_id` 才会丢弃缓存 shell，错会话 `Disconnected` 不再影响当前交互通道。
- 最新核心新增：后端事件终态判断集中到 `BackendEvent::is_terminal()`，`CommandExited` / `Failed` / `Disconnected` 共用同一口径，shell drain 缓存丢弃复用该判断。
- 最新核心新增：后端队列泵执行 `DrainSessionOutput` 前会复查会话仍是可轮询的交互式 shell，终态 shell 的迟到 drain 命令会被静默丢弃，不再触发后端错误。
- 最新核心新增：后端队列泵执行 `SendShellInput` 前会复查会话仍可交互，终态 shell 的迟到输入命令会被静默丢弃，不再触发真实后端写入错误。
- 最新核心新增：后端队列泵执行 SFTP `ListDir` 前会复查会话仍是当前可用 SFTP 浏览器 owner，终态 owner 的迟到浏览命令会被跳过并清理 loading。
- 最新核心新增：后端队列泵执行 SFTP 上传/下载前会复查对应 SFTP 标签页仍非终态，终态传输命令会在本地标记失败并清理 loading，不再触发真实后端请求。
- 最新核心新增：后端队列泵执行 SFTP 删除/建目录前会复查当前 SFTP 浏览器 owner 仍可用，终态写命令会本地写入浏览器错误并清理 loading，不再触发真实后端请求。
- 最新核心新增：后端队列泵执行 `StartTunnel` 前会复查会话仍是匹配规则的非终态隧道标签页，终态或串台启动命令会被静默跳过，不再触发真实后端请求。
- 最新核心新增：后端队列泵执行 `StopTunnel` 前会复查会话仍是匹配规则的非终态隧道标签页，终态或串台停止命令会被静默跳过，不再触发真实后端请求。
- 最新核心新增：后端队列泵执行 `OpenShell` / `RunCommand` 前会复查会话仍是对应类型的非终态标签页，连接后迟到的 shell/命令启动请求会被静默跳过，不再触发真实后端请求。
- 最新核心新增：后端队列泵执行 `Connect` 前会复查会话仍是目标 host 匹配的非终态远程标签页，终态、缺失、本地或错 host 连接命令会被静默跳过，不再创建真实后端连接。
- 最新核心新增：后端队列泵跳过失效 `Connect` 时会本地标记该会话失败，并裁剪同会话后续 pending 启动命令，避免连接未建立时继续打开 shell/SFTP/隧道。
- 最新核心新增：关闭已连接但 `StartTunnel` 仍 pending 的隧道标签页时会取消启动命令但仍排 `Disconnect`，确保已建立的真实 SSH connection 继续进入后端资源清理；队列泵即使标签页已关闭也会执行 `Disconnect`。
- 最新核心新增：后端队列泵跳过终态 `RunCommand` 时会复用远程命令历史收尾逻辑，避免后端请求未执行但命令历史长期停留在未完成状态。
- 最新核心新增：隧道 `StartTunnel` / `StopTunnel` 执行许可现在同时要求会话标签页未终态，避免会话已断开但隧道 runtime 仍处于 Starting/Stopping 时继续触发真实后端。
- 最新核心新增：`RemoteCommandStarted` 后端事件只会标记非终态远程命令标签为 RunningCommand，串台到 shell/SFTP/隧道标签时会被忽略，避免迟到事件污染标签类型状态。
- 最新核心新增：`ShellOpened` 后端事件只会标记非终态 shell 标签为 Connected，串台到远程命令/SFTP/隧道标签时会被忽略，避免迟到事件污染标签类型状态。
- 最新核心新增：`CommandExited` 后端事件只会终结非终态 shell / 远程命令标签，串台到 SFTP/隧道等非进程标签时会被忽略，避免迟到退出事件污染标签状态。
- 最新核心新增：远程连接生命周期事件改由会话模块按标签类型收敛，`Connecting` / `Authenticating` / `Authenticated` 不再污染本地 shell，`Connected` 仍允许本地 shell 与远程标签接收。
- 最新核心新增：隧道后端状态事件必须匹配非终态隧道标签和规则名，已断开/失败标签不再接收迟到 Running/Failed/Stopped 事件，避免终态标签污染 runtime。
- 最新核心新增：`Failed` / `Disconnected` 的会话状态、SFTP 浏览器、传输任务和隧道 runtime 收敛下沉到 `SessionManager`，reducer 不再手写跨模块清理步骤。
- 最新核心新增：动态隧道 SOCKS5 CONNECT 握手解析从 `TcpStream` 解耦为 `AsyncRead + AsyncWrite` 协议函数，覆盖 IPv4、域名、IPv6、非 CONNECT、未知地址类型和成功响应字节。
- 最新核心新增：隧道 TCP 基础层补齐离线测试，覆盖监听绑定失败时携带规则名、accept tick 空闲超时返回 `None`、双向复制同时转发两个方向数据。
- 最新核心新增：SFTP 上传/下载共用私有 `copy_transfer_with_progress` 传输循环，分块复制、进度事件和 IO 错误映射集中到单一职责 helper，并用纯内存 IO 覆盖分块、空流和写入错误路径。
- 最新核心新增：动态隧道 SOCKS5 握手会校验客户端是否提供 no-auth 方法，缺失时回写 `0x05 0xFF` 并拒绝请求，避免无认证方法协商失败时仍继续 CONNECT。
- 最新核心新增：本地 PTY reader 将字节读取、终端流解码、关闭事件收敛到私有 `read_events_from_stream`，线程入口只负责转发事件；新增纯内存 reader 测试覆盖普通输出、ANSI clear 和读错误路径。
- 最新核心新增：本地 PTY fallback 状态机补齐离线测试，覆盖等待未到期保持 Waiting、运行中无结果保留 receiver、收到结果返回事件并清空状态、worker 断开时静默清理状态。
- 最新核心新增：本地 PTY 执行器补齐无会话和不支持命令边界测试，覆盖缺失会话 drain 空返回、缺失会话 disconnect 幂等返回 Disconnected、远程命令请求被本地执行器拒绝且不创建 session。
- 最新核心新增：`client/tests.rs` 补齐主机密钥策略、算法 hint、认证错误和 `host_key_or_connection_error` 的纯离线测试。
- 最新核心新增：`client/session/tests.rs` 补齐 `collect_command_message` 的 `ExitSignal` 输出映射和非终态消息忽略测试。
- 最新核心新增：`client/session/sftp.rs` 的 `copy_transfer_with_progress` 补齐读取错误映射测试。
- 最新核心新增：`executor/tests.rs` 补齐 `connected_session_error`、shell 输入缓存失效门和 SFTP 缓存失效门的离线测试。
- 最新核心新增：`client/session/tunnel/tests.rs` 补齐 SOCKS5 unsupported version、非法域名字节、短端口读取和 success response 写失败的纯离线错误路径测试。
- 最新核心新增：`client/tests.rs` 补齐 SSH handler 共享状态边界，覆盖拒绝的 host key 结果记录和同 endpoint forwarded channel 订阅替换关闭旧 receiver。
- 最新核心新增：`executor/tests.rs` 补齐真实 SSH executor 空状态/缺失会话幂等边界，覆盖 tunnel_count 初始值、缺失 runtime 取资源、缺失会话取隧道和停止不存在隧道。
- 最新核心新增：`plan/tests.rs` 补齐 SSH 连接计划联网前边界，覆盖 key/certificate 无 passphrase 保持 `None`，以及缺失私钥/证书凭据时保留认证用户名并映射为认证失败。
- 最新核心新增：`security/resolver.rs` 补齐认证材料解析边界，覆盖 key/certificate 无 passphrase 保持 `None`，缺失私钥口令和缺失证书时返回精确 `MissingSecret`。
- 最新核心新增：`backend/auth.rs` 补齐认证模型转换边界，覆盖 password、key、agent、certificate 从 `AuthProfile` 到 `BackendAuth` 时保留用户名、`SecretRef` 和 agent key hint。
- 最新核心新增：`backend/command.rs` 补齐后端命令路由边界，覆盖 Connect、RunCommand、SFTP、StartTunnel、StopTunnel、Disconnect 的 `session_id()` 与 `kind()` 映射。
- 最新核心新增：`backend/executor/tests.rs` 补齐执行器抽象边界，覆盖脚本执行器 FIFO 响应消费和共享 noop 执行器经锁调用仍返回 UnsupportedCommand。
- 最新核心新增：`backend/event.rs` 补齐后端事件口径边界，覆盖 HostKeyVerified、SftpEntries、TransferProgress、TunnelStatusChanged 的 `session_id()` 映射和非终态判断，以及 Disconnected 终态判断。
- 最新工程优化：`build.rs` 为 `ui/*.slint` 和 `build.rs` 显式输出 `cargo:rerun-if-changed`，减少后端代码/测试改动时误触发 Slint build script 的概率。
- 最新工程优化：`build.rs` 对 `ui/*.slint` 路径排序后再输出 `cargo:rerun-if-changed`，避免文件系统枚举顺序抖动影响构建脚本输出。
- 最新工程优化：新增 `src/lib.rs` 承接原 `main.rs` 模块导出，`main.rs` 缩为调用 `smagicalssh::app::run()`，完成 lib/bin 拆分第一步，后续可用 `cargo test --lib ...` 只跑库测试目标。
- 最新工程优化：新增 `smagical-core` workspace crate，迁移 `ids` / `security` / `history` / `visual` 四个纯模型模块，删除主 crate 中对应旧副本，并通过 `pub use smagical_core::*` 保持旧 API。
- 最新工程优化：继续迁移 `host` / `session` / `sftp` / `snippet` / `tunnel` / `workspace` 到 `smagical-core`，主 crate `src/model.rs` 仅导出应用根状态、UI 状态和 core re-export。
- 最新工程优化：新增 `smagical-session` workspace crate，迁移原 `src/session` 会话运行态管理；主 crate `src/session.rs` 改为 re-export，保持 `crate::session::*` 兼容。
- 最新工程优化：新增 `smagical-terminal` workspace crate，迁移原 `src/terminal` 终端核心状态和流解析；主 crate `src/terminal.rs` 改为 re-export，保持 `crate::terminal::*` 兼容。
- 最新工程优化：新增 `smagical-config` workspace crate，迁移原 `src/config` 的 `AppConfig`、默认值、背景归一化测试和视觉配置解析；主 crate `src/config.rs` 改为 re-export，保持 `crate::config::*` 兼容。
- 最新工程优化：新增 `smagical-storage` workspace crate，迁移原 `src/storage` 的 `StorageManager`、索引操作和 redb 快照持久化；主 crate `src/storage.rs` 改为 re-export，保持 `crate::storage::*` 兼容。
- 最新工程优化：新增 `smagical-security` workspace crate，迁移原 `src/security` 和 `src/backend/auth.rs` 的凭据解析、SecretStore、KeyringSecretStore 和 BackendAuth；主 crate 保留 `src/security.rs` 与 `src/backend/auth.rs` re-export，保持旧调用兼容。
- 最新工程优化：新增 `smagical-backend-core` workspace crate，迁移原 `src/backend` 的 BackendAuth、BackendCommand、BackendEvent、BackendCommandQueue、请求模型和 BackendExecutor 抽象；主 crate 保留同名 backend 模块 re-export，真实 SSH/PTY 执行器仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-backend-core`，迁移原 `src/backend/local_shell.rs` 的 LocalShellProfile、LocalShellKind 和 LocalShellFallbackCommand；主 crate 保留 `src/backend/local_shell.rs` re-export。
- 最新工程优化：新增 `smagical-backend-reducer` workspace crate，迁移原 `src/backend/reducer.rs` 的 BackendEventOutcome 和 apply_backend_event；主 crate 保留 `src/backend/reducer.rs` re-export，保持 `crate::backend::*` 调用兼容。
- 最新工程优化：新增 `smagical-local-backend` workspace crate，迁移原 `src/backend/local_command.rs` 和 `src/backend/local_pty` 的本地 fallback、PTY session/reader/fallback 状态机、LocalPtyBackendExecutor 与 DesktopBackendExecutor；主 crate 保留同名模块 re-export。
- 最新工程优化：新增 `smagical-ssh-plan` workspace crate，迁移原 `src/backend/ssh/plan.rs` 的 SshConnectionPlan、SshAuthPlan 和凭据错误映射；主 crate 保留 `src/backend/ssh/plan.rs` re-export，保持 `crate::backend::ssh::*` 调用兼容。
- 最新工程优化：新增 `smagical-ssh-client-core` workspace crate，迁移原 `src/backend/ssh/client/settings.rs` 的 RusshClientSettings 和 russh client config 转换；主 crate 保留 `src/backend/ssh/client/settings.rs` re-export，保持旧调用兼容。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移原 `src/backend/ssh/client/host_key.rs` 的 HostKeyPolicy、HostKeyCheck、主机密钥指纹和算法映射；主 crate 保留 `src/backend/ssh/client/host_key.rs` re-export，保持旧调用兼容。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移原 `src/backend/ssh/client/auth.rs` 中纯 ssh-agent 身份选择逻辑 select_agent_identity；真实认证流程仍留在主 crate，避免把后端错误和网络认证耦合迁入核心 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移原 `src/backend/ssh/client/handler.rs` 的 SshClientHandler、SharedHostKeyResult、SharedForwardedChannels 和 ForwardedChannel；主 crate 保留 `src/backend/ssh/client/handler.rs` re-export，真实连接器调用路径保持兼容。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移原 `src/backend/ssh/client/session.rs` 的远程命令 collect_command_message、shell_message_to_event、output_event 和退出码转换；真实 session channel 打开、PTY、SFTP 和隧道仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移原 `src/backend/ssh/client/session/sftp.rs` 的 SFTP 路径拼接、父目录推导、目录项映射和传输进度事件构造；真实 SFTP session、上传/下载和目录操作仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移原 `src/backend/ssh/client/session/tunnel/socks5.rs` 的 SOCKS5 CONNECT 握手解析和成功响应写入；真实 tunnel 监听、SSH direct-tcpip 和 TCP pipe 仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移原 `src/backend/ssh/client/session/tunnel/handle.rs` 的 RemoteTunnel 运行句柄和停止标记；真实 tunnel 启动循环、端口监听和连接转发仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移原 `src/backend/ssh/client/session/sftp.rs` 的 copy_transfer_with_progress 传输复制和进度事件生成；真实 SFTP session、文件打开、flush/shutdown 和目录刷新仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移原 `src/backend/ssh/client/session.rs` 的 SSH PTY columns/rows 尺寸转换；真实 session channel、PTY 请求和 resize 调用仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移原 `src/backend/ssh/client/session.rs`、`session/sftp.rs` 和 `session/tunnel.rs` 的 channel/sftp/tunnel 错误映射 helper；真实 request、SFTP 操作和 tunnel 启动仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移原 `src/backend/ssh/client/auth.rs` 的私钥解码、认证错误映射和 ssh-agent 身份缺失提示；真实认证流程、agent 连接和 russh handle 调用仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移原 `src/backend/ssh/client/session.rs` 的 channel request 响应状态和错误映射；真实 Channel 轮询、PTY、shell、exec 和 SFTP subsystem 请求仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移原 `src/backend/ssh/client/auth.rs` 的服务端拒绝认证错误构造；真实认证请求、AuthResult 判断和 agent 交互仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移 SSH 隧道 IO/原因错误构造 helper；真实 TCP 监听、SSH direct-tcpip、remote-forward、SOCKS5 和双向复制仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移 SSH 隧道 `copy_bidirectional` 到核心 crate 并改用内存流测试覆盖；真实 TCP 监听、SSH channel 打开和 SOCKS5 编排仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，集中 SSH 隧道内部 rule name 与 channel operation 常量；真实 TCP/SOCKS5 编排继续只引用核心常量，不再散落魔法字符串。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，集中 SSH 隧道监听/forwarded-channel 轮询 tick 常量；本地 listener accept 和远端 forwarded channel recv 使用统一空闲 tick。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移 SSH 隧道状态事件 helper；Local/Dynamic/Remote 隧道启动分支复用 `tunnel_running_event` 构造运行态事件。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移 SSH channel 生命周期事件 helper；`ShellOpened`、`RemoteCommandStarted`、`CommandExited` 和 `Disconnected` 事件构造集中到 core，真实 channel 打开、exec 和 drain 仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移 SFTP 目录事件 helper；`SftpEntries` 事件构造集中到 core，真实 `SftpSession::read_dir` 和目录项读取仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，补齐 SSH 隧道停止事件 helper；executor 停止隧道复用 `tunnel_stopped_event`，断开连接复用 `disconnected_event`，真实资源清理和停止循环仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移 SSH 连接生命周期事件 helper；`Connecting`、`HostKeyVerified`、`Authenticating`、`Authenticated` 和 `Connected` 事件构造集中到 core，真实 `russh` connect、host key policy 和认证流程仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移 SSH 连接错误 helper；`ConnectionFailed` 和 `HostKeyRejected` 错误构造集中到 core，主 crate 的 `host_key_or_connection_error` 只保留共享 host key 结果分支判断。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移 SSH 未连接会话错误 helper；executor 的 open shell、shell input、run command、SFTP 和 start tunnel 前置条件复用 `connected_session_error`，真实执行器资源查找仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移 SSH/SFTP 错误分类 helper；executor 的 shell/SFTP 缓存丢弃门改用 `is_channel_failure` 和 `is_sftp_failure`，不再直接匹配错误枚举结构。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移 ssh-agent 身份缺失认证错误 helper；真实 agent 连接、身份读取和公钥认证仍留在主 crate，`auth.rs` 不再直接构造认证错误枚举结构。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移远程 shell drain 停止判断 helper；`session.rs` 不再直接匹配 `BackendEvent::Disconnected`，但仍保持仅断开事件停止本轮 drain 的行为。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，集中 SFTP subsystem 名称和 SFTP operation 错误标签常量；真实 SFTP IO、上传/下载和目录操作仍留在主 crate，只复用 core 常量消除魔法字符串。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，集中 SSH channel/session/executor operation 错误标签常量；真实 channel 打开、PTY、shell、exec 和 executor 资源查找仍留在主 crate，只复用 core 常量保持错误标签一致。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移主机密钥策略选择 helper；`RusshConnector` 不再持有计划 known_hosts 覆盖默认策略的纯 match，真实连接和认证编排仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移 host key/connection 错误分流 helper；`RusshConnector` 不再保留被拒绝主机密钥优先于连接错误的纯分流逻辑，真实 `russh::client::connect` 调用仍留在主 crate。
- 最新工程优化：继续扩展 `smagical-ssh-client-core`，迁移 tunnel running 判断 helper；Local/Dynamic/Remote 隧道循环不再直接读取 `AtomicBool` 和 `Ordering`，真实监听、转发和 cancel 逻辑仍留在主 crate。
- 编译速度判断：当前 Rust 源文件约 138 个、总量约 904KB，文件数量不是主要慢点；更可能来自 `slint-build`、`russh`/`aws-lc-rs`、`keyring`/Windows 依赖、宏展开和测试二进制链接。
- 编译速度事实：收紧 build script 后，无代码变更重跑 `cargo test backend::event::tests -- --nocapture` 已从约 `20.93s` 降到约 `1.10s`；单 crate 有源码变更时仍会重新构建测试二进制。
- 编译速度事实：lib/bin 拆分第一步后，顺序复跑 `cargo test --lib backend::event::tests -- --nocapture` 约 `1.06s`；首次并行跑 `cargo check` 与 `cargo test --lib` 会因 Cargo 文件锁互相等待，耗时不代表缓存路径。
- 编译速度事实：`smagical-core` 独立测试 `cargo test -p smagical-core` 通过，8 个纯模型测试全部成功；主 crate `cargo test --lib model::tests -- --nocapture` 也通过。
- 编译速度事实：继续迁移纯模型后，`cargo test -p smagical-core` 通过，34 个纯模型测试全部成功；主 crate `cargo test` 通过，`509 passed, 2 ignored`，测试总量转移到 core crate 后分布发生变化。
- 编译速度事实：迁移会话运行态后，`cargo test -p smagical-session` 通过，68 个会话运行态测试全部成功；主 crate `cargo test` 通过，`441 passed, 2 ignored`，测试总量继续按 crate 分布转移。
- 编译速度事实：迁移终端核心后，`cargo test -p smagical-terminal` 通过，17 个终端核心测试全部成功；主 crate `cargo test` 通过，`424 passed, 2 ignored`，终端测试已转移到独立 crate。
- 编译速度事实：迁移配置核心后，`cargo test -p smagical-config` 通过，8 个配置测试全部成功；主 crate `cargo test` 通过，`416 passed, 2 ignored`，配置测试已转移到独立 crate。
- 编译速度事实：迁移存储核心后，`cargo test -p smagical-storage` 通过，20 个存储测试全部成功；`cargo test --lib storage -- --nocapture` 通过，4 个主 crate 存储调用面测试成功；主 crate `cargo check` 通过，用时约 `13.26s`，完整 `cargo test` 通过，`396 passed, 2 ignored`。
- 编译速度事实：迁移安全核心后，`cargo test -p smagical-security` 通过，15 个安全测试全部成功；`cargo test --lib backend::ssh::plan::tests -- --nocapture` 通过，9 个认证计划调用面测试成功；完整 `cargo test` 通过，`381 passed, 2 ignored`。
- 编译速度事实：迁移后端纯接口后，`cargo test -p smagical-backend-core` 通过，29 个纯接口测试全部成功；`cargo test --lib backend::reducer::tests -- --nocapture` 通过，34 个主 crate 调用面测试成功；完整 `cargo test` 通过，`352 passed, 2 ignored`。
- 编译速度事实：迁移本地 shell profile 后，`cargo test -p smagical-backend-core` 通过，31 个纯接口测试全部成功；`cargo test --lib local_command -- --nocapture` 通过，4 个调用面测试成功；`cargo test --lib local_pty -- --nocapture` 通过，`10 passed, 2 ignored`。
- 编译速度事实：迁移后端归约逻辑后，`cargo test -p smagical-backend-reducer` 通过，34 个归约测试全部成功；`cargo test --lib backend_pump -- --nocapture` 通过，33 个主 crate 调用面测试成功；完整 `cargo test` 通过，`316 passed, 2 ignored`。
- 编译速度事实：迁移本地后端后，`cargo test -p smagical-local-backend` 通过，`14 passed, 2 ignored`；`cargo test --lib local_terminal -- --nocapture` 通过，6 个主 crate 调用面测试成功；完整 `cargo test` 通过，`302 passed`。
- 编译速度事实：迁移 SSH 连接计划后，`cargo test -p smagical-ssh-plan` 通过，9 个连接计划测试全部成功；`cargo test --lib backend::ssh::executor::tests -- --nocapture` 通过，37 个主 crate 调用面测试成功；完整 `cargo test` 通过，`293 passed`。
- 编译速度事实：迁移 SSH 客户端配置后，`cargo test -p smagical-ssh-client-core` 通过，3 个配置测试全部成功；`cargo test --lib backend::ssh::client::tests -- --nocapture` 通过，17 个主 crate 调用面测试成功；`cargo check` 通过，用时约 `16.05s`。
- 编译速度事实：迁移 SSH 主机密钥策略后，`cargo test -p smagical-ssh-client-core` 通过，9 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client::tests -- --nocapture` 通过，11 个主 crate 调用面测试成功；`cargo check` 通过，用时约 `13.77s`；完整 `cargo test` 通过，`287 passed`。
- 编译速度事实：迁移 ssh-agent 身份选择后，`cargo test -p smagical-ssh-client-core` 通过，11 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client::tests -- --nocapture` 通过，9 个主 crate 调用面测试成功；`cargo check` 通过，用时约 `6.85s`；完整 `cargo test` 通过，`285 passed`。
- 编译速度事实：迁移 SSH client handler 后，`cargo test -p smagical-ssh-client-core` 通过，14 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client::tests -- --nocapture` 通过，6 个主 crate 调用面测试成功；`cargo check` 缓存复验通过，用时约 `1.13s`；完整 `cargo test` 通过，`282 passed`。
- 编译速度事实：迁移 SSH channel 消息映射后，`cargo test -p smagical-ssh-client-core` 通过，23 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client::session::tests -- --nocapture` 通过，4 个主 crate session 调用面测试成功；`cargo check` 缓存复验通过，用时约 `1.18s`；完整 `cargo test` 通过，`273 passed`。
- 编译速度事实：迁移 SFTP 纯映射 helper 后，`cargo test -p smagical-ssh-client-core` 通过，26 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client::session::tests -- --nocapture` 通过，1 个主 crate session 调用面测试成功；`cargo check` 缓存复验通过，用时约 `1.12s`；完整 `cargo test` 通过，`270 passed`。
- 编译速度事实：迁移 SOCKS5 握手解析后，`cargo test -p smagical-ssh-client-core` 通过，37 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client::session::tunnel::tests -- --nocapture` 通过，1 个主 crate tunnel 调用面测试成功；`cargo check` 通过，用时约 `9.81s`；完整 `cargo test` 通过，`259 passed`。
- 编译速度事实：迁移 RemoteTunnel 运行句柄后，`cargo test -p smagical-ssh-client-core` 通过，38 个客户端核心测试全部成功；`cargo test --lib backend::ssh::executor::tests -- --nocapture` 通过，37 个主 crate executor 调用面测试成功；`cargo check` 通过，用时约 `7.36s`；完整 `cargo test` 通过，`258 passed`。
- 编译速度事实：迁移 SFTP 传输复制 helper 后，`cargo test -p smagical-ssh-client-core` 通过，42 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client::session -- --nocapture` 通过，4 个主 crate session 调用面测试成功；`cargo check` 通过，用时约 `7.51s`；完整 `cargo test` 通过，`254 passed`。
- 编译速度事实：迁移 SSH PTY 尺寸 helper 后，`cargo test -p smagical-ssh-client-core` 通过，43 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client::session -- --nocapture` 通过，3 个主 crate session 调用面测试成功；`cargo check` 通过，用时约 `14.10s`；完整 `cargo test` 通过，`253 passed`。
- 编译速度事实：迁移 SSH 错误映射 helper 后，`cargo test -p smagical-ssh-client-core` 通过，44 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client -- --nocapture` 通过，9 个主 crate SSH client 调用面测试成功；`cargo check` 通过，用时约 `6.69s`；完整 `cargo test` 通过，`253 passed`。
- 编译速度事实：迁移 SSH 认证 helper 后，`cargo test -p smagical-ssh-client-core` 通过，47 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client -- --nocapture` 通过，7 个主 crate SSH client 调用面测试成功；`cargo check` 通过，用时约 `7.52s`；完整 `cargo test` 通过，`251 passed`。
- 编译速度事实：迁移 SSH channel request helper 后，`cargo test -p smagical-ssh-client-core` 通过，49 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client::session -- --nocapture` 通过，3 个主 crate session 调用面测试成功；`cargo check` 通过，用时约 `12.24s`；完整 `cargo test` 通过，`251 passed`。
- 编译速度事实：迁移 SSH 认证拒绝 helper 后，`cargo test -p smagical-ssh-client-core` 通过，50 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client -- --nocapture` 通过，7 个主 crate SSH client 调用面测试成功；`cargo check` 通过，用时约 `6.88s`；完整 `cargo test` 通过，`251 passed`。
- 编译速度事实：迁移 SSH 隧道 IO/原因错误 helper 后，`cargo test -p smagical-ssh-client-core` 通过，50 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client::session::tunnel::tcp -- --nocapture` 通过，3 个主 crate tunnel TCP 调用面测试成功；`cargo check` 通过，用时约 `6.48s`；完整 `cargo test` 通过，`251 passed`。
- 编译速度事实：迁移 SSH 隧道双向复制 helper 后，`cargo test -p smagical-ssh-client-core` 通过，51 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client::session::tunnel::tcp -- --nocapture` 通过，2 个主 crate tunnel TCP 调用面测试成功；`cargo check` 通过，用时约 `6.85s`；完整 `cargo test` 通过，`250 passed`。
- 编译速度事实：集中 SSH 隧道内部名常量后，`cargo test -p smagical-ssh-client-core` 通过，52 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client::session::tunnel::tcp -- --nocapture` 通过，2 个主 crate tunnel TCP 调用面测试成功；`cargo check` 通过，用时约 `6.25s`；完整 `cargo test` 通过，`250 passed`。
- 编译速度事实：集中 SSH 隧道轮询 tick 常量后，`cargo test -p smagical-ssh-client-core` 通过，52 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client::session::tunnel -- --nocapture` 通过，2 个主 crate tunnel 调用面测试成功；`cargo check` 通过，用时约 `7.43s`；完整 `cargo test` 通过，`250 passed`。
- 编译速度事实：迁移 SSH 隧道状态事件 helper 后，`cargo test -p smagical-ssh-client-core` 通过，53 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client::session::tunnel -- --nocapture` 通过，2 个主 crate tunnel 调用面测试成功；`cargo check` 通过，用时约 `7.18s`；完整 `cargo test` 通过，`250 passed`。
- 编译速度事实：迁移 SSH channel 生命周期事件 helper 后，`cargo test -p smagical-ssh-client-core` 通过，54 个客户端核心测试全部成功；`cargo check` 通过，用时约 `5.89s`；完整 `cargo test` 通过，`250 passed`。
- 编译速度事实：迁移 SFTP 目录事件 helper 后，`cargo test -p smagical-ssh-client-core` 通过，55 个客户端核心测试全部成功；`cargo check` 通过，用时约 `5.57s`；完整 `cargo test` 通过，`250 passed`。
- 编译速度事实：补齐 SSH 隧道停止事件 helper 后，`cargo test -p smagical-ssh-client-core` 通过，55 个客户端核心测试全部成功；`cargo test --lib backend::ssh::executor::tests -- --nocapture` 通过，37 个 executor 调用面测试成功；`cargo check` 通过，用时约 `13.65s`；完整 `cargo test` 通过，`250 passed`。
- 编译速度事实：迁移 SSH 连接生命周期事件 helper 后，`cargo test -p smagical-ssh-client-core` 通过，56 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client::tests -- --nocapture` 通过，4 个 SSH client 调用面测试成功；`cargo check` 通过，用时约 `7.89s`；完整 `cargo test` 通过，`250 passed`。
- 编译速度事实：迁移 SSH 连接错误 helper 后，`cargo test -p smagical-ssh-client-core` 通过，56 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client::tests -- --nocapture` 通过，4 个 SSH client 调用面测试成功；`cargo check` 通过，用时约 `7.18s`；完整 `cargo test` 通过，`250 passed`。
- 编译速度事实：迁移 SSH 未连接会话错误 helper 后，`cargo test -p smagical-ssh-client-core` 通过，56 个客户端核心测试全部成功；`cargo test --lib backend::ssh::executor::tests -- --nocapture` 通过，36 个 executor 调用面测试成功；`cargo check` 通过，用时约 `6.93s`；完整 `cargo test` 通过，`249 passed`。
- 编译速度事实：迁移 SSH/SFTP 错误分类 helper 后，`cargo test -p smagical-ssh-client-core` 通过，56 个客户端核心测试全部成功；`cargo test --lib backend::ssh::executor::tests -- --nocapture` 通过，36 个 executor 调用面测试成功；`cargo check` 通过，用时约 `7.90s`；完整 `cargo test` 通过，`249 passed`。
- 编译速度事实：迁移 ssh-agent 身份缺失认证错误 helper 后，`cargo test -p smagical-ssh-client-core agent_identity -- --nocapture` 通过，4 个 agent 身份相关测试成功；`cargo test -p smagical-ssh-client-core` 通过，57 个客户端核心测试全部成功；`cargo check` 通过，用时约 `8.05s`；`cargo fmt --check` 通过；完整 `cargo test` 通过，`249 passed`。
- 编译速度事实：迁移远程 shell drain 停止判断 helper 后，`cargo test -p smagical-ssh-client-core shell_drain -- --nocapture` 通过，1 个 drain 语义测试成功；`cargo test -p smagical-ssh-client-core` 通过，58 个客户端核心测试全部成功；`cargo check` 通过，用时约 `6.18s`；`cargo fmt --check` 通过；完整 `cargo test` 通过，`249 passed`。
- 编译速度事实：集中 SFTP operation 常量后，首次 `cargo fmt --check` 发现导入排序和链式调用格式差异，已运行 `cargo fmt` 修复；`cargo test -p smagical-ssh-client-core sftp_operation_names -- --nocapture` 通过，1 个常量稳定性测试成功；`cargo test -p smagical-ssh-client-core` 通过，59 个客户端核心测试全部成功；`cargo check` 通过，用时约 `5.58s`；`cargo fmt --check` 复验通过；完整 `cargo test` 通过，`249 passed`。
- 编译速度事实：集中 SSH channel/executor operation 常量后，首次 `cargo fmt --check` 发现导出列表格式差异，已运行 `cargo fmt` 修复；`cargo test -p smagical-ssh-client-core ssh_channel_operation_names -- --nocapture` 通过，1 个常量稳定性测试成功；`cargo test -p smagical-ssh-client-core` 通过，60 个客户端核心测试全部成功；`cargo check` 通过，用时约 `5.65s`；`cargo fmt --check` 复验通过；完整 `cargo test` 通过，`249 passed`。
- 编译速度事实：迁移主机密钥策略选择 helper 后，首次 `cargo fmt --check` 发现一个多余空行，已运行 `cargo fmt` 修复；`cargo test -p smagical-ssh-client-core host_key_policy_for_known_hosts -- --nocapture` 通过，3 个策略选择测试成功；`cargo test -p smagical-ssh-client-core` 通过，63 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client::tests -- --nocapture` 通过，3 个 SSH client 调用面测试成功；`cargo check` 通过，用时约 `6.34s`；`cargo fmt --check` 复验通过；完整 `cargo test` 通过，`248 passed`，减少的 1 个主 crate 测试已迁入 core。
- 编译速度事实：迁移 host key/connection 错误分流 helper 后，首次 `cargo fmt --check` 发现导出列表格式差异，已运行 `cargo fmt` 修复；首次 SSH client 调用面测试发现 `disconnect()` 仍需 `connection_error` 导入，已补回；`cargo test -p smagical-ssh-client-core ssh_error_helpers -- --nocapture` 通过，1 个错误 helper 聚焦测试成功；`cargo test -p smagical-ssh-client-core` 通过，63 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client::tests -- --nocapture` 修复后通过，1 个 SSH client 调用面测试成功；`cargo check` 通过，用时约 `5.76s`；`cargo fmt --check` 复验通过；完整 `cargo test` 通过，`246 passed`，减少的 2 个主 crate 测试已迁入 core。
- 编译速度事实：迁移 tunnel running 判断 helper 后，首次 `cargo fmt --check` 发现导出列表格式差异，已运行 `cargo fmt` 修复；`cargo test -p smagical-ssh-client-core tunnel_running -- --nocapture` 通过，1 个 running flag 测试成功；`cargo test -p smagical-ssh-client-core` 通过，64 个客户端核心测试全部成功；`cargo test --lib backend::ssh::client::session::tunnel -- --nocapture` 通过，2 个 tunnel 调用面测试成功；`cargo check` 通过，用时约 `6.30s`；`cargo fmt --check` 复验通过；完整 `cargo test` 通过，`246 passed`。
- 编译速度事实：本轮 `cargo check` 通过，`cargo test` 通过，`cargo fmt --check` 通过，`git diff --check` 仅有 CRLF 提示，无实际 diff 错误。
- 覆盖率事实：本地 `llvm-cov` 有效 profile 合并后整体行覆盖率约 `85.72%`，不是 100%；核心状态管理、SessionManager、SFTP/transfer/tunnel 管理大多已接近 98%+，低覆盖主要集中在真实 SSH 执行适配层、tunnel TCP/SOCKS5 运行路径和交互式 local PTY。

## 最近提交

- 本轮提交：拆出 tunnel running 判断 helper 并整理恢复记录
- `f378f36 拆出 host key 错误分流 helper`
- `3498693 拆出主机密钥策略选择 helper`
- `33f5c6f 集中 SSH operation 常量`
- `1db177c 集中 SFTP operation 常量`
- `b2b2f60 拆出远程 shell drain 停止判断`
- `dbfa6fd 拆出 ssh-agent 身份缺失错误 helper`
- `5c32e2c 拆出 SSH/SFTP 错误分类 helper`
- `cc76e3a 拆出 SSH 未连接会话错误 helper`
- `dcf9fb0 拆出 SSH 连接错误 helper`
- `f537119 拆出 SSH 连接生命周期事件 helper`
- `48a55b5 拆出 SSH 隧道停止事件 helper`
- `3c14cef 拆出 SFTP 目录事件 helper`
- `ef8ff7b 拆出 SSH channel 生命周期事件 helper`
- `e737a4b 拆出 SSH 隧道状态事件 helper`
- `565988c 集中 SSH 隧道轮询 tick 常量`
- `3df39c5 集中 SSH 隧道内部名常量`
- `d29ddb2 拆出 SSH 隧道双向复制 helper`
- `8eecb96 拆出 SSH 隧道错误 helper`
- `fcb67cc 拆出 SSH 认证拒绝 helper`
- `3587b7e 拆出 SSH channel request helper`
- `64dc788 拆出 SSH 认证 helper`
- `1a03691 拆出 SSH 错误映射 helper`
- `3785fda 拆出 SSH PTY 尺寸 helper`
- `fa78a67 拆出 SFTP 传输复制 helper`
- `ab83870 拆出 RemoteTunnel 运行句柄`
- `ec43bdc 拆出 SOCKS5 握手解析`
- `421734d 拆出 SFTP 纯映射 helper`
- `c73398d 拆出 SSH channel 消息映射`
- `2ba1090 拆出 SSH client handler`
- `15de3d0 稳定 Slint 构建脚本监听输出`
- `cb04901 收紧 Slint 构建脚本重跑范围`
- `227d50f 补齐后端事件口径边界测试`
- `cdfa259 补齐执行器抽象边界测试`
- `1d09201 补齐后端命令路由边界测试`
- `8ceffbc 补齐认证模型转换边界测试`
- `a71d552 补齐认证材料解析边界测试`
- `a893647 补齐 SSH 连接计划边界测试`
- `9884ad5 补齐 SSH executor 幂等边界测试`
- `3428f72 补齐 SSH handler 共享状态测试`
- `6305762 补齐 SOCKS5 隧道错误路径测试`
- `80238fd 继续收紧 SSH executor 纯 helper 边界`
- `45b82dc 继续收紧 SSH client/session/SFTP 离线边界`
- `64c6e12 补齐本地 PTY 执行器边界测试`
- `9d2ec1b 补齐本地 PTY fallback 状态测试`
- `a8280cb 抽出本地 PTY reader 读取逻辑`
- `b4cfe55 收紧 SOCKS5 方法协商`
- `1fa813e 集中 SFTP 传输进度复制`
- `e0f74d9 补齐隧道 TCP 基础测试`
- `6fc8a14 解耦 SOCKS5 握手解析`
- `c9f6281 集中会话终止收敛`
- `54999ce 隔离终态隧道事件`
- `7b598ba 隔离连接生命周期事件`
- `3552506 隔离命令退出事件`
- `fa19a04 隔离 shell 打开事件`
- `9596e61 隔离远程命令启动事件`
- `e78f7cd 阻止终态隧道命令`
- `a1440a1 收尾跳过的命令历史`
- `d66767a 锁住关闭隧道断开清理`
- `e636b5c 裁剪失效连接尾部命令`
- `e889118 跳过终态连接命令`
- `85e6ffd 跳过终态 shell 启动命令`
- `dac789b 跳过终态隧道停止命令`
- `96530e9 跳过终态隧道启动命令`

## 当前仓库状态

- 分支：`dev`
- 远端进度：本轮提交前领先 `origin/dev` 210 个提交；本轮提交后预计领先 211 个提交
- 最近验证：
  - `cargo test -p smagical-ssh-client-core ssh_error_helpers -- --nocapture` 首次失败，原因是测试在 `matches!` 部分移动错误字段后又借用原错误；调整断言顺序后通过，`1 passed`
  - `cargo test --lib backend::ssh::executor::tests -- --nocapture` 通过，`36 passed`
  - `cargo test -p smagical-ssh-client-core` 通过，`56 passed`
  - `cargo check` 通过，用时约 `7.90s`
  - `cargo test` 通过，`249 passed`
  - `cargo fmt --check` 通过
  - `git diff --check` 通过，仅有 Windows CRLF 提示
  - `cargo test -p smagical-ssh-client-core ssh_error_helpers -- --nocapture` 通过，`1 passed`
  - `cargo test --lib backend::ssh::executor::tests -- --nocapture` 通过，`36 passed`
  - `cargo test -p smagical-ssh-client-core` 通过，`56 passed`
  - `cargo check` 通过，用时约 `6.93s`
  - `cargo test` 通过，`249 passed`
  - `cargo fmt --check` 通过
  - `git diff --check` 通过，仅有 Windows CRLF 提示
  - `cargo test -p smagical-ssh-client-core ssh_error_helpers -- --nocapture` 首次失败，原因是 `HostKeyVerification::Mismatch` 是带字段变体；补 expected/actual 后通过，`1 passed`
  - `cargo test -p smagical-ssh-client-core` 通过，`56 passed`
  - `cargo test --lib backend::ssh::client::tests -- --nocapture` 通过，`4 passed`
  - `cargo check` 通过，用时约 `7.18s`
  - `cargo test` 通过，`250 passed`
  - `cargo fmt --check` 通过
  - `git diff --check` 通过，仅有 Windows CRLF 提示
  - `cargo test -p smagical-ssh-client-core connection_lifecycle_events -- --nocapture` 通过，`1 passed`
  - `cargo test -p smagical-ssh-client-core` 通过，`56 passed`
  - `cargo test --lib backend::ssh::client::tests -- --nocapture` 通过，`4 passed`
  - `cargo check` 通过，用时约 `7.89s`
  - `cargo test` 通过，`250 passed`
  - `cargo fmt --check` 通过
  - `git diff --check` 通过，仅有 Windows CRLF 提示
  - `cargo test -p smagical-ssh-client-core tunnel_status_events -- --nocapture` 通过，`1 passed`
  - `cargo test --lib backend::ssh::executor::tests -- --nocapture` 通过，`37 passed`
  - `cargo test -p smagical-ssh-client-core` 通过，`55 passed`
  - `cargo check` 通过，用时约 `13.65s`
  - `cargo test` 通过，`250 passed`
  - `cargo fmt --check` 通过
  - `git diff --check` 通过，仅有 Windows CRLF 提示
  - `cargo test -p smagical-ssh-client-core sftp_entries_event -- --nocapture` 首次失败，原因是新增测试缺少 `SftpEntry` import；补 import 后通过，`1 passed`
  - `cargo test -p smagical-ssh-client-core` 通过，`55 passed`
  - `cargo check` 通过，用时约 `5.57s`
  - `cargo test` 通过，`250 passed`
  - `cargo fmt --check` 通过
  - `git diff --check` 通过，仅有 Windows CRLF 提示
  - `cargo test -p smagical-ssh-client-core channel_lifecycle_events -- --nocapture` 通过，`1 passed`
  - `cargo test -p smagical-ssh-client-core` 通过，`54 passed`
  - `cargo check` 通过，用时约 `5.89s`
  - `cargo test` 通过，`250 passed`
  - `cargo fmt --check` 通过
  - `git diff --check` 通过，仅有 Windows CRLF 提示
  - `cargo test local_pty -- --nocapture` 通过，`10 passed, 2 ignored`
  - `cargo test` 通过，`503 passed, 2 ignored`
  - `cargo fmt --check` 通过
  - `cargo test fallback -- --nocapture` 通过，`10 passed, 1 ignored`
  - `cargo test` 通过，`500 passed, 2 ignored`
  - `cargo fmt --check` 通过
  - `cargo test read_events_from_stream -- --nocapture` 通过，`3 passed`
  - `cargo test` 通过，`496 passed, 2 ignored`
  - `cargo fmt --check` 通过
  - `cargo test socks5 -- --nocapture` 通过，`7 passed`
  - `cargo test` 通过，`493 passed, 2 ignored`
  - `cargo fmt --check` 通过
  - `cargo test copy_transfer_with_progress -- --nocapture` 通过，`3 passed`
  - `cargo test` 通过，`492 passed, 2 ignored`
  - `cargo fmt --check` 通过
  - `cargo test tunnel::tcp -- --nocapture` 通过，`3 passed`
  - `cargo test` 通过，`489 passed, 2 ignored`
  - `cargo fmt --check` 通过
  - `cargo test socks5 -- --nocapture` 通过，`6 passed`
  - `cargo test` 通过，`486 passed, 2 ignored`
  - `cargo fmt --check` 通过
  - `llvm-cov report` 基于有效 `.profraw` 合并后生成覆盖率摘要，整体行覆盖率约 `85.72%`
  - `cargo test failed_session_status_collects_sftp_and_tunnel_runtime -- --nocapture` 通过，`1 passed`
  - `cargo test disconnected_session_status_collects_sftp_and_tunnel_runtime -- --nocapture` 通过，`1 passed`
  - `cargo test backend::reducer::tests` 通过，`34 passed`
  - `cargo test session::tabs::tests` 通过，`21 passed`
  - `cargo test session::sftp::tests` 通过，`24 passed`
  - `cargo test session::tunnels::tests` 通过，`13 passed`
  - `cargo test session::transfers::tests` 通过，`8 passed`
  - `cargo fmt --check` 通过
  - `cargo check` 通过
- `cargo test model::app_state::backend_pump_tests` 通过，`33 passed`
- `cargo test` 通过，`480 passed, 2 ignored`
- `cargo test backend::ssh::client::tests -- --nocapture` 通过，`15 passed`
- `cargo test backend::ssh::client::session::tests -- --nocapture` 通过，`13 passed`
- `cargo test backend::ssh::client::session::sftp -- --nocapture` 通过，`4 passed`
- `cargo fmt --check` 通过
- `cargo test` 通过，`512 passed, 2 ignored`
- `cargo test backend::ssh::executor::tests -- --nocapture` 通过，`34 passed`
- `cargo test backend::ssh::client::session::tunnel::tests -- --nocapture` 通过，`12 passed`
- `cargo test backend::ssh::client::tests -- --nocapture` 通过，`17 passed`
- `cargo test backend::ssh::executor::tests -- --nocapture` 通过，`37 passed`
- `cargo test backend::ssh::plan::tests -- --nocapture` 通过，`9 passed`
- `cargo test security::resolver::tests -- --nocapture` 通过，`9 passed`
- `cargo test backend::auth::tests -- --nocapture` 通过，`4 passed`
- `cargo test backend::command::tests -- --nocapture` 通过，`6 passed`
- `cargo test backend::executor::tests -- --nocapture` 通过，`7 passed`
- `cargo test backend::event::tests -- --nocapture` 通过，`5 passed`
- `cargo check` 通过，用时约 `11.55s`
- `cargo test backend::event::tests -- --nocapture` 通过，`5 passed`，测试构建用时约 `20.93s`
- 无代码变更重跑 `cargo test backend::event::tests -- --nocapture` 通过，`5 passed`，完成约 `1.10s`
- `cargo check` 通过，用时约 `7.93s`
- `cargo test backend::event::tests -- --nocapture` 通过，`5 passed`，本次 build.rs 排序改动后测试构建用时约 `14.01s`
- `cargo check` 通过；本次与 `cargo test --lib` 并行运行，因 Cargo 文件锁等待总耗时约 `58.37s`
- `cargo test --lib backend::event::tests -- --nocapture` 通过，`5 passed`；本次与 `cargo check` 并行运行，因 Cargo 文件锁等待总耗时约 `43.16s`
- 顺序复跑 `cargo test --lib backend::event::tests -- --nocapture` 通过，`5 passed`，完成约 `1.06s`
- `cargo test -p smagical-core` 通过，`8 passed`
- `cargo test --lib model::tests -- --nocapture` 通过，`5 passed`
- `cargo check` 通过，用时约 `14.77s`
- `cargo test` 通过，`535 passed, 2 ignored`
- `cargo test -p smagical-core` 通过，`34 passed`
- `cargo test --lib model::tests -- --nocapture` 通过，`5 passed`
- `cargo check` 通过；本次与 `cargo test` 并行运行，因 Cargo 文件锁等待总耗时约 `41.88s`
- `cargo test` 通过，`509 passed, 2 ignored`
- `cargo test -p smagical-session` 通过，`68 passed`
- `cargo test --lib model::app_state::workspace -- --nocapture` 通过，`6 passed`
- `cargo check` 通过，用时约 `13.47s`
- `cargo test` 通过，`441 passed, 2 ignored`
- `cargo test -p smagical-terminal` 通过，`17 passed`
- `cargo test --lib terminal -- --nocapture` 通过，`54 passed`
- `cargo check` 通过，用时约 `15.12s`
- `cargo test` 通过，`424 passed, 2 ignored`
- `cargo test -p smagical-config` 通过，`8 passed`
- `cargo test --lib config -- --nocapture` 通过，`4 passed`
- `cargo check` 通过，用时约 `14.20s`
- `cargo test` 通过，`416 passed, 2 ignored`
- `cargo test -p smagical-storage` 通过，`20 passed`
- `cargo test --lib storage -- --nocapture` 通过，`4 passed`
- `cargo check` 通过，用时约 `13.26s`
- `cargo test` 通过，`396 passed, 2 ignored`
- `cargo test -p smagical-security` 通过，`15 passed`
- `cargo check` 通过，用时约 `14.25s`
- `cargo test --lib backend::ssh::plan::tests -- --nocapture` 通过，`9 passed`
- `cargo test --lib backend::command::tests -- --nocapture` 通过，`6 passed`
- `cargo fmt --check` 通过
- `cargo test` 通过，`381 passed, 2 ignored`
- `cargo test -p smagical-backend-core` 通过，`29 passed`
- `cargo check` 通过，用时约 `13.09s`
- `cargo test --lib backend::reducer::tests -- --nocapture` 通过，`34 passed`
- `cargo fmt --check` 通过
- `cargo test` 通过，`352 passed, 2 ignored`
- `cargo test -p smagical-backend-core` 通过，`31 passed`
- `cargo test --lib local_command -- --nocapture` 通过，`4 passed`
- `cargo test --lib local_pty -- --nocapture` 通过，`10 passed, 2 ignored`
- `cargo fmt --check` 通过
- `cargo test -p smagical-backend-reducer` 通过，`34 passed`
- `cargo check` 通过，用时约 `13.81s`
- `cargo test --lib backend_pump -- --nocapture` 通过，`33 passed`
- `cargo fmt --check` 通过
- `cargo test` 通过，`316 passed, 2 ignored`
- `cargo test -p smagical-local-backend` 通过，`14 passed, 2 ignored`
- `cargo check` 通过，用时约 `14.16s`
- `cargo test --lib local_terminal -- --nocapture` 通过，`6 passed`
- `cargo fmt --check` 通过
- `cargo test` 通过，`302 passed`
- `cargo test -p smagical-ssh-plan` 通过，`9 passed`
- `cargo check` 通过，用时约 `13.50s`
- `cargo test --lib backend::ssh::executor::tests -- --nocapture` 通过，`37 passed`
- `cargo fmt --check` 通过
- `cargo test` 通过，`293 passed`
- `cargo test -p smagical-ssh-client-core` 通过，`3 passed`
- `cargo test --lib backend::ssh::client::tests -- --nocapture` 通过，`17 passed`
- `cargo check` 通过，用时约 `16.05s`
- `cargo test -p smagical-ssh-client-core` 通过，`9 passed`
- `cargo test --lib backend::ssh::client::tests -- --nocapture` 通过，`11 passed`
- `cargo check` 通过，用时约 `13.77s`
- `cargo test` 通过，`287 passed`
- `cargo test -p smagical-ssh-client-core` 通过，`11 passed`
- `cargo test --lib backend::ssh::client::tests -- --nocapture` 通过，`9 passed`
- `cargo check` 通过，用时约 `6.85s`
- `cargo test` 通过，`285 passed`
- `cargo test -p smagical-ssh-client-core` 通过，`14 passed`
- `cargo test --lib backend::ssh::client::tests -- --nocapture` 通过，`6 passed`
- `cargo check` 通过，用时约 `1.13s`
- `cargo test` 通过，`282 passed`
- `cargo test -p smagical-ssh-client-core` 通过，`23 passed`
- `cargo test --lib backend::ssh::client::session::tests -- --nocapture` 通过，`4 passed`
- `cargo check` 通过，用时约 `1.18s`
- `cargo test` 通过，`273 passed`
- `cargo test -p smagical-ssh-client-core` 通过，`26 passed`
- `cargo test --lib backend::ssh::client::session::tests -- --nocapture` 通过，`1 passed`
- `cargo check` 缓存复验通过，用时约 `1.12s`
- `cargo test` 通过，`270 passed`
- `cargo test -p smagical-ssh-client-core` 通过，`37 passed`
- `cargo test --lib backend::ssh::client::session::tunnel::tests -- --nocapture` 通过，`1 passed`
- `cargo check` 通过，用时约 `9.81s`
- `cargo test` 通过，`259 passed`
- `cargo test -p smagical-ssh-client-core` 通过，`38 passed`
- `cargo test --lib backend::ssh::executor::tests -- --nocapture` 通过，`37 passed`
- `cargo check` 通过，用时约 `7.36s`
- `cargo test` 通过，`258 passed`
- `cargo test -p smagical-ssh-client-core` 通过，`42 passed`
- `cargo test --lib backend::ssh::client::session -- --nocapture` 通过，`4 passed`
- `cargo check` 通过，用时约 `7.51s`
- `cargo test` 通过，`254 passed`
- `cargo test -p smagical-ssh-client-core` 通过，`43 passed`
- `cargo test --lib backend::ssh::client::session -- --nocapture` 通过，`3 passed`
- `cargo check` 通过，用时约 `14.10s`
- `cargo test` 通过，`253 passed`
- `cargo test -p smagical-ssh-client-core` 通过，`44 passed`
- `cargo test --lib backend::ssh::client -- --nocapture` 通过，`9 passed`
- `cargo check` 通过，用时约 `6.69s`
- `cargo test` 通过，`253 passed`
- `cargo test -p smagical-ssh-client-core` 通过，`47 passed`
- `cargo test --lib backend::ssh::client -- --nocapture` 通过，`7 passed`
- `cargo check` 通过，用时约 `7.52s`
- `cargo test` 通过，`251 passed`
- `cargo test -p smagical-ssh-client-core channel_request -- --nocapture` 通过，`2 passed`
- `cargo test -p smagical-ssh-client-core` 通过，`49 passed`
- `cargo test --lib backend::ssh::client::session -- --nocapture` 通过，`3 passed`
- `cargo check` 通过，用时约 `12.24s`
- `cargo test` 通过，`251 passed`
- `cargo test -p smagical-ssh-client-core authentication -- --nocapture` 通过，`3 passed`
- `cargo test -p smagical-ssh-client-core` 通过，`50 passed`
- `cargo test --lib backend::ssh::client -- --nocapture` 通过，`7 passed`
- `cargo check` 通过，用时约 `6.88s`
- `cargo test` 通过，`251 passed`
- `cargo test -p smagical-ssh-client-core ssh_error_helpers -- --nocapture` 通过，`1 passed`
- `cargo test -p smagical-ssh-client-core` 通过，`50 passed`
- `cargo test --lib backend::ssh::client::session::tunnel::tcp -- --nocapture` 通过，`3 passed`
- `cargo check` 通过，用时约 `6.48s`
- `cargo test` 通过，`251 passed`
- `cargo test -p smagical-ssh-client-core copy_bidirectional -- --nocapture` 通过，`1 passed`
- `cargo test -p smagical-ssh-client-core` 通过，`51 passed`
- `cargo test --lib backend::ssh::client::session::tunnel::tcp -- --nocapture` 通过，`2 passed`
- `cargo check` 通过，用时约 `6.85s`
- `cargo test` 通过，`250 passed`
- `cargo test -p smagical-ssh-client-core tunnel_internal_names -- --nocapture` 通过，`1 passed`
- `cargo test -p smagical-ssh-client-core` 通过，`52 passed`
- `cargo test --lib backend::ssh::client::session::tunnel::tcp -- --nocapture` 通过，`2 passed`
- `cargo check` 通过，用时约 `6.25s`
- `cargo test` 通过，`250 passed`
- `cargo test -p smagical-ssh-client-core tunnel_internal_names -- --nocapture` 通过，`1 passed`
- `cargo test -p smagical-ssh-client-core` 通过，`52 passed`
- `cargo test --lib backend::ssh::client::session::tunnel -- --nocapture` 通过，`2 passed`
- `cargo check` 通过，用时约 `7.43s`
- `cargo test` 通过，`250 passed`
- `cargo test -p smagical-ssh-client-core tunnel_status_events -- --nocapture` 通过，`1 passed`
- `cargo test -p smagical-ssh-client-core` 通过，`53 passed`
- `cargo test --lib backend::ssh::client::session::tunnel -- --nocapture` 通过，`2 passed`
- `cargo check` 通过，用时约 `7.18s`
- `cargo test` 通过，`250 passed`
- `git diff --check` 通过，仅 Windows CRLF 提示
- BOM 与中文抽样检查通过

## 提交要求

- commit 首行使用中文总结。
- commit body 使用中英双文。
- commit body 必须记录测试结果。

## 下一步建议

1. 继续检查真实运行时资源清理和错误状态收敛。
2. 继续补真实 SSH 执行适配层的可离线测试，优先 tunnel TCP/SOCKS5 数据转发、SFTP 传输执行路径和 local PTY 可测试边界。
3. 做本地应用预览和核心烟测：本地终端、SSH shell、远程命令、SFTP、隧道。
4. 烟测通过后再进入 UI 调整阶段。

## 本轮恢复与提交前整理

- 目标：继续核心模块化拆分，优先拆分和整理，保持模块化、功能化、单一职责。
- 已完成：在 `smagical-ssh-client-core` 中补充 `tunnel_failure_parts`，统一提取隧道失败错误中的规则名和原因。
- 已完成：主 crate 的 tunnel TCP 测试改为通过 core helper 判断错误，不再直接解构 `BackendExecutionError::TunnelFailed`。
- 已完成：core 测试补充 `tunnel_failure_parts` 正反例，确认只识别隧道失败错误。
- 验证记录：
  - `cargo test -p smagical-ssh-client-core ssh_error_helpers -- --nocapture` 通过，`1 passed`
  - `cargo test --lib backend::ssh::client::session::tunnel -- --nocapture` 通过，`2 passed`
  - `cargo test -p smagical-ssh-client-core` 通过，`64 passed`
  - `cargo check` 通过，约 `4.38s`
  - `cargo fmt --check` 通过
  - `cargo test` 通过，`246 passed`
- 下一步：完成差异检查、编码抽样和提交；后续继续整理主 crate 中剩余直接构造或匹配 backend 事件、错误的测试边界。

## 本轮核心拆分：错误详情提取 helper

- 目标：继续收敛主 crate 对后端错误枚举结构的直接依赖，把可内存测试的错误构造和字段提取放回 core。
- 已完成：在 `smagical-ssh-client-core` 增加 `channel_reason_error`，让通用 SSH channel reason 错误构造复用同一入口。
- 已完成：在 `smagical-ssh-client-core` 增加 `channel_failure_parts` 和 `sftp_failure_parts`，统一提取 `operation` / `reason`。
- 已完成：`src/backend/ssh/executor/tests.rs` 的未连接会话断言改用 `channel_failure_parts`，cache/drop gate 测试样本改用 `channel_reason_error` / `sftp_error`，不再直接构造 `ChannelFailed` / `SftpFailed`。
- 验证记录：
  - `cargo test -p smagical-ssh-client-core ssh_error_helpers -- --nocapture` 通过，`1 passed`
  - `cargo test --lib backend::ssh::executor -- --nocapture` 通过，`36 passed`
  - `cargo check` 通过，约 `6.50s`
  - `cargo test` 通过，`246 passed`
  - `cargo fmt --check` 通过
  - `git diff --check` 通过，仅 Windows CRLF 提示
  - BOM 与中文抽样检查通过
- 下一步：继续清理主 crate 中剩余直接依赖 `BackendExecutionError` / `BackendEvent` 具体结构的测试断言；真实 SSH/SFTP/TCP IO 编排仍留在主 crate。

## 本轮模块化拆分：SSH executor cache

- 目标：回应“模块化、功能化、单一化”的要求，从 helper 拆分转向能力模块拆分；先拆 SSH executor 中最独立的运行态缓存管理。
- 已完成：新增 `src/backend/ssh/executor/cache.rs`，集中管理连接、shell、SFTP、tunnel 缓存取出、替换、隧道所有权判断、失败后缓存清理和 shell terminal 事件 drop gate。
- 已完成：`src/backend/ssh/executor.rs` 移除缓存内部实现，只通过 `cache` 模块调用，保留执行器入口、命令分发和真实 SSH/SFTP/tunnel 编排。
- 已完成：缓存管理相关测试迁入 `executor::cache::tests`；`executor::tests` 只保留执行器对外行为测试。
- 验证记录：
  - `cargo test --lib backend::ssh::executor -- --nocapture` 通过，`36 passed`
  - `cargo check` 通过，约 `10.81s`
  - `cargo test` 通过，`246 passed`
  - `cargo fmt --check` 通过
  - `git diff --check` 通过，仅 Windows CRLF 提示
  - BOM 与中文抽样检查通过
- 下一步：继续按能力拆 `executor`，优先把 shell / SFTP / tunnel runtime 分别拆成单一职责模块，避免后续加功能继续堆到 `executor.rs`。

## 本轮能力模块拆分：SSH executor runtime

- 目标：继续按“模块化、功能化、单一化”拆分 executor，让 `executor.rs` 变成薄分发层。
- 已完成：新增 `session_runtime.rs`，承载 connect / disconnect 和会话资源关闭生命周期。
- 已完成：新增 `shell_runtime.rs`，承载 open shell、send shell input、drain output、run command 和 detached shell input 关闭。
- 已完成：新增 `sftp_runtime.rs`，承载 SFTP 打开、请求执行、失败后缓存清理和 detached SFTP 关闭。
- 已完成：新增 `tunnel_runtime.rs`，承载 start tunnel / stop tunnel 和启动隧道前的 shell/SFTP 子资源清理。
- 已完成：`executor.rs` 现在只保留结构体、构造、缓存计数和 `BackendCommand` 分发；测试依赖改为显式导入后端请求类型。
- 验证记录：
  - `cargo test --lib backend::ssh::executor -- --nocapture` 通过，`36 passed`
  - `cargo check` 通过，约 `1.12s`
  - `cargo test` 通过，`246 passed`
  - `cargo fmt --check` 通过
  - `git diff --check` 通过，仅 Windows CRLF 提示
  - BOM 与中文抽样检查通过
- 下一步：继续检查其它大文件是否仍是多职责聚合点，优先拆测试或 app state/backend pump 中的功能模块。

## 本轮模块细化：SSH executor cache 子模块

- 目标：继续单一职责拆分，避免刚拆出的 `executor::cache` 重新变成多职责聚合点。
- 已完成：`cache.rs` 变成薄模块入口，只负责声明和导出子模块。
- 已完成：新增 `cache/resources.rs`，集中会话资源缓存取出、替换和 runtime resources 组合。
- 已完成：新增 `cache/tunnels.rs`，集中隧道所有权、按会话取出、按规则移除、替换旧隧道和停止 detached tunnel。
- 已完成：新增 `cache/drop_gates.rs`，集中 shell/SFTP 失败后的缓存清理判断，以及 shell terminal 事件后的缓存 drop 判断。
- 已完成：对应单元测试迁入各自子模块，测试命名现在反映职责归属。
- 验证记录：
  - `cargo test --lib backend::ssh::executor -- --nocapture` 通过，`36 passed`
  - `cargo check` 通过，约 `9.26s`
  - `cargo test` 通过，`246 passed`
  - `cargo fmt --check` 通过
  - `git diff --check` 通过，仅 Windows CRLF 提示
  - BOM 与中文抽样检查通过
- 下一步：继续扫描大文件，优先拆 app_state/backend_pump 或 launch_tests 中仍然聚合的功能测试模块。

## 本轮核心拆分：AppState backend pump

- 目标：继续按“模块化、功能化、单一化”拆分核心队列泵，让 `backend_pump.rs` 只保留队列循环和执行器分发。
- 已完成：新增 `backend_pump/command_eligibility.rs`，集中判断后端命令是否仍可执行。
- 已完成：新增 `backend_pump/stale_commands.rs`，集中处理过期命令的状态收尾和队列清理。
- 已完成：新增 `backend_pump/execution_failure.rs`，集中执行器错误到 `AppUpdateOutcome`、UI error、backend event 的归约。
- 已完成：新增 `backend_pump/pending.rs`，集中失败路径下待执行命令丢弃和传输失败事件生成。
- 已完成：新增 `backend_pump/transfers.rs`，集中 SFTP 传输命令识别和传输失败事件构造。
- 已完成：新增 `backend_pump/host_keys.rs`，集中 rejected host key 的 known host 记录策略。
- 已完成：`backend_pump.rs` 从多职责实现文件瘦身为薄入口，只保留 `drain_backend_queue` 主流程。
- 备注：尝试按功能拆分 `backend_pump_tests.rs` 时，PowerShell 对新增测试目录和重写测试入口返回访问拒绝；未留下测试侧半成品，本轮先收口生产核心拆分。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo check` 通过，约 `6.97s`
  - `cargo test --lib model::app_state::backend_pump -- --nocapture` 通过，`33 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，仅 Windows CRLF 提示
  - BOM 与中文抽样检查通过
- 下一步：继续优先拆大测试文件或 `launch_sftp` / `session_tabs` 等剩余聚合点；测试侧拆分建议改用 `apply_patch` 小步进行，避免 PowerShell 批量写入权限问题。

## 本轮核心拆分：SFTP transfer 调度

- 目标：继续拆生产核心聚合点，把 `launch_sftp_transfer.rs` 从上传、下载、取消、远端操作和路径 helper 混合文件拆成按功能组织的模块。
- 已完成：`launch_sftp_transfer.rs` 变成薄模块入口，只声明并挂载 SFTP transfer 子模块。
- 已完成：新增 `sftp_transfer/upload.rs`，集中上传输入校验、传输任务创建、上传请求排队和 loading 设置。
- 已完成：新增 `sftp_transfer/download.rs`，集中下载路径校验、默认本地文件名推断、传输任务创建和下载请求排队。
- 已完成：新增 `sftp_transfer/cancel.rs`，集中 queued transfer 唯一性查找、队列命令移除、取消任务和 loading 清理判断。
- 已完成：新增 `sftp_transfer/remote_actions.rs`，集中删除远端文件和创建远端目录调度。
- 已完成：新增 `sftp_transfer/path.rs`，集中本地 basename 推断和远端 plain name 校验。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo check` 通过，约 `22.73s`
  - `cargo test --lib model::app_state::launch_tests::sftp -- --nocapture` 通过，`36 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，仅 Windows CRLF 提示
  - BOM 与中文抽样检查通过
- 下一步：继续拆 `launch_sftp.rs` 的 SFTP 浏览器/书签/会话归属逻辑，或拆 `session_tabs.rs` 的关闭、激活和运行态清理逻辑。

## 本轮核心拆分：SFTP browser 调度

- 目标：继续拆 `launch_sftp.rs`，把 SFTP 浏览器打开/导航、书签、可用会话选择从一个文件中拆开。
- 已完成：`launch_sftp.rs` 变成薄模块入口，只声明并挂载 SFTP browser 子模块。
- 已完成：新增 `sftp_browser/open.rs`，集中打开 SFTP、刷新目录、导航目录、选择条目和通用路径请求排队。
- 已完成：新增 `sftp_browser/bookmarks.rs`，集中保存、打开、删除 SFTP 书签和书签 label 推断。
- 已完成：新增 `sftp_browser/session.rs`，集中 SFTP browser owner、fallback session、claim session、错误构造和 terminal 状态判断。
- 已完成：`sftp_transfer` 子模块改为依赖新的 `launch_sftp::session` helper，保持 transfer 与 browser 的调用 seam 清晰。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo check` 通过，约 `5.57s`
  - `cargo test --lib model::app_state::launch_tests::sftp -- --nocapture` 通过，`36 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，仅 Windows CRLF 提示
  - BOM 与中文抽样检查通过
- 下一步：继续拆 `session_tabs.rs`，按关闭流程、激活流程、SFTP browser 清理、tunnel runtime 清理和 pending command 清理分模块。

## 本轮核心拆分：Session tabs 生命周期

- 目标：继续拆 `session_tabs.rs`，把关闭、激活、SFTP browser 清理、tunnel runtime 清理和 pending command 清理拆成单一职责模块。
- 已完成：`session_tabs.rs` 变成薄模块入口，只挂载生命周期子模块。
- 已完成：新增 `session_tabs/close.rs`，集中关闭标签页主流程和 disconnect 排队。
- 已完成：新增 `session_tabs/activate.rs`，集中激活标签页和 SFTP browser owner 激活时重分配。
- 已完成：新增 `session_tabs/sftp_cleanup.rs`，集中关闭 SFTP 标签页后的 browser 移除或转移。
- 已完成：新增 `session_tabs/tunnel_cleanup.rs`，集中 tunnel close gate、pending launch 判断和 runtime 移除。
- 已完成：新增 `session_tabs/pending.rs`，集中关闭标签页时待执行 backend command 移除、queued transfer 取消和是否断连判断。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo check` 通过，约 `10.30s`
  - `cargo test --lib model::app_state::tests::close -- --nocapture` 通过，`19 passed`
  - `cargo test --lib model::app_state::tests::activate -- --nocapture` 通过，`4 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，仅 Windows CRLF 提示
  - BOM 与中文抽样检查通过
- 下一步：继续扫描生产核心剩余大文件；优先拆 `ui_drafts.rs`、`workspace.rs` 或开始用小步 `apply_patch` 拆大测试文件。

## 本轮核心拆分：UI drafts 输入草稿

- 目标：继续拆 `ui_drafts.rs`，把 quick host、主机作用域输入、terminal input、本地终端初始化拆成单一职责模块。
- 已完成：`ui_drafts.rs` 变成薄模块入口，只保留 `draft_changed` 和测试用 `ensure_local_terminal_tab` re-export。
- 已完成：新增 `ui_drafts/quick_host.rs`，集中 quick host 表单、认证草稿和保存逻辑。
- 已完成：新增 `ui_drafts/host_inputs.rs`，集中远程命令草稿、SFTP 初始目录草稿和 SFTP action 草稿。
- 已完成：新增 `ui_drafts/terminal_input.rs`，集中 terminal input 草稿编辑、可打印字符过滤、发送输入、历史记录和本地 echo。
- 已完成：新增 `ui_drafts/local_terminal.rs`，集中本地终端 session/tab 确保逻辑。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo check` 通过，无 warning，约 `5.40s`
  - `cargo test --lib model::app_state::ui_drafts_tests -- --nocapture` 通过，`13 passed`
  - `cargo test --lib model::app_state::tests::send_terminal_input -- --nocapture` 通过，`2 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，仅 Windows CRLF 提示
  - BOM 与中文抽样检查通过
- 下一步：继续拆 `workspace.rs` 的保存、恢复、SFTP browser 恢复和 terminal tab 恢复逻辑，或开始小步拆测试聚合文件。

## 本轮核心拆分：Workspace 快照

- 目标：继续拆 `workspace.rs`，把保存、恢复、清除和测试分离，让工作区快照逻辑更容易扩展。
- 已完成：`workspace.rs` 变成薄模块入口，只保留默认 workspace 名称和模块挂载。
- 已完成：新增 `workspace/save.rs`，集中保存当前会话标签页、工作目录和线性布局快照。
- 已完成：新增 `workspace/restore.rs`，集中恢复 session tabs、terminal tabs 和 SFTP browsers。
- 已完成：新增 `workspace/clear.rs`，集中清除已保存工作区快照。
- 已完成：新增 `workspace/tests.rs`，迁移原有 workspace 单元测试，避免生产入口文件继续混入测试实现。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo check` 通过，约 `6.57s`
  - `cargo test --lib model::app_state::workspace -- --nocapture` 通过，`6 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，仅 Windows CRLF 提示
  - BOM 与中文抽样检查通过
- 下一步：继续扫描剩余生产聚合点，优先拆 `workspace_ui.rs`、`visual_settings.rs`、`dispatch.rs`，或者开始小步拆 `backend_pump_tests.rs` / `launch_tests/sftp.rs`。

## 本轮核心拆分：Workspace UI 调度

- 目标：继续拆 `workspace_ui.rs`，把工作区页面、分栏布局、工具面板、命令面板、背景轮播和测试分离，避免 UI 调度入口继续膨胀。
- 已完成：`workspace_ui.rs` 变成薄模块入口，只挂载 Workspace UI 子模块。
- 已完成：新增 `workspace_ui/page.rs`，集中工作区一级页面、Hosts 列表模式和 Hosts 搜索条件。
- 已完成：新增 `workspace_ui/layout.rs`，集中 Hosts 面板、活动栏、工具分栏尺寸和右侧详情栏折叠。
- 已完成：新增 `workspace_ui/tool_panel.rs`，集中 D 区域辅助分栏打开/关闭，以及 SFTP 工具面板切回 Terminal 工作区的联动。
- 已完成：新增 `workspace_ui/command_palette.rs`，集中命令面板打开、查询更新和关闭。
- 已完成：新增 `workspace_ui/background.rs`，集中背景轮播切换。
- 已完成：新增 `workspace_ui/tests.rs`，迁移原有 Workspace UI 消息测试，生产入口不再混入测试实现。
- 验证记录：
  - `cargo fmt` 已执行
  - `cargo check` 通过，约 `6.69s`
  - `cargo test --lib model::app_state::workspace_ui -- --nocapture` 通过，`3 passed`
  - `cargo test` 通过，`246 passed`
- 下一步：继续拆剩余生产聚合点，优先 `visual_settings.rs` 或 `dispatch.rs`；测试大文件后续再按领域小步迁移。

## 本轮核心拆分：Visual settings 应用逻辑

- 目标：继续拆 `visual_settings.rs`，把全局视觉设置、主机视觉覆盖和错误结果构造分离，保留消息分发入口不变。
- 已完成：`visual_settings.rs` 变成薄模块入口，只挂载视觉设置子模块。
- 已完成：新增 `visual_settings/global.rs`，集中全局视觉配置草稿更新、背景开关和应用到运行配置/持久化快照。
- 已完成：新增 `visual_settings/host.rs`，集中主机视觉覆盖草稿、背景开关、应用覆盖、清除覆盖和 fallback 推断。
- 已完成：新增 `visual_settings/outcome.rs`，集中视觉配置无效和主机缺失的错误结果构造。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo check` 通过，约 `5.78s`
  - `cargo test --lib model::app_state::visual_settings_tests -- --nocapture` 通过，`4 passed`
  - `cargo test` 通过，`246 passed`
- 下一步：继续拆剩余核心聚合点；优先评估 `dispatch.rs` 的消息分发是否按领域拆到更小路由函数，或继续整理大测试文件。

## 本轮核心拆分：Dispatch 领域路由

- 目标：继续拆 `dispatch.rs`，把单个巨大 `match` 分成按领域的路由函数，同时保留 `Message` 顶层穷尽检查。
- 已完成：`dispatch.rs` 变成顶层 `apply`、错误同步和领域分流入口。
- 已完成：新增 `dispatch/visual.rs`，集中视觉设置消息分发。
- 已完成：新增 `dispatch/workspace.rs`，集中工作区快照消息分发。
- 已完成：新增 `dispatch/ui.rs`，集中 quick host、Workspace UI、terminal input 和主机输入草稿消息分发。
- 已完成：新增 `dispatch/storage.rs`，集中 credential 和 known host 存储管理消息分发。
- 已完成：新增 `dispatch/session.rs`，集中会话标签页关闭/激活消息分发。
- 已完成：新增 `dispatch/sftp.rs`，集中 SFTP 浏览器、书签和传输消息分发。
- 已完成：新增 `dispatch/launch.rs`，集中 shell/SFTP/remote command/tunnel 启动类消息分发。
- 已完成：新增 `dispatch/snippets.rs`，集中命令片段和历史命令消息分发。
- 已完成：新增 `dispatch/backend.rs`，集中后台事件消息分发。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo check` 通过，约 `7.48s`
  - `cargo test --lib model::app_state::visual_settings_tests -- --nocapture` 通过，`4 passed`
  - `cargo test --lib model::app_state::workspace_ui -- --nocapture` 通过，`3 passed`
  - `cargo test --lib model::app_state::launch_tests::sftp -- --nocapture` 通过，`36 passed`
  - `cargo test` 通过，`246 passed`
- 下一步：生产侧核心大聚合文件已基本拆完；继续按模块化原则整理测试聚合文件，优先 `backend_pump_tests.rs` 或 `tests.rs`。

## 本轮核心拆分：Tunnel 启停调度

- 目标：继续拆 `launch_tunnel.rs`，把隧道启动、隧道停止和标签页查询辅助函数分离。
- 已完成：`launch_tunnel.rs` 变成薄模块入口，只挂载 tunnel 子模块。
- 已完成：新增 `launch_tunnel/start.rs`，集中隧道规则校验、管理标签页创建、runtime 记录、最近连接记录和 StartTunnel 后端命令排队。
- 已完成：新增 `launch_tunnel/stop.rs`，集中停止请求校验、运行态状态判断、Stopping 标记和 StopTunnel 后端命令排队。
- 已完成：新增 `launch_tunnel/lookup.rs`，集中同名隧道标签页检测和停止目标标签页匹配。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo check` 通过，约 `8.89s`
  - `cargo test --lib model::app_state::launch_tests::tunnel -- --nocapture` 通过，`12 passed`
  - `cargo test` 通过，`246 passed`
- 下一步：生产侧核心聚合点继续收尾，优先评估 `backend_events.rs` 或 `snippets.rs`；测试聚合文件随后按领域迁移。

## 本轮核心拆分：Backend events 归约入口

- 目标：继续拆 `backend_events.rs`，把后端事件应用入口、共享执行器入口和远程命令历史回写分离。
- 已完成：`backend_events.rs` 变成薄模块入口，只挂载后端事件子模块。
- 已完成：新增 `backend_events/apply.rs`，集中 `BackendEvent` 应用、远程命令终结事件预处理和共享执行器队列泵入口。
- 已完成：新增 `backend_events/remote_command_history.rs`，集中远程命令历史匹配、关闭标签页完成回写、后端事件完成回写和 duration 计算。
- 验证记录：
  - `cargo fmt` 已执行
  - `cargo fmt --check` 通过
  - `cargo check` 通过，约 `6.15s`
  - `cargo test --lib model::app_state::launch_tests::remote_command -- --nocapture` 通过，`13 passed`
  - `cargo test --lib model::app_state::tests::backend_event -- --nocapture` 通过，`1 passed`
  - `cargo test --lib model::app_state::tests::close_pending_remote_command_tab_finishes_history_without_exit_code -- --nocapture` 通过，`1 passed`
  - `cargo test` 通过，`246 passed`
- 下一步：继续生产侧核心收尾，优先 `snippets.rs`，之后再系统拆测试聚合文件。

## 本轮核心拆分：Snippets 应用行为

- 目标：继续拆 `snippets.rs`，把快捷命令保存、执行、维护和错误结果构造分离。
- 已完成：`snippets.rs` 变成薄模块入口，只挂载 snippets 子模块。
- 已完成：新增 `snippets/save.rs`，集中从主机命令草稿保存快捷命令、名称截断和模板变量推断。
- 已完成：新增 `snippets/run.rs`，集中快捷命令适用性检查、变量渲染、空命令防护和远程命令执行。
- 已完成：新增 `snippets/manage.rs`，集中快捷命令删除和变量最近输入值更新。
- 已完成：新增 `snippets/outcome.rs`，集中主机缺失和快捷命令缺失错误结果构造。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo check` 通过，约 `5.80s`
  - `cargo test --lib model::app_state::snippets_tests -- --nocapture` 通过，`10 passed`
  - `cargo test` 通过，`246 passed`
- 下一步：生产侧剩余聚合点主要是 `message.rs`、`launch_remote_command.rs`、`storage_admin.rs`、`launch.rs` 等中小文件；更大的维护压力已转向测试聚合文件，优先拆 `backend_pump_tests.rs` / `tests.rs`。

## 本轮核心拆分：Remote command 启动与历史

- 目标：继续拆 `launch_remote_command.rs`，把一次性远程命令启动、命令历史重跑/记录和后端请求构造分离。
- 已完成：`launch_remote_command.rs` 变成薄模块入口，只挂载 remote command 子模块。
- 已完成：新增 `launch_remote_command/run.rs`，集中命令空值校验、host 查询、远程命令标签页创建、terminal tab 创建、最近连接记录和 RunCommand 后端命令排队。
- 已完成：新增 `launch_remote_command/history.rs`，集中命令历史记录和带主机历史重跑。
- 已完成：新增 `launch_remote_command/request.rs`，集中 exec / PTY 远程命令请求构造。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo check` 通过，约 `6.23s`
  - `cargo test --lib model::app_state::launch_tests::remote_command -- --nocapture` 通过，`13 passed`
  - `cargo test` 通过，`246 passed`
- 下一步：生产侧继续小步收尾 `storage_admin.rs` / `launch.rs`，或开始拆测试聚合文件。

## 本轮核心拆分：Storage admin 管理操作

- 目标：继续拆 `storage_admin.rs`，把凭据元数据管理、Known Hosts 管理和测试分离。
- 已完成：`storage_admin.rs` 变成薄模块入口，只挂载 storage admin 子模块。
- 已完成：新增 `storage_admin/credential.rs`，集中凭据元数据删除和缺失错误结果。
- 已完成：新增 `storage_admin/known_hosts.rs`，集中 Known Hosts 信任标记和删除逻辑。
- 已完成：新增 `storage_admin/tests.rs`，迁移原有 storage admin 单元测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo check` 通过，约 `6.29s`
  - `cargo test --lib model::app_state::storage_admin -- --nocapture` 通过，`2 passed`
  - `cargo test` 通过，`246 passed`
- 下一步：生产侧继续收尾 `launch.rs`；之后转入测试聚合文件拆分。

## 本轮核心拆分：Launch 共享启动模块

- 目标：继续拆 `launch.rs`，把交互式 shell 启动、主机/连接命令 helper、结果构造、远端路径处理和时间函数分离。
- 已完成：`launch.rs` 变成薄模块入口，只挂载并 re-export 启动共享子模块。
- 已完成：新增 `launch/shell.rs`，集中 shell tab、terminal tab、PTY、最近连接和 OpenShell 后端命令排队。
- 已完成：新增 `launch/host.rs`，集中 host 查询、最近连接记录和带 Known Hosts 的 Connect 后端命令构造。
- 已完成：新增 `launch/outcome.rs`，集中 queued outcome 和 missing host 错误结果。
- 已完成：新增 `launch/path.rs`，集中远端目录归一化和远端路径拼接。
- 已完成：新增 `launch/time.rs`，集中 Unix 秒时间读取。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo check` 通过，约 `6.02s`
  - `cargo test --lib model::app_state::launch_tests::shell -- --nocapture` 通过，`5 passed`
  - `cargo test --lib model::app_state::launch_tests::sftp -- --nocapture` 通过，`36 passed`
  - `cargo test --lib model::app_state::launch_tests::tunnel -- --nocapture` 通过，`12 passed`
  - `cargo test --lib model::app_state::launch_tests::remote_command -- --nocapture` 通过，`13 passed`
  - `cargo test` 通过，`246 passed`
- 下一步：生产侧核心入口基本收口；继续拆测试聚合文件，优先 `backend_pump_tests.rs`、`tests.rs` 或 `launch_tests/sftp.rs`。

## 本轮测试拆分：Backend pump 基础测试

- 目标：开始拆最大测试聚合文件 `backend_pump_tests.rs`，先迁移低风险基础队列泵测试。
- 已完成：新增 `backend_pump_tests/basic.rs`，迁移基础执行、断开命令、失败裁剪、host key 拒绝、远程命令历史失败收尾和跨会话保留测试。
- 已完成：`backend_pump_tests.rs` 保留共享 fixture/executor 和其余测试，后续可继续按 command eligibility、SFTP、tunnel、local terminal 分组迁移。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::backend_pump_tests::basic -- --nocapture` 通过，`7 passed`
  - `cargo test --lib model::app_state::backend_pump_tests -- --nocapture` 通过，`33 passed`
  - `cargo test` 通过，`246 passed`
- 下一步：继续拆 `backend_pump_tests.rs`，优先迁移 terminal/stale command eligibility 相关测试。

## 本轮测试拆分：Backend pump 能力模块

- 目标：继续拆 `backend_pump_tests.rs`，按后端队列泵能力把终态命令、SFTP、隧道和本地终端测试模块化。
- 已完成：新增 `backend_pump_tests/connect.rs`，迁移 Connect 终态跳过、host 不匹配和 stale connect 尾队列裁剪测试。
- 已完成：新增 `backend_pump_tests/terminal.rs`，迁移 OpenShell、RunCommand、DrainSessionOutput 和 SendShellInput 终态跳过测试。
- 已完成：新增 `backend_pump_tests/sftp.rs`，迁移 SFTP list、transfer/write 失败清理、SFTP 操作错误和 pending SFTP 命令裁剪测试，并把 `FailingSftpExecutor` 移入该模块。
- 已完成：新增 `backend_pump_tests/tunnel.rs`，迁移隧道启动失败、终态 start/stop 跳过和 stale stop 保护测试。
- 已完成：新增 `backend_pump_tests/local_terminal.rs`，迁移本地终端输入可见性和立即清空输入草稿测试。
- 已完成：`backend_pump_tests.rs` 现在只保留共享 fixture、模块声明和空队列 no-op 测试，后续新增队列泵行为可以直接落到对应能力模块。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::backend_pump_tests::connect -- --nocapture` 通过，`4 passed`
  - `cargo test --lib model::app_state::backend_pump_tests -- --nocapture` 通过，`33 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且中文断言文本正常
- 下一步：继续拆剩余测试聚合文件，优先 `launch_tests/sftp.rs` 或 `app_state/tests.rs`。

## 本轮测试拆分：Launch SFTP 能力模块

- 目标：拆分最大启动测试聚合文件 `launch_tests/sftp.rs`，让 SFTP 启动测试按浏览器、书签、传输、取消和文件操作能力组织。
- 已完成：新增 `launch_tests/sftp/browser.rs`，迁移 OpenSftp、RefreshSftp 和 NavigateSftp 测试。
- 已完成：新增 `launch_tests/sftp/bookmark.rs`，迁移保存、打开、重开和删除 SFTP 书签测试。
- 已完成：新增 `launch_tests/sftp/transfer.rs`，迁移上传、下载、输入校验、owner 重分配和 loading 状态测试。
- 已完成：新增 `launch_tests/sftp/cancel.rs`，迁移取消 queued transfer、同 ID 防串扰、歧义拒绝和取消后 loading 清理测试。
- 已完成：新增 `launch_tests/sftp/file_actions.rs`，迁移创建目录、删除文件和路径校验测试。
- 已完成：`launch_tests/sftp.rs` 现在只保留 SFTP launch 测试模块入口。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::launch_tests::sftp::browser -- --nocapture` 通过，`7 passed`
  - `cargo test --lib model::app_state::launch_tests::sftp -- --nocapture` 通过，`36 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且中文断言文本正常
- 下一步：继续拆 `app_state/tests.rs`，优先按 close session、activate/select、backend event、terminal input 等能力分组。

## 本轮测试拆分：AppState 消息行为测试

- 目标：拆分 `app_state/tests.rs`，把根状态消息行为测试按基础状态、激活、关闭、终端输入和 SFTP 选择能力组织。
- 已完成：新增 `tests/base.rs`，迁移默认状态、BackendEvent、credential 和 known host 消息测试。
- 已完成：新增 `tests/activation.rs`，迁移 terminal/SFTP tab 激活和 SFTP browser owner 重分配测试。
- 已完成：新增 `tests/terminal_input.rs`，迁移远程终端输入排队、空命令拒绝和不可交互状态拒绝测试。
- 已完成：新增 `tests/sftp_selection.rs`，迁移 SFTP 条目选择、断连 owner 重分配和无可用会话拒绝测试。
- 已完成：新增 `tests/close_tabs.rs` 入口，并继续拆分 `tests/close_tabs/shell.rs`、`tests/close_tabs/sftp.rs`、`tests/close_tabs/tunnel.rs`，分别承载 shell/remote、SFTP、tunnel 关闭路径测试。
- 已完成：`app_state/tests.rs` 现在只保留共享 fixture 和模块入口。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::tests -- --nocapture` 通过，`34 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且中文断言文本正常
- 下一步：继续审视剩余中型测试文件 `snippets_tests.rs`、`ui_drafts_tests.rs`，或转回生产侧中小模块收尾。

## 本轮测试拆分：Snippets 测试能力模块

- 目标：让 `snippets_tests.rs` 和生产侧 `snippets/{save,run,manage}` 对齐，按保存、运行、维护能力拆分。
- 已完成：新增 `snippets_tests/save.rs`，迁移保存主机命令快捷命令、模板变量推断和空命令拒绝测试。
- 已完成：新增 `snippets_tests/run.rs`，迁移快捷命令渲染执行、跨主机拒绝、缺变量、空渲染和参数化执行测试。
- 已完成：新增 `snippets_tests/manage.rs`，迁移未知变量参数更新和删除快捷命令测试。
- 已完成：`snippets_tests.rs` 现在只保留共享 fixture 和模块入口。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::snippets_tests -- --nocapture` 通过，`10 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且中文断言文本正常
- 下一步：继续拆 `ui_drafts_tests.rs`，按 quick host、terminal input、command draft、SFTP draft 等能力分组。

## 本轮测试拆分：UI Draft 测试能力模块

- 目标：拆分 `ui_drafts_tests.rs`，按 quick host、普通草稿字段和 terminal input 能力组织测试。
- 已完成：新增 `ui_drafts_tests/quick_host.rs`，迁移 quick host 草稿、认证草稿和保存 quick host 测试。
- 已完成：新增 `ui_drafts_tests/draft_fields.rs`，迁移远程命令、SFTP 初始目录和 SFTP action 草稿字段测试。
- 已完成：新增 `ui_drafts_tests/terminal_input.rs`，迁移 terminal input 草稿编辑、本地终端可见输入、无 help banner 和空回车测试。
- 已完成：`ui_drafts_tests.rs` 现在只保留共享导入和模块入口。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::ui_drafts_tests -- --nocapture` 通过，`13 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且抽样文本正常
- 下一步：剩余较大的测试文件已基本拆完；后续可继续审视 `launch_tests/remote_command.rs`、`launch_tests/tunnel.rs` 或转回生产侧中小模块收尾。

## 本轮测试拆分：远程命令启动能力模块

- 目标：拆分 `launch_tests/remote_command.rs`，把远程命令启动、历史重跑、后端事件落盘和输入校验分别放到单一职责测试模块。
- 已完成：`launch_tests/remote_command.rs` 现在只保留共享导入和模块入口。
- 已完成：新增 `launch_tests/remote_command_launch.rs`，迁移远程命令启动、命令裁剪、历史记录和 PTY 请求测试。
- 已完成：新增 `launch_tests/remote_command_history.rs`，迁移命令历史重跑、缺失历史和无 host 全局历史拒绝测试。
- 已完成：新增 `launch_tests/remote_command_backend_events.rs`，迁移命令退出、失败、断连和 legacy history fallback 的后端事件落盘测试。
- 已完成：新增 `launch_tests/remote_command_validation.rs`，迁移空命令和缺失 host 的输入校验测试。
- 约束记录：Windows 当前拒绝在 `launch_tests` 下创建新的 `remote_command` 目录，因此本轮采用同级子文件 + `#[path = "..."]` 聚合方式，仍保持薄入口和按能力拆分。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::launch_tests::remote_command -- --nocapture` 通过，`13 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余中大型测试模块，优先 `launch_tests/tunnel.rs` 或 `launch_tests/sftp/cancel.rs`。

## 本轮测试拆分：隧道启动停止能力模块

- 目标：拆分 `launch_tests/tunnel.rs`，把隧道启动和停止路径分成单一职责测试模块。
- 已完成：`launch_tests/tunnel.rs` 现在只保留共享导入、`tunnel_rule` fixture 和模块入口。
- 已完成：新增 `launch_tests/tunnel_start.rs`，迁移隧道启动、重复规则、规则规范化和无效规则拒绝测试。
- 已完成：新增 `launch_tests/tunnel_stop.rs`，迁移隧道停止、会话匹配、运行态缺失、规则名规范化和重复停止拒绝测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::launch_tests::tunnel -- --nocapture` 通过，`12 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余中大型测试模块，优先 `launch_tests/sftp/cancel.rs` 或 `launch_tests/sftp/transfer.rs`。

## 本轮测试拆分：SFTP 取消能力模块

- 目标：拆分 `launch_tests/sftp/cancel.rs`，把通用传输取消和上传取消后的浏览器 loading 清理分离。
- 已完成：`launch_tests/sftp/cancel.rs` 现在只保留 SFTP cancel 测试模块入口。
- 已完成：新增 `launch_tests/sftp/cancel_transfer.rs`，迁移下载/通用传输取消、同 ID 跨 session 保留、歧义拒绝和已开始传输拒绝测试。
- 已完成：新增 `launch_tests/sftp/cancel_upload_loading.rs`，迁移上传取消后 loading 清理、stale session 防串扰和保留其他 refresh 请求测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::launch_tests::sftp::cancel -- --nocapture` 通过，`8 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余中大型测试模块，优先 `launch_tests/sftp/transfer.rs`。

## 本轮测试拆分：SFTP 传输能力模块

- 目标：拆分 `launch_tests/sftp/transfer.rs`，把上传和下载路径分成单一职责测试模块。
- 已完成：`launch_tests/sftp/transfer.rs` 现在只保留 SFTP transfer 测试模块入口。
- 已完成：新增 `launch_tests/sftp/transfer_upload.rs`，迁移上传请求排队、owner 重分配、输入校验、断连拒绝和 remote name 路径校验测试。
- 已完成：新增 `launch_tests/sftp/transfer_download.rs`，迁移下载请求排队、loading 保持、无效路径不重分配 owner 和空/root 路径拒绝测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::launch_tests::sftp::transfer -- --nocapture` 通过，`9 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余中型测试模块，优先 `launch_tests/sftp/cancel_upload_loading.rs`、`launch_tests/sftp/bookmark.rs` 或 `launch_tests/sftp/browser.rs`。

## 本轮测试拆分：Backend Pump SFTP 能力模块

- 目标：拆分 `backend_pump_tests/sftp.rs`，把 SFTP 队列泵的终态会话跳过、传输失败收敛和 SFTP 操作错误裁剪分离。
- 已完成：`backend_pump_tests/sftp.rs` 现在只保留 SFTP 队列泵测试模块入口和 `FailingSftpExecutor` helper。
- 已完成：新增 `backend_pump_tests/sftp_terminal.rs`，迁移终态 SFTP list/write 命令跳过和 browser error/loading 清理测试。
- 已完成：新增 `backend_pump_tests/sftp_transfer.rs`，迁移终态传输、terminal error 裁剪和 executor 不支持时的传输失败收敛测试。
- 已完成：新增 `backend_pump_tests/sftp_error.rs`，迁移 SFTP 操作错误保持会话连接、裁剪 pending transfer/write 并保留刷新命令测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::backend_pump_tests::sftp -- --nocapture` 通过，`8 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余中大型测试模块，优先 `tests/close_tabs/sftp.rs`、`tests/close_tabs/tunnel.rs` 或 `backend_pump_tests/tunnel.rs`。

## 本轮测试拆分：Close Tabs SFTP 能力模块

- 目标：拆分 `tests/close_tabs/sftp.rs`，把 SFTP 标签关闭时的 browser 生命周期、owner 重分配和 pending transfer 清理分离。
- 已完成：`tests/close_tabs/sftp.rs` 现在只保留 close tabs SFTP 测试模块入口。
- 已完成：新增 `tests/close_tabs/sftp_lifecycle.rs`，迁移最后一个 browser 删除和同 host tab 保留 browser 测试。
- 已完成：新增 `tests/close_tabs/sftp_pending.rs`，迁移 pending SFTP transfer 取消、队列清理和同 ID 跨 session 保留测试。
- 已完成：新增 `tests/close_tabs/sftp_owner.rs`，迁移关闭当前/旧 SFTP tab 后的 browser owner 重分配、loading 清理和断连 fallback 测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::tests::close_tabs::sftp -- --nocapture` 通过，`9 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余中大型测试模块，优先 `tests/close_tabs/tunnel.rs` 或 `backend_pump_tests/tunnel.rs`。

## 本轮测试拆分：Close Tabs 隧道能力模块

- 目标：拆分 `tests/close_tabs/tunnel.rs`，把隧道标签关闭时的 pending launch 清理、关闭前保护和 runtime 清理分离。
- 已完成：`tests/close_tabs/tunnel.rs` 现在只保留 close tabs tunnel 测试模块入口和共享 `tunnel_rule` fixture。
- 已完成：新增 `tests/close_tabs/tunnel_pending.rs`，迁移 pending tunnel tab 关闭、取消 launch 命令和 connected pending 后补 disconnect 测试。
- 已完成：新增 `tests/close_tabs/tunnel_guard.rs`，迁移 starting 无 pending launch 与 running tunnel 需要先停止的保护测试。
- 已完成：新增 `tests/close_tabs/tunnel_runtime.rs`，迁移只移除匹配 session runtime 和忽略其他 session running 同名规则测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::tests::close_tabs::tunnel -- --nocapture` 通过，`6 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余中大型测试模块，优先 `backend_pump_tests/tunnel.rs` 或 `backend_pump_tests/basic.rs`。

## 本轮测试拆分：Backend Pump 隧道能力模块

- 目标：拆分 `backend_pump_tests/tunnel.rs`，把隧道队列泵的 executor error、start 跳过和 stop/stale 跳过分离。
- 已完成：`backend_pump_tests/tunnel.rs` 现在只保留隧道队列泵测试模块入口。
- 已完成：新增 `backend_pump_tests/tunnel_error.rs`，迁移隧道启动命令在 executor 不支持时标记失败的测试。
- 已完成：新增 `backend_pump_tests/tunnel_start.rs`，迁移终态 start command 跳过和 session terminal 时保持 Starting 的测试。
- 已完成：新增 `backend_pump_tests/tunnel_stop.rs`，迁移终态 stop command 跳过、session terminal 时保持 Stopping 和 stale stop 防串扰测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::backend_pump_tests::tunnel -- --nocapture` 通过，`6 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余中大型测试模块，优先 `backend_pump_tests/basic.rs` 或 `launch_tests/sftp/bookmark.rs`。

## 本轮测试拆分：Backend Pump 基础能力模块

- 目标：拆分 `backend_pump_tests/basic.rs`，把基础执行、失败裁剪和 host key 错误处理分离。
- 已完成：`backend_pump_tests/basic.rs` 现在只保留基础队列泵测试模块入口。
- 已完成：新增 `backend_pump_tests/basic_execution.rs`，迁移正常执行 connect/open shell 事件应用和已关闭 tab disconnect 执行测试。
- 已完成：新增 `backend_pump_tests/basic_failure.rs`，迁移失败 session 尾命令裁剪、远程命令历史收尾和跨 session 队列保留测试。
- 已完成：新增 `backend_pump_tests/basic_host_key.rs`，迁移未知 host key 候选记录和 mismatch 不覆盖已信任 host key 测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::backend_pump_tests::basic -- --nocapture` 通过，`7 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余中大型测试模块，优先 `launch_tests/sftp/bookmark.rs`、`launch_tests/sftp/cancel_upload_loading.rs` 或 `backend_pump_tests/connect.rs`。

## 本轮测试拆分：SFTP 书签能力模块

- 目标：拆分 `launch_tests/sftp/bookmark.rs`，把 SFTP 书签保存、打开和删除路径分离。
- 已完成：`launch_tests/sftp/bookmark.rs` 现在只保留 SFTP bookmark 测试模块入口。
- 已完成：新增 `launch_tests/sftp/bookmark_save.rs`，迁移保存当前目录、断连 browser 拒绝、owner 重分配后保存和缺失 browser 报错测试。
- 已完成：新增 `launch_tests/sftp/bookmark_open.rs`，迁移无 browser 打开、已有 browser 导航和断连 browser 重开测试。
- 已完成：新增 `launch_tests/sftp/bookmark_remove.rs`，迁移删除书签和缺失书签报错测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::launch_tests::sftp::bookmark -- --nocapture` 通过，`8 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余中型测试模块，优先 `launch_tests/sftp/cancel_upload_loading.rs` 或 `backend_pump_tests/connect.rs`。

## 本轮测试拆分：SFTP 上传取消 Loading 能力模块

- 目标：拆分 `launch_tests/sftp/cancel_upload_loading.rs`，把取消上传后应清理 loading 和应保留 loading 的场景分离。
- 已完成：`launch_tests/sftp/cancel_upload_loading.rs` 现在只保留 upload loading 测试模块入口。
- 已完成：新增 `launch_tests/sftp/cancel_upload_loading_clear.rs`，迁移 queued upload 被移除后清理 loading 和忽略 stale refresh 后清理 loading 测试。
- 已完成：新增 `launch_tests/sftp/cancel_upload_loading_keep.rs`，迁移 stale upload 保留当前 browser loading 和另一个 refresh 请求仍存在时保留 loading 测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::launch_tests::sftp::cancel::upload_loading -- --nocapture` 通过，`4 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余中型测试模块，优先 `backend_pump_tests/connect.rs`、`launch_tests/sftp/transfer_upload.rs` 或 `launch_tests/sftp/browser.rs`。

## 本轮测试拆分：Backend Pump 连接能力模块

- 目标：拆分 `backend_pump_tests/connect.rs`，把连接命令终态跳过、host mismatch 和 stale connect 尾队列裁剪分离。
- 已完成：`backend_pump_tests/connect.rs` 现在只保留连接队列泵测试模块入口。
- 已完成：新增 `backend_pump_tests/connect_terminal.rs`，迁移终态 connect 命令跳过测试。
- 已完成：新增 `backend_pump_tests/connect_mismatch.rs`，迁移 connect target host 与 tab host 不匹配时失败测试。
- 已完成：新增 `backend_pump_tests/connect_stale.rs`，迁移 stale connect 后裁剪同 session 尾命令、保留其他 session 尾命令测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::backend_pump_tests::connect -- --nocapture` 通过，`4 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余中型测试模块，优先 `launch_tests/sftp/transfer_upload.rs` 或 `launch_tests/sftp/browser.rs`。

## 本轮测试拆分：SFTP 上传传输能力模块

- 目标：拆分 `launch_tests/sftp/transfer_upload.rs`，把上传成功、browser owner 重分配和输入/连接校验场景分离。
- 已完成：`launch_tests/sftp/transfer_upload.rs` 现在只保留上传传输测试模块入口。
- 已完成：新增 `launch_tests/sftp/transfer_upload_success.rs`，迁移上传消息创建 transfer 并队列化 SFTP upload request 的成功路径测试。
- 已完成：新增 `launch_tests/sftp/transfer_upload_owner.rs`，迁移断连 owner 重分配后设置 loading，以及无效输入不重分配 owner 的测试。
- 已完成：新增 `launch_tests/sftp/transfer_upload_validation.rs`，迁移断连 session 拒绝上传和 remote name 包含路径分隔符拒绝上传的测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::launch_tests::sftp::transfer::upload -- --nocapture` 通过，`5 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余中型测试模块，优先 `launch_tests/sftp/browser.rs`、`launch_tests/sftp/cancel_transfer.rs` 或 `launch_tests/sftp/transfer_download.rs`。

## 本轮测试拆分：SFTP Browser 能力模块

- 目标：拆分 `launch_tests/sftp/browser.rs`，把 SFTP browser 的打开、刷新和导航场景分离。
- 已完成：`launch_tests/sftp/browser.rs` 现在只保留 browser 测试模块入口。
- 已完成：新增 `launch_tests/sftp/browser_open.rs`，迁移创建 browser 并队列化 list dir、空初始目录默认根目录和缺失 host 拒绝打开测试。
- 已完成：新增 `launch_tests/sftp/browser_refresh.rs`，迁移刷新当前目录、断连 browser 拒绝刷新和 owner 重分配后刷新测试。
- 已完成：新增 `launch_tests/sftp/browser_navigate.rs`，迁移导航目标目录并队列化 list dir 测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::launch_tests::sftp::browser -- --nocapture` 通过，`7 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余中型测试模块，优先 `launch_tests/sftp/cancel_transfer.rs`、`launch_tests/sftp/transfer_download.rs` 或 `launch_tests/sftp/file_actions.rs`。

## 本轮测试拆分：SFTP 取消传输能力模块

- 目标：拆分 `launch_tests/sftp/cancel_transfer.rs`，把取消成功路径和拒绝取消路径分离。
- 已完成：`launch_tests/sftp/cancel_transfer.rs` 现在只保留取消传输测试模块入口。
- 已完成：新增 `launch_tests/sftp/cancel_transfer_success.rs`，迁移 queued transfer 取消并移除 backend command，以及同 id 其他 session command 保留测试。
- 已完成：新增 `launch_tests/sftp/cancel_transfer_reject.rs`，迁移同 id transfer 不唯一拒绝取消和 backend queue 已移除时拒绝取消测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::launch_tests::sftp::cancel::transfer -- --nocapture` 通过，`4 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余中型测试模块，优先 `launch_tests/sftp/transfer_download.rs`、`launch_tests/sftp/file_actions.rs` 或 `launch_tests/tunnel_stop.rs`。

## 本轮测试拆分：SFTP 下载传输能力模块

- 目标：拆分 `launch_tests/sftp/transfer_download.rs`，和上传传输测试保持同样的薄入口与能力子模块结构。
- 已完成：`launch_tests/sftp/transfer_download.rs` 现在只保留下载传输测试模块入口。
- 已完成：新增 `launch_tests/sftp/transfer_download_success.rs`，迁移下载消息创建 transfer 并队列化 SFTP download request 的成功路径测试。
- 已完成：新增 `launch_tests/sftp/transfer_download_owner.rs`，迁移下载不改变 browser loading，以及无效 remote path 不重分配 owner 的测试。
- 已完成：新增 `launch_tests/sftp/transfer_download_validation.rs`，迁移空 remote path 和根目录 remote path 拒绝下载测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::launch_tests::sftp::transfer::download -- --nocapture` 通过，`4 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余中型测试模块，优先 `launch_tests/sftp/file_actions.rs`、`launch_tests/tunnel_stop.rs` 或 `backend_pump_tests/terminal.rs`。

## 本轮测试拆分：SFTP 文件操作能力模块

- 目标：拆分 `launch_tests/sftp/file_actions.rs`，把创建/删除成功路径、删除路径校验和创建目录名校验分离。
- 已完成：`launch_tests/sftp/file_actions.rs` 现在只保留文件操作测试模块入口。
- 已完成：新增 `launch_tests/sftp/file_actions_success.rs`，迁移 create dir 与 remove file 队列化对应 SFTP path request 的成功路径测试。
- 已完成：新增 `launch_tests/sftp/file_actions_remove_validation.rs`，迁移删除文件空路径和根目录路径拒绝测试。
- 已完成：新增 `launch_tests/sftp/file_actions_create_validation.rs`，迁移创建目录名包含路径分隔符和父目录别名拒绝测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::launch_tests::sftp::file_actions -- --nocapture` 通过，`4 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：SFTP 测试入口已基本瘦身，继续拆剩余 AppState 大模块，优先 `launch_tests/remote_command_backend_events.rs`、`launch_tests/tunnel_stop.rs` 或 `backend_pump_tests/terminal.rs`。

## 本轮测试拆分：远程命令 Backend Events 能力模块

- 目标：拆分 `launch_tests/remote_command_backend_events.rs`，把远程命令历史对 exit、failure 和 disconnect 后端事件的响应分离。
- 已完成：`launch_tests/remote_command_backend_events.rs` 现在只保留 backend events 测试模块入口。
- 已完成：新增 `launch_tests/remote_command_backend_events_exit.rs`，迁移 exit code 更新最新历史、按 session history id 更新历史和 legacy fallback 测试。
- 已完成：新增 `launch_tests/remote_command_backend_events_failure.rs`，迁移 failure 结束历史和 failure 后 late exit 不再覆盖历史测试。
- 已完成：新增 `launch_tests/remote_command_backend_events_disconnect.rs`，迁移 disconnect 结束历史但不写 exit code 测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::launch_tests::remote_command::backend_events -- --nocapture` 通过，`6 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余 AppState 大模块，优先 `launch_tests/tunnel_stop.rs`、`backend_pump_tests/terminal.rs` 或 `snippets_tests/run.rs`。

## 本轮测试拆分：Tunnel Stop 能力模块

- 目标：拆分 `launch_tests/tunnel_stop.rs`，把停止隧道的成功路径、运行态缺失/不匹配和输入校验分离。
- 已完成：`launch_tests/tunnel_stop.rs` 现在只保留 stop tunnel 测试模块入口。
- 已完成：新增 `launch_tests/tunnel_stop_success.rs`，迁移标记 runtime stopping 并队列化 StopTunnel，以及 rule name 归一化测试。
- 已完成：新增 `launch_tests/tunnel_stop_runtime.rs`，迁移缺失 runtime、session 不匹配和缺失当前会话 runtime 的拒绝测试。
- 已完成：新增 `launch_tests/tunnel_stop_validation.rs`，迁移空 rule name 和重复 stop 拒绝测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::launch_tests::tunnel::stop -- --nocapture` 通过，`7 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余 AppState 大模块，优先 `backend_pump_tests/terminal.rs`、`snippets_tests/run.rs` 或 `ui_drafts/terminal_input.rs`。

## 本轮测试拆分：Backend Pump Terminal 能力模块

- 目标：拆分 `backend_pump_tests/terminal.rs`，把终态 Shell 命令跳过和终态 Remote Command 历史收尾分离。
- 已完成：`backend_pump_tests/terminal.rs` 现在只保留 terminal 队列泵测试模块入口。
- 已完成：新增 `backend_pump_tests/terminal_shell.rs`，迁移 terminal open shell、shell drain 和 shell input command 跳过测试。
- 已完成：新增 `backend_pump_tests/terminal_remote_command.rs`，迁移 terminal remote command 跳过和历史结束测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::backend_pump_tests::terminal -- --nocapture` 通过，`5 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余 AppState 大模块，优先 `snippets_tests/run.rs`、`backend_pump_tests/sftp_error.rs` 或 `backend_pump_tests/sftp_transfer.rs`。

## 本轮测试拆分：Snippets Run 能力模块

- 目标：拆分 `snippets_tests/run.rs`，把快捷命令运行成功、运行校验和参数化变量场景分离。
- 已完成：`snippets_tests/run.rs` 现在只保留 run snippets 测试模块入口。
- 已完成：新增 `snippets_tests/run_success.rs`，迁移渲染并执行远程命令的成功路径测试。
- 已完成：新增 `snippets_tests/run_validation.rs`，迁移跨 host snippet 拒绝和空渲染结果无副作用测试。
- 已完成：新增 `snippets_tests/run_arguments.rs`，迁移缺失必填变量提示和更新变量后允许运行测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::snippets_tests::run -- --nocapture` 通过，`5 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余 AppState 大模块，优先 `backend_pump_tests/sftp_error.rs`、`backend_pump_tests/sftp_transfer.rs` 或生产侧 `ui_drafts/terminal_input.rs`。

## 本轮测试拆分：Backend Pump SFTP Error 能力模块

- 目标：拆分 `backend_pump_tests/sftp_error.rs`，把 SFTP 操作错误后的 browser 状态、transfer 裁剪和 write command 裁剪分离。
- 已完成：`backend_pump_tests/sftp_error.rs` 现在只保留 SFTP error 队列泵测试模块入口。
- 已完成：新增 `backend_pump_tests/sftp_error_browser.rs`，迁移 SFTP 操作错误后 session 保持 connected、browser loading 清理和 last_error 记录测试。
- 已完成：新增 `backend_pump_tests/sftp_error_transfer.rs`，迁移 SFTP error 后 pending transfer 标记失败并保留后续 list dir 测试。
- 已完成：新增 `backend_pump_tests/sftp_error_write.rs`，迁移 SFTP error 后 pending write command 裁剪并保留后续 list dir 测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::backend_pump_tests::sftp::error -- --nocapture` 通过，`3 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余 AppState 大模块，优先 `backend_pump_tests/sftp_transfer.rs`、`visual_settings_tests.rs` 或生产侧 `ui_drafts/terminal_input.rs`。

## 本轮测试拆分：Backend Pump SFTP Transfer 能力模块

- 目标：拆分 `backend_pump_tests/sftp_transfer.rs`，把 SFTP transfer 在终态 session、terminal pruning 和 executor error 下的失败归约分离。
- 已完成：`backend_pump_tests/sftp_transfer.rs` 现在只保留 SFTP transfer 队列泵测试模块入口。
- 已完成：新增 `backend_pump_tests/sftp_transfer_terminal.rs`，迁移终态 SFTP session 下 upload/download transfer 标记失败测试。
- 已完成：新增 `backend_pump_tests/sftp_transfer_pruned.rs`，迁移 terminal error 裁剪后 transfer 标记失败测试。
- 已完成：新增 `backend_pump_tests/sftp_transfer_executor_error.rs`，迁移 executor 不支持 SFTP transfer 时失败并记录 browser last_error 测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::backend_pump_tests::sftp::transfer -- --nocapture` 通过，`3 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余 AppState 大模块，优先 `visual_settings_tests.rs`、`tests/close_tabs/sftp_owner.rs` 或生产侧 `ui_drafts/terminal_input.rs`。

## 本轮生产拆分：Terminal Input 草稿与发送逻辑

- 目标：拆分生产侧 `ui_drafts/terminal_input.rs`，把终端输入草稿编辑和发送到 Shell 后端的行为分离。
- 已完成：`ui_drafts/terminal_input.rs` 现在只保留 update/append/backspace 草稿编辑逻辑和可见字符过滤 helper。
- 已完成：新增 `ui_drafts/terminal_input_send.rs`，承载 `send_terminal_input`、本地终端 tab 确保、Shell 会话校验、命令历史写入、backend command 队列化和本地 echo 行为。
- 已完成：`ui_drafts.rs` 增加 `terminal_input_send` 子模块声明，调用方接口保持不变。
- 验证记录：
  - `cargo fmt` 已执行并修正格式
  - `cargo fmt --check` 通过
  - `cargo test --lib terminal_input -- --nocapture` 通过，`11 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM，中文注释和错误文案抽样正常
- 下一步：继续拆剩余 AppState 大模块，优先 `visual_settings_tests.rs`、`tests/close_tabs/sftp_owner.rs` 或 `visual_settings/host.rs`。

## 本轮测试拆分：Visual Settings 能力模块

- 目标：拆分 `visual_settings_tests.rs`，把全局视觉配置、Host 级覆盖和校验错误场景分离。
- 已完成：`visual_settings_tests.rs` 保留共享 `sample_host` helper，并只聚合三个能力子模块。
- 已完成：新增 `visual_settings_tests_global.rs`，迁移全局 visual settings draft 更新并应用到 config/storage 的测试。
- 已完成：新增 `visual_settings_tests_host.rs`，迁移 host visual settings 应用与清除 override 的测试。
- 已完成：新增 `visual_settings_tests_validation.rs`，迁移无效全局配置不改变 config 和缺失 host 报错测试。
- 验证记录：
  - `cargo fmt --check` 通过
  - `cargo test --lib model::app_state::visual_settings_tests -- --nocapture` 通过，`4 passed`
  - `cargo test` 通过，`246 passed`
  - `git diff --check` 通过，只有 Windows CRLF 提示
  - BOM/中文抽样检查通过，相关 Rust 文件均无 BOM 且 `session.md` 中文抽样正常
- 下一步：继续拆剩余 AppState 大模块，优先 `tests/close_tabs/sftp_owner.rs`、`visual_settings/host.rs` 或 `dispatch.rs`。
