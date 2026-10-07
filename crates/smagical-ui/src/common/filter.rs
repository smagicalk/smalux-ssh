//! 大小写无关模糊搜索与过滤匹配公共工具 (Search & Filter Utilities)。
//!
//! 消除检索时逐字段 `.to_lowercase()` 造成的瞬时数千次堆内存分配，
//! 采用 ASCII 极速切片扫描快径，非 ASCII 平滑兼顾。

/// 大小写不敏感子串匹配：
/// 1. 优先采用纯 ASCII 字节切片滑动窗口扫描（零堆分配）；
/// 2. 包含 UTF-8 多字节字符时进行标准大小写转换比对。
#[inline]
pub fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    if haystack.len() < needle.len() {
        return false;
    }
    if haystack.is_ascii() && needle.is_ascii() {
        let needle_bytes = needle.as_bytes();
        let haystack_bytes = haystack.as_bytes();
        haystack_bytes.windows(needle_bytes.len()).any(|window| {
            window.iter().zip(needle_bytes.iter()).all(|(&a, &b)| {
                a.to_ascii_lowercase() == b.to_ascii_lowercase()
            })
        })
    } else {
        haystack.to_lowercase().contains(&needle.to_lowercase())
    }
}

/// 多候选字段模糊匹配：只要任意一个字段命中即返回 true。
#[inline]
pub fn matches_any_ignore_case(fields: &[&str], needle_lower: &str) -> bool {
    if needle_lower.is_empty() {
        return true;
    }
    fields.iter().any(|&f| contains_ignore_case(f, needle_lower))
}
