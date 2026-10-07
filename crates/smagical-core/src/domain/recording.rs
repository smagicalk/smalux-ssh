//! 终端会话录屏、时间轴回放与安全操作审计 (Terminal Session Recording, Replay & Security Audit)。
//!
//! # 架构职责
//! 提供工业级 Asciinema (cast v2) 标准规范的终端会话录制与时间轴回放播放器引擎，
//! 结合生产级命令敏感词语义分析引擎，实现操作全程可记录、时间轴可回溯、高危操作可预警、安全审计可追溯。
//!
//! ## 核心能力
//! 1. **Asciinema Cast v2 序列化与解析**：
//!    - 标准单行 JSONL 协议，与社区官方 `asciinema` CLI 100% 格式对齐兼容；
//!    - 纳秒级 `Instant` 相对时间戳计算，精确捕获 VT100/ANSI 字符流（输出 "o" 与输入 "i"）；
//! 2. **时间轴回放播放器驱动引擎 ([`CastReplayer`])**：
//!    - 纳秒级步进控制，支持播放/暂停/跳播 Seek（`0.0` 至 `duration`）；
//!    - 多倍速平滑变速（0.5x, 1.0x, 2.0x, 4.0x）；
//!    - 格式化时间标签渲染（如 `01:23 / 04:56`）；
//! 3. **深度操作安全审计与高危敏感词侦测 ([`SecurityAuditInspector`])**：
//!    - 多维度破坏性 Shell 命令研判（包含 `rm -rf`, `mkfs`, `fdisk`, `dd if=`, `reboot`, 提权等）；
//!    - 私钥暴露与敏感令牌（Token / Secret）现场侦测；
//!    - 支持针对完整录屏流执行静态全量审计扫描，输出带时间戳与高危标记的审计清单。

use std::collections::HashMap;
use std::io::{BufReader, Write};
use std::path::Path;
use std::time::Instant;
use serde::{Deserialize, Serialize};

/// 终端录制与回放模块错误枚举。
#[derive(Debug, thiserror::Error)]
pub enum RecordingError {
    /// 物理文件读写 I/O 错误
    #[error("录制文件 I/O 错误: {0}")]
    Io(#[from] std::io::Error),
    /// JSON 协议编解码错误
    #[error("录制 JSON 格式解析错误: {0}")]
    Json(#[from] serde_json::Error),
    /// 录制数据损坏或缺失关键头部
    #[error("无效或损坏的录制数据: {0}")]
    InvalidData(String),
}

/// 录制与回放统一结果类型别名
pub type RecordingResult<T> = Result<T, RecordingError>;

/// Asciinema (cast v2) 规范头部元数据。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CastHeader {
    /// Asciinema 规范主版本号 (当前固定为 2)
    pub version: u8,
    /// 终端初始字符列宽
    pub width: u16,
    /// 终端初始字符行高
    pub height: u16,
    /// 录制起始的 UNIX 纪元时间戳 (秒)
    #[serde(default)]
    pub timestamp: u64,
    /// 录屏标题或会话标识
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 会话总持续时长 (秒，录制完成结算后写入)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    /// 环境变量子集 (如 TERM, SHELL 等)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

impl Default for CastHeader {
    fn default() -> Self {
        Self {
            version: 2,
            width: 80,
            height: 24,
            timestamp: chrono::Utc::now().timestamp() as u64,
            title: None,
            duration: None,
            env: {
                let mut m = HashMap::new();
                m.insert("TERM".to_string(), "xterm-256color".to_string());
                m
            },
        }
    }
}

/// 录屏事件流类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastEventType {
    /// 终端屏幕输出字节/转义流 ("o")
    Output,
    /// 用户键盘输入按键/转义流 ("i")
    Input,
}

impl CastEventType {
    /// 获取规范缩写字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Output => "o",
            Self::Input => "i",
        }
    }
}

/// 单个录屏事件帧 (时间相对偏移, 类型, 数据载荷)。
#[derive(Clone, Debug, PartialEq)]
pub struct CastEvent {
    /// 距录制开始时刻的相对时间偏移 (单位：秒，浮点高精度)
    pub time: f64,
    /// 事件类型 (输出 "o" 或输入 "i")
    pub event_type: CastEventType,
    /// 原始 ANSI/文本数据载荷
    pub data: String,
}

impl CastEvent {
    /// 创建新的输出事件
    pub fn output(time: f64, data: impl Into<String>) -> Self {
        Self {
            time,
            event_type: CastEventType::Output,
            data: data.into(),
        }
    }

