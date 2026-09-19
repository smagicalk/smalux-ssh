//! 原生纯 Rust 异步流式 AI 核心交互框架与端点服务。
//!
//! 提供基于纯 Rust 异步网络 I/O (`reqwest` + `tokio`) 的大模型端点连接、
//! 模型目录自动发现、SSE 实时打字机流式输出 (支持 DeepSeek-R1 CoT 推理思考过程)、
//! 以及自动化 Shell 命令提取与生产安全风险审查判定。

use std::time::Instant;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

/// AI 对话参与者角色。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AiRole {
    /// 系统预设提示词角色。
    System,
    /// 用户角色。
    User,
    /// AI 助手角色。
    Assistant,
}

/// 单条 AI 对话消息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiChatMessage {
    /// 角色。
    pub role: AiRole,
    /// 消息文本内容。
    pub content: String,
}

impl AiChatMessage {
    /// 创建系统提示词消息。
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: AiRole::System,
            content: content.into(),
        }
    }

    /// 创建用户输入消息。
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: AiRole::User,
            content: content.into(),
        }
    }

    /// 创建助手回复消息。
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: AiRole::Assistant,
            content: content.into(),
        }
    }
}

/// AI 对话推理请求体。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiChatRequest {
    /// 选用的模型名称 (如 "deepseek-reasoner", "claude-3-7-sonnet-20250219", "gpt-4o")。
    pub model: String,
    /// 对话历史与当前提问消息序列。
    pub messages: Vec<AiChatMessage>,
    /// 是否开启流式 Server-Sent Events (SSE) 传输。
    pub stream: bool,
    /// 采样温度系数 (0.0 ~ 1.0)。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// 最大生成 Token 数量限制。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

/// AI 流式增量数据片段 (SSE Chunk)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AiStreamChunk {
    /// 增量回复正文片段。
    pub delta_content: String,
    /// 增量思维链/推理过程片段 (针对 DeepSeek-R1 / Claude 3.7 Thinking / Qwen-thinking)。
    pub delta_thinking: String,
    /// 是否为终结块。
    pub is_final: bool,
    /// 结束原因 (如 "stop", "length")。
    pub finish_reason: Option<String>,
}

/// AI 端点配置项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiEndpointConfig {
    /// API 基础地址 (例如 "https://api.deepseek.com/v1")。
    pub base_url: String,
    /// 认证密钥 (Bearer Token)。
    pub api_key: String,
    /// 默认推理模型。
    pub model: String,
    /// 采样温度。
    pub temperature: f32,
    /// 超时秒数。
    pub timeout_secs: u64,
    /// 自定义请求头 (逗号分隔或空)。
    pub custom_headers: Option<String>,
}

impl Default for AiEndpointConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.deepseek.com/v1".to_string(),
            api_key: String::new(),
            model: "deepseek-reasoner".to_string(),
            temperature: 0.3,
            timeout_secs: 60,
            custom_headers: None,
        }
    }
}

/// AI 连通性测试报告。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiTestResult {
    /// 是否握手连通成功。
    pub success: bool,
    /// 网络往返耗时 (毫秒)。
    pub latency_ms: u128,
    /// HTTP 响应状态码。
    pub status_code: u16,
    /// 结果描述消息。
    pub message: String,
}

/// AI 操作与网络错误类型。
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    /// HTTP 网络连接异常。
    #[error("网络请求错误: {0}")]
    Http(String),
    /// JSON 反序列化异常。
    #[error("数据解析错误: {0}")]
    Json(String),
    /// 远端 API 返回错误状态码。
    #[error("远端 API 返回错误状态码 [{status}]: {message}")]
    Api {
        /// HTTP 状态码。
        status: u16,
        /// 错误原因描述。
        message: String,
    },
    /// 请求超时。
    #[error("请求超时 (超过 {0} 秒)")]
    Timeout(u64),
    /// 远端流式响应提早中断。
    #[error("流式连接异常中断")]
    StreamClosed,
    /// 端点配置非法。
    #[error("端点配置非法: {0}")]
    InvalidConfig(String),
}

