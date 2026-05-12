use crate::model::{BackgroundProfile, ThemeProfile};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app_name: String,
    pub theme: ThemeProfile,
    pub background: BackgroundProfile,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_name: String::from("smagicalssh"),
            theme: ThemeProfile {
                name: String::from("Default Dark"),
                font_family: String::from("JetBrains Mono"),
                font_size: 14.0,
            },
            background: BackgroundProfile {
                enabled: false,
                sources: Vec::new(),
                rotation_interval_secs: 300,
                opacity: 0.18,
                blur: 8.0,
            },
        }
    }
}