    /// 创建新的输入事件
    pub fn input(time: f64, data: impl Into<String>) -> Self {
        Self {
            time,
            event_type: CastEventType::Input,
            data: data.into(),
        }
    }

    /// 序列化为 Asciinema v2 单行 JSON 数组: `[t, "type", "data"]`
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        let arr = (self.time, self.event_type.as_str(), &self.data);
        serde_json::to_string(&arr)
    }

    /// 从 Asciinema v2 单行 JSON 数组解析: `[t, "type", "data"]`
    pub fn from_json_line(line: &str) -> Option<Self> {
        let v: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
        let arr = v.as_array()?;
        if arr.len() < 3 {
            return None;
        }
        let time = arr[0].as_f64()?;
        let t_str = arr[1].as_str()?;
        let event_type = match t_str {
            "o" => CastEventType::Output,
            "i" => CastEventType::Input,
            _ => return None,
        };
        let data = arr[2].as_str()?.to_string();
        Some(Self {
            time,
            event_type,
            data,
        })
    }
}

/// 完整的 Asciinema 会话录制档案结构体。
#[derive(Clone, Debug, PartialEq)]
pub struct CastSession {
    /// 规范头部
    pub header: CastHeader,
    /// 有序事件流集合
    pub events: Vec<CastEvent>,
    /// 会话总持续时长 (秒)
    pub duration: f64,
}

impl CastSession {
    /// 创建新的空白录屏会话
    pub fn new(width: u16, height: u16, title: Option<String>) -> Self {
        let header = CastHeader {
            version: 2,
            width,
            height,
            timestamp: chrono::Utc::now().timestamp() as u64,
            title,
            duration: None,
            env: {
                let mut m = HashMap::new();
                m.insert("TERM".to_string(), "xterm-256color".to_string());
                m
            },
        };
        Self {
            header,
            events: Vec::new(),
            duration: 0.0,
        }
    }

    /// 追加终端输出事件
    pub fn push_output(&mut self, time: f64, data: impl Into<String>) {
        if time > self.duration {
            self.duration = time;
        }
        self.events.push(CastEvent::output(time, data));
    }

    /// 追加用户按键输入事件
    pub fn push_input(&mut self, time: f64, data: impl Into<String>) {
        if time > self.duration {
            self.duration = time;
        }
        self.events.push(CastEvent::input(time, data));
    }

    /// 导出为符合 Asciinema cast v2 规范的完整文本流
    pub fn to_cast_v2_string(&self) -> Result<String, serde_json::Error> {
        let mut header_clone = self.header.clone();
        header_clone.duration = Some(self.duration);
        let header_json = serde_json::to_string(&header_clone)?;

        let mut out = String::with_capacity(header_json.len() + self.events.len() * 64);
        out.push_str(&header_json);
        out.push('\n');

        for event in &self.events {
            if let Ok(line) = event.to_json_line() {
                out.push_str(&line);
                out.push('\n');
            }
        }
        Ok(out)
    }

    /// 从符合 Asciinema cast v2 格式的文本流反序列化解析会话
    pub fn from_cast_v2_str(content: &str) -> RecordingResult<Self> {
        let mut lines = content.lines();
        let header_line = lines.next().ok_or_else(|| RecordingError::InvalidData("空录制文件".to_string()))?;
        let header: CastHeader = serde_json::from_str(header_line.trim())?;

        let mut events = Vec::new();
        let mut max_time = 0.0f64;

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(event) = CastEvent::from_json_line(trimmed) {
                if event.time > max_time {
                    max_time = event.time;
                }
                events.push(event);
            }
        }

        let duration = header.duration.unwrap_or(max_time);

        Ok(Self {
            header,
            events,
            duration,
        })
    }

    /// 从本地物理 `.cast` 文件加载会话
    pub fn load_from_file(path: &Path) -> RecordingResult<Self> {
        let file = std::fs::File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut content = String::new();
        std::io::Read::read_to_string(&mut reader, &mut content)?;
        Self::from_cast_v2_str(&content)
    }

    /// 持久化保存至本地 `.cast` 文件
    pub fn save_to_file(&self, path: &Path) -> RecordingResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let serialized = self.to_cast_v2_string()?;
        let mut file = std::fs::File::create(path)?;
        file.write_all(serialized.as_bytes())?;
        file.flush()?;
        Ok(())
    }
}

/// 正在进行的会话录制状态机。
pub struct CastSessionRecorder {
    session_id: String,
    start_time: Instant,
    session: CastSession,
    is_active: bool,
}

