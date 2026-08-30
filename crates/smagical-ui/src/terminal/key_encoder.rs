//! 终端键盘按键编码器 (Terminal Keyboard Event Encoder)。
//!
//! 将 Slint 前端捕获的字符、修饰键（Ctrl, Alt, Shift）与特殊功能键精确翻译为 ANSI / VT100 / xterm 控制转义序列。

/// 将 Slint 键盘事件文本与修饰键状态编码为发送至 PTY 的原始 ANSI 字节序列。
///
/// # 参数
/// - `text`: 按键文本或 Slint 特殊键字符串
/// - `is_ctrl`: 是否按住 Ctrl 键
/// - `is_shift`: 是否按住 Shift 键
/// - `is_alt`: 是否按住 Alt 键
///
/// # 返回值
/// 返回编码后的 ANSI 控制字节切片向量。
pub fn encode_key_event(text: &str, is_ctrl: bool, _is_shift: bool, is_alt: bool) -> Vec<u8> {
    if text.is_empty() {
        return Vec::new();
    }

    let first_char = text.chars().next().unwrap();

    // 1. 处理 Ctrl 组合键 (Ctrl+A ~ Ctrl+Z 及 ASCII 控制符)
    if is_ctrl {
        if first_char.is_ascii_alphabetic() {
            let code = (first_char.to_ascii_uppercase() as u8) - b'@';
            return vec![code];
        }
        match first_char {
            '@' => return vec![0],
            '[' => return vec![27],
            '\\' => return vec![28],
            ']' => return vec![29],
            '^' => return vec![30],
            '_' => return vec![31],
            '?' => return vec![127],
            _ => {}
        }
    }

    // 2. 处理特殊功能键与方向键 (匹配 Slint 平台原生 Unicode 码点)
    let special_bytes: Option<&'static [u8]> = match first_char {
        // 回车 / 换行 (CR)
        '\n' | '\r' => Some(b"\r"),
        // 退格键 (BS / DEL)
        '\u{0008}' | '\u{007f}' => Some(b"\x08"),
        // 制表符 (Tab)
        '\t' => Some(b"\t"),
        // 反向制表符 (Shift+Tab)
        '\u{0019}' => Some(b"\x1b[Z"),
        // Escape
        '\u{001b}' => Some(b"\x1b"),
        // 方向键
        '\u{F700}' => Some(b"\x1b[A"), // Up
        '\u{F701}' => Some(b"\x1b[B"), // Down
        '\u{F703}' => Some(b"\x1b[C"), // Right
        '\u{F702}' => Some(b"\x1b[D"), // Left
        // 导航键
        '\u{F729}' => Some(b"\x1b[H"), // Home
        '\u{F72B}' => Some(b"\x1b[F"), // End
        '\u{F72C}' => Some(b"\x1b[5~"), // PageUp
        '\u{F72D}' => Some(b"\x1b[6~"), // PageDown
        '\u{F727}' => Some(b"\x1b[2~"), // Insert
        '\u{F728}' => Some(b"\x1b[3~"), // Delete
        // 功能键 F1 - F12
        '\u{F704}' => Some(b"\x1bOP"),
        '\u{F705}' => Some(b"\x1bOQ"),
        '\u{F706}' => Some(b"\x1bOR"),
        '\u{F707}' => Some(b"\x1bOS"),
        '\u{F708}' => Some(b"\x1b[15~"),
        '\u{F709}' => Some(b"\x1b[17~"),
        '\u{F70A}' => Some(b"\x1b[18~"),
        '\u{F70B}' => Some(b"\x1b[19~"),
        '\u{F70C}' => Some(b"\x1b[20~"),
        '\u{F70D}' => Some(b"\x1b[21~"),
        '\u{F70E}' => Some(b"\x1b[23~"),
        '\u{F70F}' => Some(b"\x1b[24~"),
        _ => None,
    };

    if let Some(bytes) = special_bytes {
        if is_alt {
            let mut res = vec![0x1b];
            res.extend_from_slice(bytes);
            return res;
        }
        return bytes.to_vec();
    }

    // 3. 处理普通 UTF-8 文本字符 (若有 Alt 则加 ESC 前缀)
    let raw_bytes = text.as_bytes();
    if is_alt {
        let mut res = vec![0x1b];
        res.extend_from_slice(raw_bytes);
        res
    } else {
        raw_bytes.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_ctrl_keys() {
        assert_eq!(encode_key_event("c", true, false, false), vec![3]); // Ctrl+C -> \x03
        assert_eq!(encode_key_event("d", true, false, false), vec![4]); // Ctrl+D -> \x04
        assert_eq!(encode_key_event("z", true, false, false), vec![26]); // Ctrl+Z -> \x1a
        assert_eq!(encode_key_event("a", true, false, false), vec![1]); // Ctrl+A -> \x01
        assert_eq!(encode_key_event("l", true, false, false), vec![12]); // Ctrl+L -> \x0c
    }

    #[test]
    fn test_encode_special_keys() {
        assert_eq!(encode_key_event("\n", false, false, false), b"\r".to_vec());
        assert_eq!(encode_key_event("\t", false, false, false), b"\t".to_vec());
        assert_eq!(encode_key_event("\u{F700}", false, false, false), b"\x1b[A".to_vec());
        assert_eq!(encode_key_event("\u{F701}", false, false, false), b"\x1b[B".to_vec());
        assert_eq!(encode_key_event("\u{F703}", false, false, false), b"\x1b[C".to_vec());
        assert_eq!(encode_key_event("\u{F702}", false, false, false), b"\x1b[D".to_vec());
    }

    #[test]
    fn test_encode_normal_text() {
        assert_eq!(encode_key_event("ls -la", false, false, false), b"ls -la".to_vec());
        assert_eq!(encode_key_event("中文测试", false, false, false), "中文测试".as_bytes().to_vec());
    }
}
