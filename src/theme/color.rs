//! 主题颜色字符串的解析和格式化。

use super::ThemeError;

/// 将 `#rrggbb` 或 `#rrggbbaa` 统一成 `#rrggbb` / `#rrggbbaa` 小写格式。
///
/// Slint 投影层最终使用 ARGB `u32`，但原生主题文件保留 Web 常见的
/// `#rrggbb` / `#rrggbbaa` 表示，方便用户手写和从外部主题导入。
pub(super) fn normalize_color(field: &'static str, value: &str) -> Result<String, ThemeError> {
    parse_color(field, value).map(format_color)
}

/// 解析主题颜色字符串，返回 ARGB。
pub(super) fn parse_color(field: &'static str, value: &str) -> Result<u32, ThemeError> {
    let value = value.trim();
    let hex = value
        .strip_prefix('#')
        .ok_or_else(|| ThemeError::InvalidColor {
            field,
            value: value.to_owned(),
        })?;
    if hex.len() != 6 && hex.len() != 8 {
        return Err(ThemeError::InvalidColor {
            field,
            value: value.to_owned(),
        });
    }
    let parsed = u32::from_str_radix(hex, 16);
    parsed
        .map_err(|_| ThemeError::InvalidColor {
            field,
            value: value.to_owned(),
        })
        .map(|rgb| {
            if hex.len() == 6 {
                0xff00_0000 | rgb
            } else {
                rgb
            }
        })
}

/// 把 ARGB 颜色格式化回原生主题文件使用的字符串。
pub(super) fn format_color(argb: u32) -> String {
    if argb >> 24 == 0xff {
        format!("#{:06x}", argb & 0x00ff_ffff)
    } else {
        format!("#{:08x}", argb)
    }
}

/// 把 `0xrrggbb` 常量提升成不透明 ARGB。
pub(super) const fn rgb(value: u32) -> u32 {
    0xff00_0000 | value
}
