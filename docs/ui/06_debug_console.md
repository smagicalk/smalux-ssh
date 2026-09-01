# 🛠️ 06. 开发者调试控制台 (Developer Debug Console)

---

## 📌 模块概述

开发者调试控制台用于在桌面开发与联调期间进行大规模压力测试、造数、状态模拟以及与全局 Tracing 实时日志流同步。

---

## 🧩 UI 组件与文件结构

- **弹窗组件**：[`crates/smagical-ui/ui/components/debug-modal.slint`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/ui/components/debug-modal.slint)
- **调试核心服务**：[`crates/smagical-debug/src/lib.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-debug/src/lib.rs)
- **日志模型同步**：[`crates/smagical-ui/src/debug_ui.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/src/debug_ui.rs)
- **路由回调处理器**：[`crates/smagical-ui/src/handlers/debug_handlers.rs`](file:///F:/code/rust/smalux-ssh/crates/smagical-ui/src/handlers/debug_handlers.rs)

---

## 📐 核心数据契约 (Data Contracts)

### 日志条目模型 (`LogEntryData`)
```slint
export struct LogEntryData {
    timestamp: string,  // 时间戳 (HH:MM:SS)
    level: string,      // "INFO" | "WARN" | "ERROR" | "DEBUG" | "TRACE"
    target: string,     // 模块目标 (如 "smagical_ui::files", "audit::file")
    message: string,    // 日志消息文本
}
```

---

## ⚡ 核心调试功能

1. **批量主机造数 (Batch Generator)**：
   - 支持生成 10 ~ 1000+ 台带分组的主机树，测试 UI 虚拟化与横向/纵向滚动性能；
   - 支持追加模式 (`Append`) 与全量覆盖模式 (`Overwrite`)。
2. **批量状态变更 (Batch Status Simulation)**：
   - 支持一键将全量主机置为 `全在线 (Online)`、`全离线 (Offline)`、`全告警 (Warning)` 或随机混合状态；
   - 自动联动 Rust 端 Storage 批量持久化。
3. **实时 Tracing 日志流捕获**：
   - 捕获 `tracing` 产生的全部终端渲染、文件操作、路由跳转与状态变更事件；
   - 支持按日志级别过滤（All / Info / Warn / Error）与一键清空日志。
4. **凭据安全调试工作台 (Credentials Debug Tab)**：
   - 快速生成 5+ 项典型种子凭据（Ed25519 密钥、RSA 密钥、SSH Agent、口令密码）；
   - 一键清空凭据保险库与生成指定规格仿真密钥对。

---

## 🏷️ 全局日志命名空间与分类规范 (Tracing Target Namespaces)

控制台及滚动日志文件按照如下命名空间规范进行结构化分发：

| Target 命名空间 | 分类与职责 | 级别与样例 |
| :--- | :--- | :--- |
| **`smalux::credential`** | **凭据安全管理与生成日志** | `INFO` `[事件总线:凭据保存] ID: [cred-1], 名称: '生产集群 Root 私钥'`<br>`INFO` `[事件总线:密钥生成] 规格: 'Ed25519', 生成公钥指纹: [SHA256:4a8b...]` |
| **`smalux::security`** | **安全审计与高危拦截日志** | `WARN` `[安全审计:机密提取] 凭据 ID: [cred-1], 动作: 复制【PrivateKey】至系统剪贴板 (高危安全操作)`<br>`WARN` `[安全守护] 拦截高危文件删除尝试: /etc/passwd` |
| **`smalux::file`** | **文件与 SFTP 领域操作日志** | `INFO` `[文件管理:会话建立] 会话 Tab: [rtab-1], 目标主机: [h-1]`<br>`INFO` `[文件传输:传输完成] 任务 ID: [task-1], 字节: 128MB` |
| **`smagical_ui::files`** | **UI 交互与视图刷新日志** | `INFO` `进入本地目录: F:/code/rust`<br>`DEBUG` `传输任务折叠/展开: task_id=task-1, is_expanded=true` |
| **`smalux::terminal`** | **终端建立与生命周期日志** | `INFO` `[终端会话:建立] 会话 ID: [term-1], 目标主机: Prod-Web` |
| **`smalux::host`** | **主机资产变动日志** | `INFO` `[主机管理:新增] 主机 ID: [h-100], 名称: 'DB-Master'` |
| **`smalux::history`** | **历史记录管理日志** | `WARN` `[历史记录:清空] 用户清空了非置顶历史会话记录` |
| **`smalux::config`** | **全局配置变动通知** | `INFO` `[事件总线:配置变动] 键: 'ui.theme', 旧值: 'dark', 新值: 'nord'` |
