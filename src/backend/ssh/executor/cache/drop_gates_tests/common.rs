use uuid::Uuid;

use crate::model::SessionId;

pub(super) fn session_id() -> SessionId {
    SessionId(Uuid::new_v4())
}
