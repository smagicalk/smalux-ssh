//! 双盘文件浏览器与 SFTP 传输领域模型。
//!
//! 提供本地与远程文件系统树节点模型、会话 Tab 状态与后台异步传输任务调度。

use serde::{Deserialize, Serialize};
use std::path::Path;

/// 格式化文件大小为人类可读格式 (B, KB, MB, GB, TB)
pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes == 0 {
        "0 B".to_string()
    } else if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes < TB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    }
}

/// 格式化时间戳为可读日期字符串 (YYYY-MM-DD HH:MM)
pub fn format_file_time(timestamp: u64) -> String {
    if timestamp == 0 {
        return "-".to_string();
    }
    let total_secs = timestamp + 8 * 3600; // CST (UTC+8)
    let secs_of_day = total_secs % 86400;
    let hours = secs_of_day / 3600;
    let minutes = (secs_of_day % 3600) / 60;

    let mut days = (total_secs / 86400) as i64;
    days += 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02} {:02}:{:02}", y, m, d, hours, minutes)
}

/// 文件或目录条目模型 (用于 Slint 树形渲染与文件操作)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileItemData {
    /// 唯一标识 (通常为规范化绝对路径)
    pub id: String,
    /// 显示文件名 (如 "Cargo.toml", "src")
    pub name: String,
    /// 完整绝对路径 (如 "/home/user/project/Cargo.toml")
    pub path: String,
    /// 是否为目录文件夹
    pub is_dir: bool,
    /// 字节大小
    pub size: u64,
    /// 格式化大小 (如 "1.8 KB", "4.2 MB", 目录为空或 "-")
    pub size_formatted: String,
    /// 修改时间 Unix 时间戳 (秒)
    pub modified_at: u64,
    /// 格式化修改时间 (如 "2026-08-31 18:30")
    pub modified_formatted: String,
    /// Linux/Unix 权限字符串 (如 "drwxr-xr-x", "0755")
    pub permissions: String,
    /// 所有者用户 (如 "root", "www-data")
    pub owner: String,
    /// 所属组 (如 "root", "staff")
    pub group: String,
    /// 是否为符号链接 / 快捷方式
    pub is_symlink: bool,
    /// 是否为隐藏文件 (以 . 开头)
    pub is_hidden: bool,
    /// 树形展开状态 (仅目录有效)
    pub is_expanded: bool,
    /// 树形缩进层级 (从 0 开始)
    pub level: i32,
    /// 包含的子项数 (目录特有)
    pub item_count: i32,
}

impl FileItemData {
    /// 构造新文件项
    pub fn new_file(
        name: impl Into<String>,
        path: impl Into<String>,
        size: u64,
        modified_at: u64,
        permissions: impl Into<String>,
    ) -> Self {
        let n = name.into();
        let is_hidden = n.starts_with('.');
        Self {
            id: path.into(),
            name: n,
            path: "".into(),
            is_dir: false,
            size,
            size_formatted: format_file_size(size),
            modified_at,
            modified_formatted: format_file_time(modified_at),
            permissions: permissions.into(),
            owner: "user".into(),
            group: "user".into(),
            is_symlink: false,
            is_hidden,
            is_expanded: false,
            level: 0,
            item_count: 0,
        }
    }

    /// 构造新目录项
    pub fn new_dir(
        name: impl Into<String>,
        path: impl Into<String>,
        modified_at: u64,
        permissions: impl Into<String>,
    ) -> Self {
        let n = name.into();
        let is_hidden = n.starts_with('.');
        let p = path.into();
        Self {
            id: p.clone(),
            name: n,
            path: p,
            is_dir: true,
            size: 0,
            size_formatted: "-".into(),
            modified_at,
            modified_formatted: format_file_time(modified_at),
            permissions: permissions.into(),
            owner: "user".into(),
            group: "user".into(),
            is_symlink: false,
            is_hidden,
            is_expanded: false,
            level: 0,
            item_count: 0,
        }
    }
}

/// 本地文件浏览器会话 Tab 模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalFileTabSession {
    /// 会话 Tab 唯一 ID (如 "ltab-1")
    pub tab_id: String,
    /// 标签展示标题 (如 "本地 (主目录)")
    pub title: String,
    /// 本地当前工作目录路径 (如 "C:\Users\dev" 或 "/home/dev")
    pub current_path: String,
    /// 路径历史记录栈 (用于 <- 后退 和 -> 前进)
    #[serde(default)]
    pub history: Vec<String>,
    /// 当前历史栈索引指针
    #[serde(default)]
    pub history_index: usize,
}

