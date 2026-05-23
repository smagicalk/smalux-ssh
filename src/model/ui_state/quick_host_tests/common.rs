use uuid::Uuid;

use crate::model::HostId;

pub(super) fn host_id() -> HostId {
    HostId(Uuid::new_v4())
}