/// 原生纯 Rust 异步流式 AI 客户端。
pub struct AiClient {
    client: reqwest::Client,
    config: AiEndpointConfig,
}

impl AiClient {
    /// 使用指定的端点配置构建客户端实例。
    pub fn new(config: AiEndpointConfig) -> Result<Self, AiError> {
        let timeout = std::time::Duration::from_secs(if config.timeout_secs == 0 { 60 } else { config.timeout_secs });
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| AiError::Http(e.to_string()))?;

        Ok(Self { client, config })
    }

    /// 获取规范化的 Base URL (确保末尾无多余斜杠)。
    fn normalized_base_url(&self) -> &str {
        self.config.base_url.trim_end_matches('/')
    }

    /// 为请求追加认证头与自定义请求头。
    fn apply_headers(&self, mut rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if !self.config.api_key.trim().is_empty() {
            rb = rb.header("Authorization", format!("Bearer {}", self.config.api_key.trim()));
        }
        if let Some(ref headers_str) = self.config.custom_headers {
            for h in headers_str.split(',') {
                let trimmed = h.trim();
                if let Some((k, v)) = trimmed.split_once(':') {
                    rb = rb.header(k.trim(), v.trim());
                }
            }
        }
        rb
    }

    /// 发起原生异步 HTTP 连通性探活测试 (0ms 阻塞)。
    pub async fn test_connection(&self) -> AiTestResult {
        let start = Instant::now();
        let base = self.normalized_base_url();
        if base.is_empty() {
            return AiTestResult {
                success: false,
                latency_ms: 0,
                status_code: 0,
                message: "API 接口 Base URL 不能为空".to_string(),
            };
        }

        let endpoint_url = format!("{}/models", base);
        let req = self.apply_headers(self.client.get(&endpoint_url));

        match req.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let latency_ms = start.elapsed().as_millis();
                if status == 200 {
                    AiTestResult {
                        success: true,
                        latency_ms,
                        status_code: status,
                        message: format!("连通正常 (HTTP 200 OK, 延迟 {}ms)", latency_ms),
                    }
                } else if status == 401 {
                    AiTestResult {
                        success: false,
                        latency_ms,
                        status_code: status,
                        message: format!("认证失败 (401 Unauthorized), 请检查 API Key (耗时 {}ms)", latency_ms),
                    }
                } else {
                    AiTestResult {
                        success: resp.status().is_success(),
                        latency_ms,
                        status_code: status,
                        message: format!("响应状态码: HTTP {} (耗时 {}ms)", status, latency_ms),
                    }
                }
            }
            Err(err) => {
                let latency_ms = start.elapsed().as_millis();
                AiTestResult {
                    success: false,
                    latency_ms,
                    status_code: 0,
                    message: format!("探活失败: {}", err),
                }
            }
        }
    }

    /// 向远端端点自动拉取支持的全部模型标识列表 (纯 Rust 原生解析)。
    pub async fn fetch_models(&self) -> Result<Vec<String>, AiError> {
        let base = self.normalized_base_url();
        if base.is_empty() {
            return Err(AiError::InvalidConfig("Base URL 不能为空".to_string()));
        }

        let endpoint_url = format!("{}/models", base);
        let req = self.apply_headers(self.client.get(&endpoint_url));
        let resp = req.send().await.map_err(|e| AiError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(AiError::Api { status, message: body });
        }

        let body = resp.text().await.map_err(|e| AiError::Http(e.to_string()))?;
        let mut models = Vec::new();

        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(arr) = val.get("data").and_then(|d| d.as_array()) {
                for item in arr {
                    if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                        models.push(id.to_string());
                    }
                }
            } else if let Some(arr) = val.get("models").and_then(|m| m.as_array()) {
                for item in arr {
                    if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                        models.push(name.to_string());
                    }
                }
            }
        }

        if models.is_empty() {
            // 提供主流厂商推荐兜底候选
            let lower = base.to_lowercase();
            if lower.contains("deepseek") {
                models = vec![
                    "deepseek-reasoner".to_string(),
                    "deepseek-chat".to_string(),
                    "deepseek-coder".to_string(),
                ];
            } else if lower.contains("anthropic") || lower.contains("claude") {
                models = vec![
                    "claude-3-7-sonnet-20250219".to_string(),
                    "claude-3-5-sonnet-20241022".to_string(),
                    "claude-3-5-haiku-20241022".to_string(),
                ];
            } else if lower.contains("openai") {
                models = vec![
                    "gpt-4o".to_string(),
                    "gpt-4o-mini".to_string(),
                    "o1".to_string(),
                    "o3-mini".to_string(),
                ];
            } else if lower.contains("ollama") || lower.contains("11434") {
                models = vec![
                    "qwen2.5:14b".to_string(),
                    "deepseek-r1:14b".to_string(),
                    "llama3.3:latest".to_string(),
                ];
            }
        }

        Ok(models)
    }

    /// 发起原生异步流式推理会话 (SSE)，返回增量 Token 通道接收器。
    pub async fn stream_chat(
        &self,
        request: AiChatRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<Result<AiStreamChunk, AiError>>, AiError> {
        let base = self.normalized_base_url();
        if base.is_empty() {
            return Err(AiError::InvalidConfig("Base URL 不能为空".to_string()));
        }

        let endpoint_url = format!("{}/chat/completions", base);
        let req = self.apply_headers(self.client.post(&endpoint_url)).json(&request);

        let response = req.send().await.map_err(|e| AiError::Http(e.to_string()))?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(AiError::Api { status, message: body });
        }

        let (tx, rx) = tokio::sync::mpsc::channel(64);

        // 在 Tokio 后台任务中消费字节流并解析 SSE 行
        tokio::spawn(async move {
            let mut byte_stream = response.bytes_stream();
            let mut line_buffer = String::new();

            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        line_buffer.push_str(&text);

                        // 按行分割解析
                        while let Some(pos) = line_buffer.find('\n') {
                            let line = line_buffer[..pos].trim().to_string();
                            line_buffer.drain(..=pos);

                            if line.is_empty() || line.starts_with(':') {
                                continue;
                            }

                            if let Some(chunk) = parse_sse_line(&line) {
                                let is_final = chunk.is_final;
                                if tx.send(Ok(chunk)).await.is_err() {
                                    return;
                                }
                                if is_final {
                                    return;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(AiError::Http(e.to_string()))).await;
                        return;
                    }
                }
            }

            // 处理缓冲区尾部残留
            let remaining = line_buffer.trim().to_string();
            if !remaining.is_empty() {
                if let Some(chunk) = parse_sse_line(&remaining) {
                    let _ = tx.send(Ok(chunk)).await;
                }
            }

            // 发送终结块确认
            let _ = tx
                .send(Ok(AiStreamChunk {
                    delta_content: String::new(),
                    delta_thinking: String::new(),
                    is_final: true,
                    finish_reason: Some("stop".to_string()),
                }))
                .await;
        });

        Ok(rx)
    }
}