impl LocalFileTabSession {
    /// 构造新本地文件会话 Tab
    pub fn new(tab_id: impl Into<String>, title: impl Into<String>, path: impl Into<String>) -> Self {
        let p = path.into();
        Self {
            tab_id: tab_id.into(),
            title: title.into(),
            current_path: p.clone(),
            history: vec![p],
            history_index: 0,
        }
    }

    /// 是否可以后退
    pub fn can_go_back(&self) -> bool {
        self.history_index > 0
    }

    /// 是否可以前进
    pub fn can_go_forward(&self) -> bool {
        self.history_index + 1 < self.history.len()
    }

    /// 记录新访问的路径 (如果中途跳转则丢弃前进栈)
    pub fn push_path(&mut self, new_path: String) {
        if self.current_path == new_path {
            return;
        }
        if self.history_index + 1 < self.history.len() {
            self.history.truncate(self.history_index + 1);
        }
        self.history.push(new_path.clone());
        if self.history.len() > 50 {
            self.history.remove(0);
        }
        self.history_index = self.history.len().saturating_sub(1);
        self.current_path = new_path;
    }

    /// 后退并返回目标路径
    pub fn go_back(&mut self) -> Option<String> {
        if self.can_go_back() {
            self.history_index -= 1;
            let target = self.history[self.history_index].clone();
            self.current_path = target.clone();
            Some(target)
        } else {
            None
        }
    }

    /// 前进并返回目标路径
    pub fn go_forward(&mut self) -> Option<String> {
        if self.can_go_forward() {
            self.history_index += 1;
            let target = self.history[self.history_index].clone();
            self.current_path = target.clone();
            Some(target)
        } else {
            None
        }
    }
}

/// 远程 SFTP 文件传输 Tab 会话模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteFileTabSession {
    /// 会话 Tab 唯一 ID (如 "rtab-1", "rtab-prod")
    pub tab_id: String,
    /// 关联的主机资产 ID (如 "host-prod-01")
    pub host_id: String,
    /// 主机名称 (如 "Prod-Web-01")
    pub host_name: String,
    /// 主机 IP 地址 (如 "192.168.1.100:22")
    pub host_address: String,
    /// 远程当前工作目录 (如 "/var/www/html" 或 "/root")
    pub current_path: String,
    /// 连接状态 ("online" | "warning" | "offline")
    pub status: String,
    /// 路径历史记录栈 (用于 <- 后退 和 -> 前进)
    #[serde(default)]
    pub history: Vec<String>,
    /// 当前历史栈索引指针
    #[serde(default)]
    pub history_index: usize,
}

impl RemoteFileTabSession {
    /// 构造新远程 SFTP 文件会话 Tab
    pub fn new(
        tab_id: impl Into<String>,
        host_id: impl Into<String>,
        host_name: impl Into<String>,
        host_address: impl Into<String>,
        initial_path: impl Into<String>,
    ) -> Self {
        let p = initial_path.into();
        Self {
            tab_id: tab_id.into(),
            host_id: host_id.into(),
            host_name: host_name.into(),
            host_address: host_address.into(),
            current_path: p.clone(),
            status: "online".into(),
            history: vec![p],
            history_index: 0,
        }
    }

    /// 是否可以后退
    pub fn can_go_back(&self) -> bool {
        self.history_index > 0
    }

    /// 是否可以前进
    pub fn can_go_forward(&self) -> bool {
        self.history_index + 1 < self.history.len()
    }

    /// 记录新访问的路径
    pub fn push_path(&mut self, new_path: String) {
        if self.current_path == new_path {
            return;
        }
        if self.history_index + 1 < self.history.len() {
            self.history.truncate(self.history_index + 1);
        }
        self.history.push(new_path.clone());
        if self.history.len() > 50 {
            self.history.remove(0);
        }
        self.history_index = self.history.len().saturating_sub(1);
        self.current_path = new_path;
    }

    /// 后退并返回目标路径
    pub fn go_back(&mut self) -> Option<String> {
        if self.can_go_back() {
            self.history_index -= 1;
            let target = self.history[self.history_index].clone();
            self.current_path = target.clone();
            Some(target)
        } else {
            None
        }
    }

    /// 前进并返回目标路径
    pub fn go_forward(&mut self) -> Option<String> {
        if self.can_go_forward() {
            self.history_index += 1;
            let target = self.history[self.history_index].clone();
            self.current_path = target.clone();
            Some(target)
        } else {
            None
        }
    }
}


