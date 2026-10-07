//! Slint 集合模型通用构造与 Diff 工具 (Model Utilities)。
//!
//! 统一 `ModelRc` 的快速构建并重新导出增量更新 Diff 引擎。

use slint::{ModelRc, VecModel};

/// 从标准 `Vec<T>` 快速构建 `slint::ModelRc<T>`。
#[inline]
pub fn to_model_rc<T: Clone + 'static>(items: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(items))
}