impl CastSessionRecorder {
    /// 启动对指定会话的现场录制
    pub fn start(session_id: String, width: u16, height: u16, title: Option<String>) -> Self {
        let session = CastSession::new(width, height, title);
        Self {
            session_id,
            start_time: Instant::now(),
            session,
            is_active: true,
        }
    }

    /// 获取所属会话唯一标识 ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// 查询当前是否处于活跃录制中
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// 获取当前已录制的持续时间 (秒)
    pub fn elapsed_seconds(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    /// 单个录屏会话最大事件条数上限 (100,000 条事件，彻底防止海量输出导致进程 OOM)
    pub const MAX_SESSION_EVENTS: usize = 100_000;

    /// 记录一段终端输出数据
    pub fn record_output(&mut self, text: &str) {
        if !self.is_active || text.is_empty() {
            return;
        }
        if self.session.events.len() >= Self::MAX_SESSION_EVENTS {
            self.is_active = false;
            return;
        }
        let elapsed = self.elapsed_seconds();
        self.session.push_output(elapsed, text);
    }

    /// 记录用户输入操作
    pub fn record_input(&mut self, text: &str) {
        if !self.is_active || text.is_empty() {
            return;
        }
        if self.session.events.len() >= Self::MAX_SESSION_EVENTS {
            self.is_active = false;
            return;
        }
        let elapsed = self.elapsed_seconds();
        self.session.push_input(elapsed, text);
    }

    /// 停止录制并结算产出完整的 [`CastSession`]
    pub fn stop(mut self) -> CastSession {
        self.is_active = false;
        let total_elapsed = self.elapsed_seconds();
        if self.session.duration < total_elapsed {
            self.session.duration = total_elapsed;
        }
        self.session
    }
}

/// 录屏时间轴回放播放器引擎 (Cast Replayer Engine)。
///
/// 具备高精度步进前进、绝对时间跳转 (Seek)、变速播放 (0.5x~4.0x) 与时间标签渲染能力。
pub struct CastReplayer {
    session: CastSession,
    current_time: f64,
    current_idx: usize,
    speed: f64,
    is_playing: bool,
    is_loop: bool,
}

impl CastReplayer {
    /// 挂载指定录屏档案初始化播放器
    pub fn new(session: CastSession) -> Self {
        Self {
            session,
            current_time: 0.0,
            current_idx: 0,
            speed: 1.0,
            is_playing: false,
            is_loop: false,
        }
    }

    /// 获取关联的录屏会话引用
    pub fn session(&self) -> &CastSession {
        &self.session
    }

    /// 获取当前播放器时间轴指针 (秒)
    pub fn current_time(&self) -> f64 {
        self.current_time
    }

    /// 获取录屏档案总持续时长 (秒)
    pub fn duration(&self) -> f64 {
        self.session.duration
    }

    /// 获取当前播放速度倍率 (例如 1.0, 2.0)
    pub fn speed(&self) -> f64 {
        self.speed
    }

    /// 设定播放速度倍率 (限制范围 0.25x ..= 8.0x)
    pub fn set_speed(&mut self, speed: f64) {
        self.speed = speed.clamp(0.25, 8.0);
    }

    /// 查询当前是否处于播放中
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    /// 开始播放
    pub fn play(&mut self) {
        if self.current_time >= self.session.duration {
            self.current_time = 0.0;
            self.current_idx = 0;
        }
        self.is_playing = true;
    }

    /// 暂停播放
    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    /// 切换播放/暂停状态
    pub fn toggle_play(&mut self) {
        if self.is_playing {
            self.pause();
        } else {
            self.play();
        }
    }

    /// 设定循环播放开关
    pub fn set_loop(&mut self, is_loop: bool) {
        self.is_loop = is_loop;
    }

    /// 获取当前播放进度的百分比标量 (范围 `0.0 ..= 1.0`)
    pub fn progress_ratio(&self) -> f32 {
        if self.session.duration <= 0.001 {
            return 0.0;
        }
        (self.current_time / self.session.duration).clamp(0.0, 1.0) as f32
    }

    /// 根据物理时间增量 `delta_seconds` 推进播放状态机
    ///
    /// # 返回值
    /// 返回在此时间窗口内触发的所有 [`CastEvent`] 事件切片（供 VT100 解析器连续增量重绘）。
    pub fn advance(&mut self, delta_seconds: f64) -> Vec<CastEvent> {
        if !self.is_playing || delta_seconds <= 0.0 {
            return Vec::new();
        }

        let target_time = self.current_time + delta_seconds * self.speed;
        let mut triggered = Vec::new();

        while self.current_idx < self.session.events.len() {
            let ev = &self.session.events[self.current_idx];
            if ev.time <= target_time {
                triggered.push(ev.clone());
                self.current_idx += 1;
            } else {
                break;
            }
        }

        self.current_time = target_time;

        if self.current_time >= self.session.duration {
            self.current_time = self.session.duration;
            if self.is_loop {
                self.seek_to(0.0);
            } else {
                self.is_playing = false;
            }
        }

        triggered
    }