/// 单个主机的文件传输 Tab 会话模型 (兼容层)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTabSession {
    /// 会话 Tab 唯一 ID (如 "ftab-1", "ftab-prod")
    pub tab_id: String,
    /// 关联的主机资产 ID (如 "host-prod-01")
    pub host_id: String,
    /// 主机名称 (如 "Prod-Web-01")
    pub host_name: String,
    /// 主机 IP 地址 (如 "192.168.1.100:22")
    pub host_address: String,
    /// 本地当前工作目录 (如 "/Users/dev/workspace")
    pub local_current_path: String,
    /// 远程当前工作目录 (如 "/var/www/html")
    pub remote_current_path: String,
}

impl FileTabSession {
    /// 构造新文件会话 Tab
    pub fn new(
        tab_id: impl Into<String>,
        host_id: impl Into<String>,
        host_name: impl Into<String>,
        host_address: impl Into<String>,
    ) -> Self {
        let home_dir = directories::BaseDirs::new()
            .map(|p| p.home_dir().to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());

        Self {
            tab_id: tab_id.into(),
            host_id: host_id.into(),
            host_name: host_name.into(),
            host_address: host_address.into(),
            local_current_path: home_dir,
            remote_current_path: "/root".into(),
        }
    }
}


/// 传输方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferDirection {
    /// 本地上传至远程 (Local -> Remote)
    Upload,
    /// 远程下载至本地 (Remote -> Local)
    Download,
}

impl std::fmt::Display for TransferDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferDirection::Upload => write!(f, "upload"),
            TransferDirection::Download => write!(f, "download"),
        }
    }
}

/// 传输状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferStatus {
    /// 任务排队中
    Pending,
    /// 正在传输中
    Transferring,
    /// 传输已完成
    Completed,
    /// 传输失败
    Failed,
    /// 任务已暂停
    Paused,
}

impl std::fmt::Display for TransferStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferStatus::Pending => write!(f, "pending"),
            TransferStatus::Transferring => write!(f, "transferring"),
            TransferStatus::Completed => write!(f, "completed"),
            TransferStatus::Failed => write!(f, "failed"),
            TransferStatus::Paused => write!(f, "paused"),
        }
    }
}

/// 异步文件传输任务 (支持单文件与文件夹树状层级展开)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferTask {
    /// 任务唯一 ID
    pub id: String,
    /// 所属父级任务 ID (如果是文件夹下的子文件，则指向父文件夹任务 ID；若为顶层任务则为 None)
    pub parent_id: Option<String>,
    /// 所属文件会话 Tab ID
    pub session_id: String,
    /// 传输文件名或文件夹名
    pub filename: String,
    /// 是否为目录/文件夹任务
    pub is_dir: bool,
    /// 文件夹任务是否在传输列表中展开
    pub is_expanded: bool,
    /// 树层级缩进 (0: 顶层, 1: 文件夹内子项, 2: 二级嵌套...)
    pub level: i32,
    /// 子项数量统计说明 (如 "4 项")
    pub item_count_text: String,
    /// 源文件绝对路径
    pub source_path: String,
    /// 目标文件绝对路径
    pub target_path: String,
    /// 传输方向 (上传/下载)
    pub direction: TransferDirection,
    /// 文件总字节数 (文件夹为所有子文件总字节数)
    pub total_bytes: u64,
    /// 已传输字节数 (文件夹为所有子文件已传输字节数)
    pub transferred_bytes: u64,
    /// 当前传输速率 (字节/秒)
    pub speed_bytes_per_sec: u64,
    /// 任务运行状态
    pub status: TransferStatus,
    /// 失败时的错误信息
    pub error_message: Option<String>,
}



impl TransferTask {
    /// 计算完成百分比 (0.0 ~ 1.0)
    pub fn progress(&self) -> f32 {
        if self.total_bytes == 0 {
            if self.status == TransferStatus::Completed {
                1.0
            } else {
                0.0
            }
        } else {
            (self.transferred_bytes as f32 / self.total_bytes as f32).min(1.0)
        }
    }

    /// 格式化速度文本 (如 "12.4 MB/s")
    pub fn speed_formatted(&self) -> String {
        if self.status != TransferStatus::Transferring {
            return "-".into();
        }
        format!("{}/s", format_file_size(self.speed_bytes_per_sec))
    }
}