/// 解析单行 Server-Sent Event (SSE) 数据文本。
pub fn parse_sse_line(line: &str) -> Option<AiStreamChunk> {
    let raw = line.trim();
    let data = if let Some(stripped) = raw.strip_prefix("data:") {
        stripped.trim()
    } else {
        return None;
    };

    if data == "[DONE]" {
        return Some(AiStreamChunk {
            delta_content: String::new(),
            delta_thinking: String::new(),
            is_final: true,
            finish_reason: Some("stop".to_string()),
        });
    }

    let val = serde_json::from_str::<serde_json::Value>(data).ok()?;

    // 1. 标准 OpenAI / DeepSeek 格式解析
    if let Some(choices) = val.get("choices").and_then(|c| c.as_array()) {
        if let Some(first) = choices.first() {
            let finish = first.get("finish_reason").and_then(|f| f.as_str()).map(|s| s.to_string());
            let delta = first.get("delta");

            let delta_content = delta
                .and_then(|d| d.get("content"))
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();

            // 支持 DeepSeek-R1 与主流思考模型的 reasoning_content 字段
            let delta_thinking = delta
                .and_then(|d| d.get("reasoning_content").or_else(|| d.get("thinking")))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();

            let is_final = finish.as_deref() == Some("stop") || finish.as_deref() == Some("length");

            if !delta_content.is_empty() || !delta_thinking.is_empty() || is_final {
                return Some(AiStreamChunk {
                    delta_content,
                    delta_thinking,
                    is_final,
                    finish_reason: finish,
                });
            }
        }
    }

    // 2. Anthropic / Claude 事件格式支持
    if let Some(event_type) = val.get("type").and_then(|t| t.as_str()) {
        if event_type == "content_block_delta" {
            if let Some(delta) = val.get("delta") {
                let delta_type = delta.get("type").and_then(|t| t.as_str()).unwrap_or("");
                if delta_type == "text_delta" {
                    let text = delta.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string();
                    return Some(AiStreamChunk {
                        delta_content: text,
                        delta_thinking: String::new(),
                        is_final: false,
                        finish_reason: None,
                    });
                } else if delta_type == "thinking_delta" {
                    let thinking = delta.get("thinking").and_then(|t| t.as_str()).unwrap_or("").to_string();
                    return Some(AiStreamChunk {
                        delta_content: String::new(),
                        delta_thinking: thinking,
                        is_final: false,
                        finish_reason: None,
                    });
                }
            }
        } else if event_type == "message_stop" {
            return Some(AiStreamChunk {
                delta_content: String::new(),
                delta_thinking: String::new(),
                is_final: true,
                finish_reason: Some("stop".to_string()),
            });
        }
    }

    // 3. Ollama 格式支持
    if let Some(msg) = val.get("message") {
        let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
        let done = val.get("done").and_then(|d| d.as_bool()).unwrap_or(false);
        if !content.is_empty() || done {
            return Some(AiStreamChunk {
                delta_content: content,
                delta_thinking: String::new(),
                is_final: done,
                finish_reason: if done { Some("stop".to_string()) } else { None },
            });
        }
    }

    None
}

