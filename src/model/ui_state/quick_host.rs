//! 快速新增主机表单和认证草稿。

#[path = "quick_host/auth.rs"]
mod auth;
#[path = "quick_host/draft.rs"]
mod draft;
#[path = "quick_host/error.rs"]
mod error;
#[path = "quick_host/types.rs"]
mod types;
#[path = "quick_host/ui.rs"]
mod ui;

pub use draft::QuickHostDraft;
pub use error::QuickHostDraftError;
pub use types::{QuickHostAuthDraft, QuickHostAuthField, QuickHostAuthKind, QuickHostDraftField};
