use super::super::{VisualSettingsDraft, VisualSettingsDraftError};
use super::common::background;

#[test]
fn draft_reports_invalid_background_sources() {
    let draft = VisualSettingsDraft {
        background_sources: "url:".to_owned(),
        ..VisualSettingsDraft::default()
    };

    assert!(matches!(
        draft.build_background_profile(&background()),
        Err(VisualSettingsDraftError::InvalidBackgroundSource(_))
    ));
}
