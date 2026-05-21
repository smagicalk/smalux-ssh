use super::*;
use crate::backend::BackendCommand;
use crate::model::{
    AuthProfile, Message, QuickHostAuthField, QuickHostAuthKind, QuickHostDraftField, SecretRef,
    SessionId, SftpActionDraftField,
};
use uuid::Uuid;

mod draft_fields;
mod quick_host;
mod terminal_input;
