//! 工作区内置视觉主题状态。

use serde::{Deserialize, Serialize};

/// 可选的内置视觉主题。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuiltInTheme {
    ProfessionalDark,
    OceanDark,
    ForestDark,
}

impl Default for BuiltInTheme {
    fn default() -> Self {
        Self::ProfessionalDark
    }
}
