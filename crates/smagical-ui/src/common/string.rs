//! 零拷贝 Slint SharedString 转换公共特征 (ToSharedString)。
//!
//! 消除 `s.clone().into()` 造成的中间堆内存分配，直接通过底层切片引用构建 `slint::SharedString`。

use slint::SharedString;

/// 零拷贝转换为 `slint::SharedString` 的统一特征。
pub trait ToSharedString {
    /// 转换为 Slint 共享引用计数字符串。
    fn to_shared(&self) -> SharedString;
}

impl ToSharedString for str {
    #[inline]
    fn to_shared(&self) -> SharedString {
        self.into()
    }
}

impl ToSharedString for String {
    #[inline]
    fn to_shared(&self) -> SharedString {
        self.as_str().into()
    }
}

impl<T: AsRef<str>> ToSharedString for Option<T> {
    #[inline]
    fn to_shared(&self) -> SharedString {
        self.as_ref().map(|s| s.as_ref()).unwrap_or_default().into()
    }
}

/// 将数值类型直接格式化为 `slint::SharedString`。
#[inline]
pub fn num_to_shared(n: impl std::fmt::Display) -> SharedString {
    n.to_string().into()
}
