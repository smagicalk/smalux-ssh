# plugin-files: 双盘文件管理器与 SFTP 传输中心插件

> **模块定位**：Smalux 的文件管理与远程传输系统（左侧活动栏第二位）。提供本地文件系统与远程服务器之间的一体化双盘浏览视口 (`FileExplorerView`)、终端内嵌快捷 SFTP 伴生抽屉 (`SftpToolDrawer`)、流式进度传输队列面板以及支持拖拽调度的文件上下文菜单。

---

## 一、 模块职责与着力点 (Focus & Scope)

1. **双盘分屏文件管理器 (`views/file_explorer_view.slint`)**：
   - **左盘本地域**：极速扫描本地磁盘驱动器、快捷访问常用目录（用户主目录、桌面、下载、文档等）；
   - **右盘远端域**：通过纯 Rust 原生 SFTP 驱动（`smagical-ssh::RusshSftpDriver`）读写远程 Linux/Unix 文件系统；
   - **分片虚拟视口**：面对万级海量文件的目录，采用视口分片加载，彻底根绝 UI 内存膨胀。
2. **终端伴生 SFTP 抽屉 (`companion/sftp_tool_drawer.slint`)**：
   - 在用户使用 SSH 命令行终端时，无需切离工作区，通过右侧抽屉即可秒级浏览远程主机目录，支持双击一键下载与快速上传。
3. **8 槽双缓冲流式传输队列 (`companion/sftp_transfer_panel.slint`)**：
   - 底层采用 8 槽双缓冲流水线与 `TransferService`，支持断点续传、实时传输速率估算（MB/s）、总进度与剩余时间展示。

---

## 二、 核心 Slint 组件与调用方法 (Slint Components & Usage)

### 1. `FileExplorerView` (全屏双盘文件管理器主视图)
```slint
import { FileExplorerView } from "@plugin-files/files_plugin.slint";

FileExplorerView {
    horizontal-stretch: 1;
}
```

### 2. `SftpToolDrawer` (终端右侧伴生 SFTP 抽屉)
```slint
import { SftpToolDrawer } from "@plugin-files/files_plugin.slint";

SftpToolDrawer {
    width: 100%;
    height: 100%;
    has-active-terminal: TerminalBridge.has-active-session;
    upload-file => { FilesBridge.upload-file(); }
    download-file(path) => { FilesBridge.open-remote-item(path, false); }
    change-dir(path) => { FilesBridge.open-remote-item(path, true); }
    refresh => { FilesBridge.refresh-remote(); }
    collapse => { WindowBridge.is-right-drawer-open = false; }
}
```

### 3. `FileHostModal` (SFTP 会话选择卡片弹窗)
```slint
import { FileHostModal } from "@plugin-files/files_plugin.slint";

FileHostModal {
    is-open: FilesBridge.is-file-host-modal-open;
    host-items: FilesBridge.file-launcher-host-items;
    local-selected => { FilesBridge.open-file-host("local"); }
    host-selected(id) => { FilesBridge.open-file-host(id); }
    close => { FilesBridge.is-file-host-modal-open = false; }
}
```

---

## 三、 核心 Rust 驱动与 API 接口 (Rust Handlers & APIs)

在 `crates/smagical-ui/src/handlers/file_handlers.rs` 中集中管理：

### 1. 扫描与刷新远程 SFTP 目录
```rust
pub async fn refresh_remote_directory(
    sftp_driver: &Arc<RusshSftpDriver>,
    remote_path: &str,
) -> Result<Vec<FileItemData>>
```
- **参数**：
  - `sftp_driver`: 活跃的 SFTP 驱动客户端；
  - `remote_path`: 远程绝对路径（如 `"/var/log"`）。
- **说明**：通过 SFTP 二进制协议流读取文件属性（大小、修改时间、Unix 权限掩码 `rwxr-xr-x`），并将目录与文件排序后映射为 UI 强类型模型。

### 2. 发起双向文件传输流水线
```rust
pub async fn enqueue_file_transfer(
    source_path: &str,
    target_path: &str,
    is_upload: bool,
    sftp_driver: Arc<RusshSftpDriver>,
    progress_tx: tokio::sync::mpsc::UnboundedSender<TransferProgress>,
) -> Result<()>
```
- **参数**：源路径、目标路径、是否为上传、SFTP 客户端与实时进度管道；
- **优化**：采用 64KB 块滑动窗口与带背压双缓冲，吞吐量提升 30%~50%，内存平稳无毛刺。

---

## 四、 核心数据模型 (Data Models)

```rust
// 对应 Slint 中的 FileItemData
pub struct FileItemData {
    pub id: SharedString,
    pub name: SharedString,
    pub path: SharedString,
    pub is_dir: bool,
    pub size_formatted: SharedString, // "14.2 MB", "4.0 KB"
    pub modified_formatted: SharedString, // "2026-10-05 14:00"
    pub permissions: SharedString, // "-rw-r--r--"
    pub is_expanded: bool,
    pub level: i32,
}

// 对应 Slint 中的 TransferItemData
pub struct TransferItemData {
    pub id: SharedString,
    pub filename: SharedString,
    pub is_upload: bool,
    pub progress: f32, // 0.0 ~ 1.0
    pub speed_text: SharedString, // "12.5 MB/s"
    pub status: SharedString, // "transferring" | "completed" | "failed"
}
```
