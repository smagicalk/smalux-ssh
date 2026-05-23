use super::super::{BackgroundProfile, ThemeProfile, VisualSettingsDraft};
use super::common::background as default_background;
use crate::model::ImageSource;

#[test]
fn draft_round_trips_profiles() {
    let theme = ThemeProfile {
        name: "Solarized".to_owned(),
        font_family: "Maple Mono".to_owned(),
        font_size: 15.5,
    };
    let background = BackgroundProfile {
        enabled: true,
        sources: vec![
            ImageSource::LocalPath("wallpapers/a.jpg".to_owned()),
            ImageSource::Url("https://example.com/b.jpg".to_owned()),
        ],
        rotation_interval_secs: 120,
        opacity: 0.4,
        blur: 12.0,
    };

    let draft = VisualSettingsDraft::from_profiles(&theme, &background);
    let rebuilt_theme = draft
        .build_theme_profile(&ThemeProfile {
            name: String::new(),
            font_family: String::new(),
            font_size: 14.0,
        })
        .expect("主题草稿应该可以还原");
    let rebuilt_background = draft
        .build_background_profile(&default_background())
        .expect("背景草稿应该可以还原");

    assert_eq!(rebuilt_theme, theme);
    assert_eq!(rebuilt_background, background.normalized());
}