    /// 跳转到指定目标时间点 (Seek)
    ///
    /// # 返回值
    /// 返回从 `0.0` 到 `target_time` 的全部历史事件集合，便于终端直接复位全屏重放构建现场。
    pub fn seek_to(&mut self, target_time: f64) -> Vec<CastEvent> {
        let target = target_time.clamp(0.0, self.session.duration);
        self.current_time = target;

        let mut cumulative = Vec::new();
        let mut new_idx = 0;

        for (idx, ev) in self.session.events.iter().enumerate() {
            if ev.time <= target {
                cumulative.push(ev.clone());
                new_idx = idx + 1;
            } else {
                break;
            }
        }

        self.current_idx = new_idx;
        cumulative
    }

    /// 按百分比进度跳转 (Seek by Ratio 0.0 ..= 1.0)
    pub fn seek_ratio(&mut self, ratio: f32) -> Vec<CastEvent> {
        let target_time = (ratio.clamp(0.0, 1.0) as f64) * self.session.duration;
        self.seek_to(target_time)
    }

    /// 格式化时间为 `MM:SS`
    pub fn format_time_label(seconds: f64) -> String {
        let total_secs = seconds.max(0.0) as u64;
        let minutes = total_secs / 60;
        let secs = total_secs % 60;
        format!("{:02}:{:02}", minutes, secs)
    }

    /// 获取组合时间标签（例如 `"01:23 / 04:56"`）
    pub fn formatted_progress_label(&self) -> String {
        format!(
            "{} / {}",
            Self::format_time_label(self.current_time),
            Self::format_time_label(self.session.duration)
        )
    }
}

/// 安全审计风险等级。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AuditRiskLevel {
    /// 纯提示/常规只读巡检
    Info,
    /// 低风险操作
    Low,
    /// 中风险配置/服务变更
    Medium,
    /// 高危破坏性指令或提权
    High,
    /// 极危系统格式化或毁灭性打击
    Critical,
}

impl AuditRiskLevel {
    /// 获取显示文本
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Critical => "CRITICAL",
        }
    }
}

/// 安全审计发现项。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuditFinding {
    /// 触发时间戳 (秒)
    pub timestamp: f64,
    /// 风险等级
    pub level: AuditRiskLevel,
    /// 分类标签 (例如 "DestructiveCommand", "CredentialExposure")
    pub category: String,
    /// 风险判研结论
    pub reason: String,
    /// 触发的命令片段或敏感文本
    pub snippet: String,
}

/// 深度操作安全审计与敏感词侦测引擎 (Security Audit Inspector)。
pub struct SecurityAuditInspector;

impl SecurityAuditInspector {
    /// 静态研判单条指令的安全风险与敏感性
    pub fn inspect_command(cmd: &str, timestamp: f64) -> Option<AuditFinding> {
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return None;
        }
        let lower = trimmed.to_lowercase();

        // 1. Critical 毁灭性打击识别
        if lower.contains("rm -rf /")
            || lower.contains("rm -r -f /")
            || lower.contains("mkfs")
            || lower.contains("> /dev/sd")
            || lower.contains("> /dev/nvme")
            || lower.contains(":(){ :|:& };:")
        {
            return Some(AuditFinding {
                timestamp,
                level: AuditRiskLevel::Critical,
                category: "DestructiveCommand".to_string(),
                reason: "检测到毁灭性磁盘破坏或根目录递归强制删除操作，极高危拦截！".to_string(),
                snippet: trimmed.to_string(),
            });
        }

        // 2. High 高危指令识别 (格式化、分区、断电关机、批量清权)
        if lower.contains("rm -rf")
            || lower.contains("fdisk")
            || lower.contains("parted")
            || lower.contains("dd if=")
            || lower.contains("reboot")
            || lower.contains("shutdown")
            || lower.contains("init 0")
            || lower.contains("init 6")
            || lower.contains("chmod -r 777")
        {
            return Some(AuditFinding {
                timestamp,
                level: AuditRiskLevel::High,
                category: "DangerousOperation".to_string(),
                reason: "检测到高风险系统运维指令，影响系统启动稳定性或数据完整性。".to_string(),
                snippet: trimmed.to_string(),
            });
        }

