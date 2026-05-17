# smagicalssh 会话恢复基线

## 项目目标

- 项目：`smagicalssh`
- 目标：Rust 跨平台桌面 SSH 工作台，参考 XTerminal / Termora / Termius / Tabby。
- 当前优先级：先完成核心，UI 后续由用户指导调整。
- 核心范围：SSH shell/PTY、远程命令、SFTP、端口转发/隧道、命令历史、主机/分组/标签页/最近连接、Known Hosts、凭据安全存储、Snippets、工作区恢复。
- 工程要求：UI 和核心分离，模块化、功能化、小文件、单一职责，中文注释，完整测试。

## 技术栈

- GUI：`slint`
- SSH：`russh`
- SFTP：`russh-sftp`
- 本地 PTY：`portable-pty`
- 终端状态：`alacritty_terminal`
- 存储：`redb`
- 凭据：`keyring`
- 异步运行时：`tokio`
- 序列化：`serde` / `toml`

## 当前进度

- 已从 iced 迁移到 Slint。
- 已实现真实 SSH 后端：连接、认证、shell、远程命令、SFTP、隧道。
- 已实现本地终端、命令历史、远程命令退出码和耗时回写。
- 已实现 SFTP 浏览、上传、下载、删除、创建目录、书签、传输状态、取消队列任务。
- 已实现隧道 Local / Remote / Dynamic SOCKS5 的启动、停止、状态管理。
- 已实现 Known Hosts、凭据引用、安全解析、redb 快照持久化。
- 当前核心接近最小可用版，但仍需要真实运行预览和烟测确认。

## 最近提交

- `331488d 同步隧道事件标签状态`
- `661339a 补齐证书私钥口令认证`
- `cdde843 加载存储时重建不变量`
- `2f2013b 限制命令历史容量`
- `2201865 规范化隧道规则`

## 当前仓库状态

- 分支：`dev`
- 工作区：创建本文件前为干净状态
- 相对远端：领先 `origin/dev` 76 个提交
- 最近全量验证：`cargo test` 为 `337 passed, 2 ignored`

## 提交要求

- commit 首行使用中文总结。
- commit body 使用中英双文。
- commit body 必须记录测试结果。
- 示例结构：
  - `中文说明：...`
  - `English: ...`
  - `测试：cargo fmt --check；cargo check；cargo test（xxx passed，x ignored）。`

## 验证要求

- 常规每轮至少执行：
  - `cargo fmt --check`
  - `cargo check`
  - `cargo test`
  - `git diff --check`
  - BOM 检查
  - 中文抽样检查
- 重要核心改动按三轮验证执行。

## 开发偏好

- 回复始终使用简体中文。
- 当前阶段不要主动打磨 UI。
- 不做大范围重构，优先小步核心闭环。
- 修改含中文文件前确认 BOM；无 BOM 保持 UTF-8 无 BOM。
- 优先 `apply_patch` 局部修改，不整文件覆盖。
- 不删除、不回滚用户或已有改动，除非用户明确要求。

## 下一步建议

1. 继续核心优先，检查真实运行时资源生命周期和错误状态收敛。
2. 做一次本地应用预览和核心烟测：本地终端、SSH shell、远程命令、SFTP、隧道各跑一条主流程。
3. 烟测通过后再进入 UI 调整阶段，由用户逐步指导布局和样式。
