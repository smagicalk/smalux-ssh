//! 视觉配置草稿。

#[path = "visual_settings/draft.rs"]
mod draft;
#[path = "visual_settings/error.rs"]
mod error;
#[path = "visual_settings/field.rs"]
mod field;
#[path = "visual_settings/profiles.rs"]
mod profiles;
#[path = "visual_settings/ui_drafts.rs"]
mod ui_drafts;

pub use draft::VisualSettingsDraft;
pub use error::VisualSettingsDraftError;
pub use field::VisualSettingsDraftField;
pub use ui_drafts::HostVisualSettingsDraft;
