use crate::model::CredentialId;
use uuid::Uuid;

pub(super) fn new_credential_id() -> CredentialId {
    CredentialId(Uuid::new_v4())
}
