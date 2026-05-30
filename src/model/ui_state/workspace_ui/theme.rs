//! 工作区内置视觉主题状态。

use serde::{Deserialize, Serialize};

use crate::config::BuiltInThemePreference;

/// 可选的内置视觉主题。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuiltInTheme {
    ProfessionalDark,
    CatppuccinMocha,
    NordDark,
    Dracula,
    SolarizedDark,
    OceanDark,
    ForestDark,
}

impl Default for BuiltInTheme {
    fn default() -> Self {
        Self::ProfessionalDark
    }
}

impl BuiltInTheme {
    pub const ALL: [Self; 7] = [
        Self::ProfessionalDark,
        Self::CatppuccinMocha,
        Self::NordDark,
        Self::Dracula,
        Self::SolarizedDark,
        Self::OceanDark,
        Self::ForestDark,
    ];

    pub fn next(self) -> Self {
        match self {
            Self::ProfessionalDark => Self::CatppuccinMocha,
            Self::CatppuccinMocha => Self::NordDark,
            Self::NordDark => Self::Dracula,
            Self::Dracula => Self::SolarizedDark,
            Self::SolarizedDark => Self::OceanDark,
            Self::OceanDark => Self::ForestDark,
            Self::ForestDark => Self::ProfessionalDark,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::ProfessionalDark => "ProfessionalDark",
            Self::CatppuccinMocha => "CatppuccinMocha",
            Self::NordDark => "NordDark",
            Self::Dracula => "Dracula",
            Self::SolarizedDark => "SolarizedDark",
            Self::OceanDark => "OceanDark",
            Self::ForestDark => "ForestDark",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "ProfessionalDark" => Some(Self::ProfessionalDark),
            "CatppuccinMocha" => Some(Self::CatppuccinMocha),
            "NordDark" => Some(Self::NordDark),
            "Dracula" => Some(Self::Dracula),
            "SolarizedDark" => Some(Self::SolarizedDark),
            "OceanDark" => Some(Self::OceanDark),
            "ForestDark" => Some(Self::ForestDark),
            _ => None,
        }
    }

    pub fn from_preference(preference: BuiltInThemePreference) -> Self {
        match preference {
            BuiltInThemePreference::ProfessionalDark => Self::ProfessionalDark,
            BuiltInThemePreference::CatppuccinMocha => Self::CatppuccinMocha,
            BuiltInThemePreference::NordDark => Self::NordDark,
            BuiltInThemePreference::Dracula => Self::Dracula,
            BuiltInThemePreference::SolarizedDark => Self::SolarizedDark,
            BuiltInThemePreference::OceanDark => Self::OceanDark,
            BuiltInThemePreference::ForestDark => Self::ForestDark,
        }
    }

    pub fn preference(self) -> BuiltInThemePreference {
        match self {
            Self::ProfessionalDark => BuiltInThemePreference::ProfessionalDark,
            Self::CatppuccinMocha => BuiltInThemePreference::CatppuccinMocha,
            Self::NordDark => BuiltInThemePreference::NordDark,
            Self::Dracula => BuiltInThemePreference::Dracula,
            Self::SolarizedDark => BuiltInThemePreference::SolarizedDark,
            Self::OceanDark => BuiltInThemePreference::OceanDark,
            Self::ForestDark => BuiltInThemePreference::ForestDark,
        }
    }
}

impl super::WorkspaceUiState {
    /// 切换到下一个内置主题。
    pub fn next_theme(&mut self) {
        self.theme = self.theme.next();
    }

    /// 设置内置主题。
    pub fn set_built_in_theme(&mut self, theme: BuiltInTheme) {
        self.theme = theme;
    }
}