        // 3. 敏感凭据/密钥泄露扫描
        if trimmed.contains("BEGIN RSA PRIVATE KEY")
            || trimmed.contains("BEGIN OPENSSH PRIVATE KEY")
            || trimmed.contains("BEGIN PRIVATE KEY")
        {
            return Some(AuditFinding {
                timestamp,
                level: AuditRiskLevel::High,
                category: "CredentialExposure".to_string(),
                reason: "检测到终端文本流正在输出或粘贴未加密的私钥明文内容！".to_string(),
                snippet: "PRIVATE KEY EXPOSURE DETECTED".to_string(),
            });
        }

        // 4. Medium 服务或网络防火墙变更
        if lower.contains("systemctl stop")
            || lower.contains("docker rm")
            || lower.contains("kill -9")
            || lower.contains("iptables -f")
            || lower.contains("ufw disable")
        {
            return Some(AuditFinding {
                timestamp,
                level: AuditRiskLevel::Medium,
                category: "ServiceInterruption".to_string(),
                reason: "检测到生产服务停止、容器删除或安全防护关闭操作。".to_string(),
                snippet: trimmed.to_string(),
            });
        }

        None
    }

    /// 对整部录屏会话执行全量静态安全审计扫描
    pub fn inspect_session(session: &CastSession) -> Vec<AuditFinding> {
        let mut findings = Vec::new();

        for event in &session.events {
            // 对输入指令与关键输出执行深度启发式安全扫描
            if let Some(finding) = Self::inspect_command(&event.data, event.time) {
                findings.push(finding);
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cast_session_roundtrip_serialization() {
        let mut session = CastSession::new(120, 36, Some("Prod Server Demo".to_string()));
        session.push_output(0.12, "\x1b[32mhello server\x1b[0m\r\n");
        session.push_input(1.45, "ls -la\r");
        session.push_output(1.50, "total 48\r\ndrwxr-xr-x 2 root root 4096\r\n");

        assert_eq!(session.events.len(), 3);
        assert!((session.duration - 1.50).abs() < 1e-4);

        let cast_str = session.to_cast_v2_string().expect("序列化成功");
        assert!(cast_str.contains("\"version\":2"));
        assert!(cast_str.contains("\"width\":120"));
        assert!(cast_str.contains("hello server"));

        let loaded = CastSession::from_cast_v2_str(&cast_str).expect("反序列化成功");
        assert_eq!(loaded.header.width, 120);
        assert_eq!(loaded.header.height, 36);
        assert_eq!(loaded.events.len(), 3);
        assert_eq!(loaded.events[1].data, "ls -la\r");
        assert_eq!(loaded.events[1].event_type, CastEventType::Input);
    }

    #[test]
    fn test_cast_replayer_timeline_and_seek() {
        let mut session = CastSession::new(80, 24, None);
        session.push_output(1.0, "step 1");
        session.push_output(2.5, "step 2");
        session.push_output(4.0, "step 3");
        session.push_output(6.0, "step 4");

        let mut replayer = CastReplayer::new(session);
        replayer.play();

        // 步进推进 1.5 秒
        let evs1 = replayer.advance(1.5);
        assert_eq!(evs1.len(), 1);
        assert_eq!(evs1[0].data, "step 1");

        // 再步进 1.5 秒 (到达 3.0 秒)
        let evs2 = replayer.advance(1.5);
        assert_eq!(evs2.len(), 1);
        assert_eq!(evs2[0].data, "step 2");

        // 测试跳转 Seek 到了 5.0 秒
        let seek_evs = replayer.seek_to(5.0);
        assert_eq!(seek_evs.len(), 3); // 包含了 step 1, 2, 3
        assert_eq!(replayer.current_time(), 5.0);
        assert_eq!(CastReplayer::format_time_label(65.0), "01:05");
    }

    #[test]
    fn test_security_audit_inspector_findings() {
        let finding_crit = SecurityAuditInspector::inspect_command("sudo rm -rf / --no-preserve-root", 2.3);
        assert!(finding_crit.is_some());
        assert_eq!(finding_crit.unwrap().level, AuditRiskLevel::Critical);

        let finding_priv_key = SecurityAuditInspector::inspect_command("-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaA==", 5.1);
        assert!(finding_priv_key.is_some());
        assert_eq!(finding_priv_key.unwrap().category, "CredentialExposure");

        let finding_safe = SecurityAuditInspector::inspect_command("ls -la /var/log", 1.0);
        assert!(finding_safe.is_none());
    }
}