/// 扫描指定本地目录下的直接子文件与子目录
pub fn scan_local_directory(dir_path: &Path) -> std::io::Result<Vec<FileItemData>> {
    let mut items = Vec::new();

    if !dir_path.exists() {
        return Ok(items);
    }

    let entries = std::fs::read_dir(dir_path)?;
    for entry in entries.flatten() {
        let path = entry.path();
        let filename = entry.file_name().to_string_lossy().to_string();
        let metadata = entry.metadata().ok();

        let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let size = if is_dir {
            0
        } else {
            metadata.as_ref().map(|m| m.len()).unwrap_or(0)
        };

        let modified_at = metadata
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let permissions = if is_dir {
            "drwxr-xr-x".to_string()
        } else {
            "-rw-r--r--".to_string()
        };

        let path_str = path.to_string_lossy().to_string();
        let is_hidden = filename.starts_with('.');

        items.push(FileItemData {
            id: path_str.clone(),
            name: filename,
            path: path_str,
            is_dir,
            size,
            size_formatted: if is_dir { "-".into() } else { format_file_size(size) },
            modified_at,
            modified_formatted: format_file_time(modified_at),
            permissions,
            owner: "local".into(),
            group: "staff".into(),
            is_symlink: metadata.as_ref().map(|m| m.is_symlink()).unwrap_or(false),
            is_hidden,
            is_expanded: false,
            level: 0,
            item_count: 0,
        });
    }

    // 目录优先，其次按名称字母升序排序
    items.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(items)
}

