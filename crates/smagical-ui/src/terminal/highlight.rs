//! 终端智能语法高亮、关键词/正则匹配与可交互超链接识别引擎。
//!
//! 提供：
//! 1. `HighlightEngine`: 负责编译并维护运维高亮规则 (URL, IPv4, Error, Warn, Success 等)；
//! 2. `SpanHighlight`: 字符网格行内高亮区间与样式元数据；
//! 3. URL 与 IPv4 地址光标坐标反查与跨平台默认浏览器唤起能力。

use regex::Regex;

/// 单个编译后的高效正则高亮规则
#[derive(Clone, Debug)]
pub struct CompiledHighlightRule {
    /// 规则唯一标识符
    pub id: String,
    /// 原始正则表达式文本
    pub pattern_str: String,
    /// 编译后的正则表达式对象
    pub regex: Regex,
    /// 渲染色彩 RGBA
    pub color_rgba: [u8; 4],
    /// 是否带有下划线装饰
    pub is_underline: bool,
    /// 是否属于超链接 URL
    pub is_url: bool,
    /// 是否属于 IPv4 地址
    pub is_ip: bool,
}

/// 行内命中高亮区间元数据
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpanHighlight {
    /// 起始字符列号 (0-indexed)
    pub start_col: usize,
    /// 结束字符列号 (exclusive)
    pub end_col: usize,
    /// 高亮文字 RGBA 色彩
    pub color: [u8; 4],
    /// 是否包含下划线
    pub underline: bool,
    /// 是否属于超链接 URL
    pub is_url: bool,
}

/// 终端运维规则高亮与链接提取核心引擎
#[derive(Clone, Debug)]
pub struct HighlightEngine {
    rules: Vec<CompiledHighlightRule>,
}

impl Default for HighlightEngine {
    fn default() -> Self {
        let default_rules = vec![
            (
                "kw_err",
                r"\b(ERROR|FATAL|CRITICAL|Failed|Error|panic|EXCEPTION)\b",
                [0xef, 0x44, 0x44, 0xff], // 红
                false,
                false,
                false,
            ),
            (
                "kw_warn",
                r"\b(WARN|WARNING|Warning|Warn)\b",
                [0xf5, 0x9e, 0x0b, 0xff], // 橙黄
                false,
                false,
                false,
            ),
            (
                "kw_ok",
                r"\b(SUCCESS|OK|Finished|Done)\b",
                [0x10, 0xb9, 0x81, 0xff], // 绿
                false,
                false,
                false,
            ),
            (
                "kw_url",
                r"(?:https?://|www\.)[^\s<>`{}|]+",
                [0x3b, 0x82, 0xf6, 0xff], // 科技蓝
                true,
                true,
                false,
            ),
            (
                "kw_ip",
                r"\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)(?::\d{1,5})?\b",
                [0x8b, 0x5c, 0xf6, 0xff], // 亮紫
                false,
                false,
                true,
            ),
        ];

        let mut compiled = Vec::new();
        for (id, pattern, color, underline, is_url, is_ip) in default_rules {
            if let Ok(re) = Regex::new(pattern) {
                compiled.push(CompiledHighlightRule {
                    id: id.to_string(),
                    pattern_str: pattern.to_string(),
                    regex: re,
                    color_rgba: color,
                    is_underline: underline,
                    is_url,
                    is_ip,
                });
            }
        }

        Self { rules: compiled }
    }
}

impl HighlightEngine {
    /// 创建空规则引擎
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// 从规则列表更新高亮引擎
    pub fn update_from_rules<I>(&mut self, rules_iter: I)
    where
        I: IntoIterator<Item = (String, String, [u8; 4], bool)>, // (id, pattern, rgba, enabled)
    {
        let mut new_rules = Vec::new();
        for (id, pattern, color, enabled) in rules_iter {
            if !enabled || pattern.trim().is_empty() {
                continue;
            }
            if let Ok(re) = Regex::new(&pattern) {
                let is_url = pattern.contains("http://") || pattern.contains("https://") || id.contains("url");
                let is_ip = pattern.contains("\\d{1,3}") || id.contains("ip");
                let is_underline = is_url;
                new_rules.push(CompiledHighlightRule {
                    id,
                    pattern_str: pattern,
                    regex: re,
                    color_rgba: color,
                    is_underline,
                    is_url,
                    is_ip,
                });
            }
        }
        self.rules = new_rules;
    }

