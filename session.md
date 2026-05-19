# smagicalssh 会话恢复基线

## 项目目标

- 项目：`smagicalssh`
- 目标：Rust 跨平台桌面 SSH 工作台，参考 XTerminal / Termora / Termius / Tabby。
- 当前优先级：继续完成核心，UI 先不扩展。
- 核心范围：SSH shell/PTY、远程命令、SFTP、端口转发/隧道、命令历史、主机/分组/标签页/最近连接、Known Hosts、凭据安全存储、Snippets、工作区恢复。
- 工程要求：模块化、功能化、单一职责，小文件，中文注释，完整测试。

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

## 最近提交

- 本轮待提交：锁住关闭隧道断开清理
- `e636b5c 裁剪失效连接尾部命令`
- `e889118 跳过终态连接命令`
- `85e6ffd 跳过终态 shell 启动命令`
- `dac789b 跳过终态隧道停止命令`
- `96530e9 跳过终态隧道启动命令`

## 当前仓库状态

- 分支：`dev`
- 远端进度：本轮提交后预计领先 `origin/dev` 144 个提交
- 最近验证：
  - `cargo fmt --check` 通过
  - `cargo check` 通过
  - `cargo test close_connected_pending_tunnel_tab_queues_disconnect_after_cancelling_launch -- --nocapture` 通过，`1 passed`
  - `cargo test backend_queue_pump_executes_disconnect_for_closed_tabs -- --nocapture` 通过，`1 passed`
  - `cargo test model::app_state::tests` 通过，`34 passed`
  - `cargo test model::app_state::backend_pump_tests` 通过，`30 passed`
  - `cargo test session::tabs::tests` 通过，`14 passed`
  - `cargo test model::app_state::launch_tests::tunnel` 通过，`12 passed`
  - `cargo test` 通过，`463 passed, 2 ignored`
  - `git diff --check` 通过，仅 Windows CRLF 提示
  - BOM 与中文抽样检查通过

## 提交要求

- commit 首行使用中文总结。
- commit body 使用中英双文。
- commit body 必须记录测试结果。

## 下一步建议

1. 继续检查真实运行时资源清理和错误状态收敛。
2. 做本地应用预览和核心烟测：本地终端、SSH shell、远程命令、SFTP、隧道。
3. 烟测通过后再进入 UI 调整阶段。