/// 生成模拟的远程 Linux SFTP 文件系统目录结构 (用于测试与前端交互预览)
pub fn generate_mock_remote_directory(current_path: &str) -> Vec<FileItemData> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1725100000);

    let path_clean = current_path.trim_end_matches('/');

    let entries = match path_clean {
        "" | "/" => vec![
            ("bin", true, 0, "drwxr-xr-x", "root", "root"),
            ("etc", true, 0, "drwxr-xr-x", "root", "root"),
            ("home", true, 0, "drwxr-xr-x", "root", "root"),
            ("opt", true, 0, "drwxr-xr-x", "root", "root"),
            ("root", true, 0, "drwx------", "root", "root"),
            ("usr", true, 0, "drwxr-xr-x", "root", "root"),
            ("var", true, 0, "drwxr-xr-x", "root", "root"),
        ],
        "/var" | "/var/www" => vec![
            ("html", true, 0, "drwxr-xr-x", "www-data", "www-data"),
            ("log", true, 0, "drwxr-xr-x", "root", "root"),
            ("nginx.conf", false, 4096, "-rw-r--r--", "root", "root"),
        ],
        "/root" | "/home/ubuntu" => vec![
            (".bashrc", false, 3771, "-rw-r--r--", "root", "root"),
            (".ssh", true, 0, "drwx------", "root", "root"),
            ("docker-compose.yml", false, 1850, "-rw-r--r--", "root", "root"),
            ("server.py", false, 14200, "-rwxr-xr-x", "root", "root"),
            ("deploy.sh", false, 2400, "-rwxr-xr-x", "root", "root"),
            ("app_logs", true, 0, "drwxr-xr-x", "root", "root"),
            ("database.sql", false, 4820000, "-rw-r--r--", "root", "root"),
        ],
        _ => vec![
            ("src", true, 0, "drwxr-xr-x", "root", "root"),
            ("config.json", false, 1024, "-rw-r--r--", "root", "root"),
            ("README.md", false, 4520, "-rw-r--r--", "root", "root"),
        ],
    };

    let mut items = Vec::new();
    for (name, is_dir, size, perms, owner, group) in entries {
        let full_p = format!("{}/{}", path_clean, name);
        items.push(FileItemData {
            id: full_p.clone(),
            name: name.to_string(),
            path: full_p,
            is_dir,
            size,
            size_formatted: if is_dir { "-".into() } else { format_file_size(size) },
            modified_at: now - 3600,
            modified_formatted: format_file_time(now - 3600),
            permissions: perms.to_string(),
            owner: owner.to_string(),
            group: group.to_string(),
            is_symlink: false,
            is_hidden: name.starts_with('.'),
            is_expanded: false,
            level: 0,
            item_count: 0,
        });
    }

    items.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_file_size(1024 * 1024 * 1024 * 2), "2.00 GB");
    }

    #[test]
    fn test_file_item_creation() {
        let file = FileItemData::new_file("test.rs", "/project/test.rs", 2048, 1725000000, "-rw-r--r--");
        assert!(!file.is_dir);
        assert_eq!(file.size, 2048);
        assert_eq!(file.size_formatted, "2.0 KB");

        let dir = FileItemData::new_dir("src", "/project/src", 1725000000, "drwxr-xr-x");
        assert!(dir.is_dir);
        assert_eq!(dir.size_formatted, "-");
    }

    #[test]
    fn test_mock_remote_directory_generation() {
        let root_files = generate_mock_remote_directory("/");
        assert!(!root_files.is_empty());
        assert!(root_files.iter().any(|f| f.name == "etc" && f.is_dir));
        assert!(root_files.iter().any(|f| f.name == "var" && f.is_dir));

        let var_files = generate_mock_remote_directory("/var");
        assert!(var_files.iter().any(|f| f.name == "nginx.conf" && !f.is_dir));
    }

    #[test]
    fn test_transfer_task_progress() {
        let task = TransferTask {
            id: "t-1".into(),
            parent_id: None,
            session_id: "s-1".into(),
            filename: "video.mp4".into(),
            is_dir: false,
            is_expanded: false,
            level: 0,
            item_count_text: "".into(),
            source_path: "/local/video.mp4".into(),
            target_path: "/remote/video.mp4".into(),
            direction: TransferDirection::Upload,
            total_bytes: 1000,
            transferred_bytes: 500,
            speed_bytes_per_sec: 1024 * 1024,
            status: TransferStatus::Transferring,
            error_message: None,
        };


        assert!((task.progress() - 0.5).abs() < 0.001);
        assert_eq!(task.speed_formatted(), "1.0 MB/s");
    }

    #[test]
    fn test_local_tab_session_history_navigation() {
        let mut session = LocalFileTabSession::new("ltab-1", "Local", "/home/user");
        assert_eq!(session.current_path, "/home/user");
        assert!(!session.can_go_back());
        assert!(!session.can_go_forward());

        // 访问第二个路径
        session.push_path("/home/user/rust".into());
        assert_eq!(session.current_path, "/home/user/rust");
        assert!(session.can_go_back());
        assert!(!session.can_go_forward());

        // 访问第三个路径
        session.push_path("/home/user/rust/src".into());
        assert_eq!(session.current_path, "/home/user/rust/src");
        assert!(session.can_go_back());
        assert!(!session.can_go_forward());

        // 后退到第二个路径
        let back1 = session.go_back();
        assert_eq!(back1, Some("/home/user/rust".to_string()));
        assert_eq!(session.current_path, "/home/user/rust");
        assert!(session.can_go_back());
        assert!(session.can_go_forward());

        // 后退到初始路径
        let back2 = session.go_back();
        assert_eq!(back2, Some("/home/user".to_string()));
        assert_eq!(session.current_path, "/home/user");
        assert!(!session.can_go_back());
        assert!(session.can_go_forward());

        // 前进到第二个路径
        let fwd1 = session.go_forward();
        assert_eq!(fwd1, Some("/home/user/rust".to_string()));
        assert_eq!(session.current_path, "/home/user/rust");
        assert!(session.can_go_back());
        assert!(session.can_go_forward());

        // 在中间跳转新分支，前进记录被丢弃
        session.push_path("/home/user/docs".into());
        assert_eq!(session.current_path, "/home/user/docs");
        assert!(session.can_go_back());
        assert!(!session.can_go_forward());
    }

    #[test]
    fn test_remote_tab_session_history_navigation() {
        let mut session = RemoteFileTabSession::new("rtab-1", "host-1", "Prod", "10.0.0.1:22", "/root");
        assert_eq!(session.current_path, "/root");
        assert!(!session.can_go_back());
        assert!(!session.can_go_forward());

        session.push_path("/var/log".into());
        assert!(session.can_go_back());
        assert_eq!(session.go_back(), Some("/root".to_string()));
        assert_eq!(session.go_forward(), Some("/var/log".to_string()));
    }

    #[test]
    fn test_tab_reordering_within_same_pane() {
        let mut tabs = vec![
            LocalFileTabSession::new("ltab-1", "Tab 1", "D:/dir1"),
            LocalFileTabSession::new("ltab-2", "Tab 2", "D:/dir2"),
            LocalFileTabSession::new("ltab-3", "Tab 3", "D:/dir3"),
        ];

        // 模拟拖拽从 index 0 移到 index 2: [Tab 1, Tab 2, Tab 3] -> [Tab 2, Tab 3, Tab 1]
        let from = 0;
        let to = 2;
        let item = tabs.remove(from);
        tabs.insert(to, item);

        assert_eq!(tabs[0].tab_id, "ltab-2");
        assert_eq!(tabs[1].tab_id, "ltab-3");
        assert_eq!(tabs[2].tab_id, "ltab-1");

        // 模拟拖拽从 index 2 移到 index 1: [Tab 2, Tab 3, Tab 1] -> [Tab 2, Tab 1, Tab 3]
        let from = 2;
        let to = 1;
        let item = tabs.remove(from);
        tabs.insert(to, item);

        assert_eq!(tabs[0].tab_id, "ltab-2");
        assert_eq!(tabs[1].tab_id, "ltab-1");
        assert_eq!(tabs[2].tab_id, "ltab-3");
    }
}