    /// 针对单行纯文本快速匹配全部高亮规则并合并产出染色区间
    pub fn match_line(&self, line_text: &str) -> Vec<SpanHighlight> {
        if line_text.is_empty() || self.rules.is_empty() {
            return Vec::new();
        }

        let mut spans = Vec::new();

        // 字符字节偏移量到字符列号的快速映射表
        let mut byte_to_char_col = Vec::with_capacity(line_text.len() + 1);
        let mut current_col = 0usize;
        for (byte_idx, _) in line_text.char_indices() {
            while byte_to_char_col.len() < byte_idx {
                byte_to_char_col.push(current_col.saturating_sub(1));
            }
            byte_to_char_col.push(current_col);
            current_col += 1;
        }
        while byte_to_char_col.len() <= line_text.len() {
            byte_to_char_col.push(current_col);
        }

        for rule in &self.rules {
            for m in rule.regex.find_iter(line_text) {
                let b_start = m.start();
                let b_end = m.end();
                let c_start = byte_to_char_col.get(b_start).copied().unwrap_or(0);
                let c_end = byte_to_char_col.get(b_end).copied().unwrap_or(current_col);

                if c_start < c_end {
                    spans.push(SpanHighlight {
                        start_col: c_start,
                        end_col: c_end,
                        color: rule.color_rgba,
                        underline: rule.is_underline,
                        is_url: rule.is_url,
                    });
                }
            }
        }

        spans
    }

    /// 根据鼠标点击的终端字符列坐标，反查命中的 URL 或 IP 地址字符串
    pub fn detect_url_or_ip_at(&self, line_text: &str, click_col: usize) -> Option<(String, bool /* is_url */)> {
        if line_text.is_empty() {
            return None;
        }

        let spans = self.match_line(line_text);
        for span in spans {
            let in_span = (click_col >= span.start_col && click_col < span.end_col)
                || (click_col + 1 >= span.start_col && click_col <= span.end_col);
            if in_span {
                // 提取匹配的子字符串
                let chars: Vec<char> = line_text.chars().collect();
                if span.start_col < chars.len() {
                    let end = span.end_col.min(chars.len());
                    let extracted: String = chars[span.start_col..end].iter().collect();
                    if span.is_url {
                        // 清理尾部标点符号如 ) ] > , .
                        let clean_url = extracted.trim_end_matches(&[')', ']', '>', ',', '.', ';', '\'', '"'][..]);
                        let clean_len = clean_url.chars().count();
                        if click_col > span.start_col + clean_len {
                            continue;
                        }
                        return Some((clean_url.to_string(), true));
                    } else {
                        return Some((extracted, false));
                    }
                }
            }
        }

        None
    }
}

/// 跨平台打开系统默认浏览器访问指定 URL
pub fn open_browser_url(url: &str) -> Result<(), String> {
    let clean_url = url.trim();
    if clean_url.is_empty() {
        return Err("URL 不能为空".to_string());
    }

    let full_url = if !clean_url.starts_with("http://") && !clean_url.starts_with("https://") {
        format!("https://{clean_url}")
    } else {
        clean_url.to_string()
    };

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // 优先使用 Windows 内建的 rundll32 FileProtocolHandler，杜绝 cmd /c 对带有 '&' 或 '?' 的 URL 进行命令行转义截断
        if let Ok(_) = std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", &full_url])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn()
        {
            return Ok(());
        }

        std::process::Command::new("cmd")
            .args(["/c", "start", "", &full_url])
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| format!("启动默认浏览器失败: {}", e))?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&full_url)
            .spawn()
            .map_err(|e| format!("启动默认浏览器失败: {}", e))?;
        Ok(())
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&full_url)
            .spawn()
            .map_err(|e| format!("启动默认浏览器失败: {}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_engine_matches_url_and_ip() {
        let engine = HighlightEngine::default();
        let sample = "Check server at 192.168.1.100:8080 or visit https://example.com/api/test for docs";

        let spans = engine.match_line(sample);
        assert!(!spans.is_empty(), "必须命中高亮");

        // 验证 URL 检测
        let (found_url, is_url) = engine.detect_url_or_ip_at(sample, 50).expect("必须命中 URL");
        assert!(is_url);
        assert_eq!(found_url, "https://example.com/api/test");

        // 验证 IP 检测
        let (found_ip, is_url_2) = engine.detect_url_or_ip_at(sample, 20).expect("必须命中 IP");
        assert!(!is_url_2);
        assert_eq!(found_ip, "192.168.1.100:8080");
    }

    #[test]
    fn test_highlight_engine_matches_error_keywords() {
        let engine = HighlightEngine::default();
        let sample = "[2026-09-22 18:00:00] [ERROR] Connection to database FATAL failed!";

        let spans = engine.match_line(sample);
        let error_spans: Vec<_> = spans.iter().filter(|s| s.color == [0xef, 0x44, 0x44, 0xff]).collect();
        assert!(!error_spans.is_empty(), "必须识别出 ERROR 关键词高亮");
    }
}