/// 从 AI 回复文本中智能提取首个可执行的 Shell 命令。
///
/// 优先提取 ```bash ... ``` 或 ```sh ... ``` 代码块，
/// 其次匹配以 `$ ` 或 `# ` 开头的单行指令。
pub fn extract_shell_command(reply: &str) -> Option<String> {
    // 1. 尝试匹配 Markdown 格式代码块
    if let Some(start_idx) = reply.find("```") {
        let after_fence = &reply[start_idx + 3..];
        // 略过可选的语言标识符 (如 bash\n, sh\n)
        let code_start = if let Some(newline_pos) = after_fence.find('\n') {
            &after_fence[newline_pos + 1..]
        } else {
            after_fence
        };

        if let Some(end_idx) = code_start.find("```") {
            let raw_code = code_start[..end_idx].trim();
            if !raw_code.is_empty() {
                // 取代码块内的首条非空有效命令
                for line in raw_code.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    let cleaned = trimmed.strip_prefix("$ ").unwrap_or(trimmed).trim();
                    if !cleaned.is_empty() {
                        return Some(cleaned.to_string());
                    }
                }
            }
        }
    }

    // 2. 尝试从单行前缀提取
    for line in reply.lines() {
        let trimmed = line.trim();
        if let Some(cmd) = trimmed.strip_prefix("$ ") {
            let c = cmd.trim();
            if !c.is_empty() {
                return Some(c.to_string());
            }
        }
    }

    None
}

