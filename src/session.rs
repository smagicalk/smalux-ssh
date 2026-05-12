use crate::model::SessionId;

#[derive(Debug, Clone, Default)]
pub struct SessionManager {
    pub active: Vec<SessionId>,
}

impl SessionManager {
    pub fn active_count(&self) -> usize {
        self.active.len()
    }
}
