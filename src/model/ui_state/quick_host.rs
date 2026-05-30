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

pub use draft::{MAX_QUICK_HOST_NAME_CHARS, QuickHostDraft, truncate_host_name};
pub use error::QuickHostDraftError;
pub use types::{
    QuickHostAgentSource, QuickHostAuthDraft, QuickHostAuthField, QuickHostAuthKind,
    QuickHostDraftField,
};