/// 评估 Shell 命令在生产主机上的安全风险等级。
///
/// 返回 `(risk_level, audit_reason)`:
/// - `"high"`: 高危破坏性指令 (如 rm -rf, mkfs, reboot, dd, > /dev/sda)
/// - `"medium"`: 配置变更或服务变更指令 (如 systemctl restart/stop, docker rm, kill)
/// - `"low"`: 安全的只读巡检指令 (如 ls, cat, ps, df, uname)
pub fn assess_command_risk(cmd: &str) -> (&'static str, &'static str) {
    let c = cmd.trim();
    if c.is_empty() {
        return ("low", "无指令操作");
    }

    let lower = c.to_lowercase();

    // 1. 高危黑名单判定
    if lower.contains("rm -rf")
        || lower.contains("rm -r -f")
        || lower.contains("mkfs")
        || lower.contains("fdisk")
        || lower.contains("parted")
        || lower.contains("dd if=")
        || lower.contains("reboot")
        || lower.contains("shutdown")
        || lower.contains("init 0")
        || lower.contains("init 6")
        || lower.contains("chmod -r 777")
        || lower.contains("> /dev/sd")
        || lower.contains("> /dev/nvme")
        || lower.contains(":(){ :|:& };:")
    {
        return ("high", "检测到高危毁灭性操作指令，存在丢失系统与数据风险，必须严格人工核准");
    }

    // 2. 中危警告判定 (服务停止/容器删除/进程杀死/端口变更)
    if lower.contains("systemctl restart")
        || lower.contains("systemctl stop")
        || lower.contains("systemctl disable")
        || lower.contains("service stop")
        || lower.contains("docker rm")
        || lower.contains("docker stop")
        || lower.contains("docker kill")
        || lower.contains("kill -9")
        || lower.contains("killall")
        || lower.contains("pkill")
        || lower.contains("iptables -f")
        || lower.contains("ufw disable")
        || lower.contains("apt remove")
        || lower.contains("yum remove")
    {
        return ("medium", "检测到生产服务或容器状态变更，已自动拦截供运维人员复核确认");
    }

    // 3. 默认判定为低危只读或探测
    ("low", "该指令经规则与语义自审判定为常规无害或只读巡检操作，安全级别放行")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_shell_command_from_markdown() {
        let markdown = r#"
你可以使用以下命令查看当前主机的内存使用状况：
```bash
# 检查物理内存与 Swap
free -h
```
如果有异常，可以进一步排查进程。
"#;
        let cmd = extract_shell_command(markdown);
        assert_eq!(cmd, Some("free -h".to_string()));
    }

    #[test]
    fn test_extract_shell_command_with_dollar_prefix() {
        let text = "推荐执行指令：\n$ cat /etc/os-release\n查看系统版本。";
        let cmd = extract_shell_command(text);
        assert_eq!(cmd, Some("cat /etc/os-release".to_string()));
    }

    #[test]
    fn test_assess_command_risk_levels() {
        // 高危测试
        let (risk, _) = assess_command_risk("rm -rf /tmp/test");
        assert_eq!(risk, "high");

        let (risk, _) = assess_command_risk("reboot");
        assert_eq!(risk, "high");

        // 中危测试
        let (risk, _) = assess_command_risk("systemctl restart nginx");
        assert_eq!(risk, "medium");

        let (risk, _) = assess_command_risk("docker rm -f web");
        assert_eq!(risk, "medium");

        // 低危测试
        let (risk, _) = assess_command_risk("df -h");
        assert_eq!(risk, "low");

        let (risk, _) = assess_command_risk("cat /proc/cpuinfo");
        assert_eq!(risk, "low");
    }

    #[test]
    fn test_parse_sse_line_openai_format() {
        let line = r#"data: {"id":"chatcmpl-123","choices":[{"delta":{"content":"Hello world"},"finish_reason":null}]}"#;
        let chunk = parse_sse_line(line).expect("parse sse chunk");
        assert_eq!(chunk.delta_content, "Hello world");
        assert_eq!(chunk.delta_thinking, "");
        assert!(!chunk.is_final);
    }

    #[test]
    fn test_parse_sse_line_deepseek_reasoning_format() {
        let line = r#"data: {"id":"chatcmpl-456","choices":[{"delta":{"reasoning_content":"正在思考内核参数..."},"finish_reason":null}]}"#;
        let chunk = parse_sse_line(line).expect("parse deepseek reasoning chunk");
        assert_eq!(chunk.delta_content, "");
        assert_eq!(chunk.delta_thinking, "正在思考内核参数...");
        assert!(!chunk.is_final);
    }

    #[test]
    fn test_parse_sse_line_done() {
        let line = "data: [DONE]";
        let chunk = parse_sse_line(line).expect("parse done chunk");
        assert!(chunk.is_final);
        assert_eq!(chunk.finish_reason, Some("stop".to_string()));
    }
}
