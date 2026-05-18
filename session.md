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
- 最新核心新增：后端隧道句柄记录所属 `session_id`，停止隧道时必须同时匹配会话和规则名，避免迟到停止命令影响同名新隧道。

## 最近提交

- `b7e70ad 断开时尽力释放后端资源`
- `ffacbca 清理启动隧道前的后端子资源`
- `3fadd34 清理重连前的后端旧资源`
- `17bca9b 清理失败的远程 shell 输入句柄`
- `9ff13f6 清理失败的 SFTP 后端会话`

## 当前仓库状态

- 分支：`dev`
- 远端进度：本轮提交后预计领先 `origin/dev` 97 个提交
- 最近验证：
  - `cargo fmt --check` 通过
  - `cargo check` 通过
  - `cargo test` 通过，`386 passed, 2 ignored`

## 提交要求

- commit 首行使用中文总结。
- commit body 使用中英双文。
- commit body 必须记录测试结果。

## 下一步建议

1. 继续检查真实运行时资源清理和错误状态收敛。
2. 做本地应用预览和核心烟测：本地终端、SSH shell、远程命令、SFTP、隧道。
3. 烟测通过后再进入 UI 调整阶段。
