//! 共享颜色空间转换工具函数 (Color Space Utilities)
//!
//! 提供 HSV <-> RGB 互相转换以及十六进制 HEX 颜色解析与序列化支持。

/// HSV 转换为 RGB (h in [0, 360], s in [0, 1], v in [0, 1])
pub(crate) fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let h_prime = (h / 60.0) % 6.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = if (0.0..1.0).contains(&h_prime) {
        (c, x, 0.0)
    } else if (1.0..2.0).contains(&h_prime) {
        (x, c, 0.0)
    } else if (2.0..3.0).contains(&h_prime) {
        (0.0, c, x)
    } else if (3.0..4.0).contains(&h_prime) {
        (0.0, x, c)
    } else if (4.0..5.0).contains(&h_prime) {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    (
        ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

/// RGB 转换为 HSV (r, g, b in [0, 255])
pub(crate) fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r_norm = r as f32 / 255.0;
    let g_norm = g as f32 / 255.0;
    let b_norm = b as f32 / 255.0;
    let max = r_norm.max(g_norm).max(b_norm);
    let min = r_norm.min(g_norm).min(b_norm);
    let delta = max - min;
    let v = max;
    let s = if max == 0.0 { 0.0 } else { delta / max };
    let h = if delta == 0.0 {
        0.0
    } else if (max - r_norm).abs() < 1e-5 {
        60.0 * (((g_norm - b_norm) / delta) % 6.0)
    } else if (max - g_norm).abs() < 1e-5 {
        60.0 * (((b_norm - r_norm) / delta) + 2.0)
    } else {
        60.0 * (((r_norm - g_norm) / delta) + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };
    (h, s, v)
}

/// 解析十六进制 HEX 颜色字符串为 (r, g, b)
pub(crate) fn parse_hex_to_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let h = hex.trim().trim_start_matches('#');
    if h.len() == 6 {
        let r = u8::from_str_radix(&h[0..2], 16).ok()?;
        let g = u8::from_str_radix(&h[2..4], 16).ok()?;
        let b = u8::from_str_radix(&h[4..6], 16).ok()?;
        Some((r, g, b))
    } else if h.len() == 3 {
        let r = u8::from_str_radix(&h[0..1], 16).ok()? * 17;
        let g = u8::from_str_radix(&h[1..2], 16).ok()? * 17;
        let b = u8::from_str_radix(&h[2..3], 16).ok()? * 17;
        Some((r, g, b))
    } else {
        None
    }
}
