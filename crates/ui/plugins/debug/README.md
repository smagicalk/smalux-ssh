# plugin-debug: 开发者诊断台与 Tracing 日志控制中心插件

> **模块定位**：Smalux 系统级现场诊断与性能压测中心（活动栏第八位 / F12 全局唤出）。为开发者与高级运维工程师提供 Tracing 环形日志流式查阅、渲染帧率探针、状态快照监控，以及针对主机、凭据和代码片段的**万级海量 Mock 数据批量生成与压力测试工具箱 (`DebugModal`)**。

---

## 一、 模块职责与着力点 (Focus & Scope)

1. **全系统 Tracing 内存环形日志控制台 (`debug-logs-tab.slint`)**：
   - 直通 Rust `tracing_subscriber` 内存环形缓冲区 (`CircularBuffer`)；
   - 支持按日志级别 (`ERROR`, `WARN`, `INFO`, `DEBUG`, `TRACE`) 进行多重快速过滤；
   - 支持一键清空日志、复制选中文本与实时滚屏追踪。
2. **海量数据压力测试与模拟发生器 (`debug-hosts-tab.slint` 等)**：
   - 支持一键批量生成 100 / 500 / 1,000 / 5,000 个带嵌套层级的测试主机与树形节点；
   - 用于严苛验证主机树虚拟视口截断算法与搜索过滤的吞吐极限。
3. **渲染引擎与显存诊断 (`debug-rendering-tab.slint`)**：
   - 实时监控 GPU 纹理光栅化帧耗时 (FPS/ms)；
   - 监测壁纸显存缓存槽位（2 槽回收）与终端 Ping-Pong 双缓冲分配抖动。

---

## 二、 核心 Slint 组件与调用方法 (Slint Components & Usage)

### `DebugModal` (全局悬浮开发者控制台)
```slint
import { DebugModal } from "@plugin-debug/debug_plugin.slint";

// 可在任意视图层级直接声明 (由 DebugBridge.is-open 全局控制显示隐藏)
DebugModal { }
```

全局唤出方式：
- 按键盘 **`F12`**；
- 按快捷键 **`Ctrl + Shift + D`**；
- 点击左侧活动栏底部的调试图标；
- 点击底部状态栏右侧的调试控制台徽标。

---

## 三、 核心 Rust 驱动与 API 接口 (Rust Handlers & APIs)

在 `crates/smagical-ui/src/handlers/debug_handlers.rs` 与 `debug/` 中集中管理：

### 1. 批量模拟注入主机资产
```rust
pub async fn mock_batch_hosts(
    count: usize,
    storage: &Arc<dyn AppStorage>,
) -> Result<()>
```
- **参数**：批量生成数量（如 500）；
- **说明**：自动构建包含生产区、灰度区与边缘节点的网状拓扑并写入内存/物理库，立即广播 `HostCreated` 事件刷新 UI。

### 2. 刷新内存日志切片至 UI
```rust
pub fn poll_latest_logs(
    max_entries: usize,
) -> Vec<LogEntryData>
```
- **说明**：从无锁环形缓冲区中提取最新产生的高优先级日志行，转换为 UI 强类型切片注入 `DebugBridge`。

---

## 四、 核心数据模型 (Data Models)

```rust
// 对应 Slint 中的 LogEntryData
pub struct LogEntryData {
    pub level: SharedString, // "ERROR" | "WARN" | "INFO" | "DEBUG" | "TRACE"
    pub target: SharedString, // "smagical_ssh::session"
    pub message: SharedString,
    pub timestamp: SharedString, // "14:20:05.123"
}
```
