//! SFTP 传输队列运行态操作。

use crate::model::{TransferId, TransferStatus, TransferTask};

use super::SessionManager;

impl SessionManager {
    /// 加入一个 SFTP 传输任务。
    pub fn enqueue_transfer(&mut self, task: TransferTask) {
        if let Some(existing) = self
            .transfers
            .iter_mut()
            .find(|existing| existing.id == task.id)
        {
            *existing = task;
        } else {
            self.transfers.push(task);
        }
    }

    /// 更新传输进度。
    pub fn update_transfer_progress(
        &mut self,
        id: TransferId,
        transferred_bytes: u64,
        status: TransferStatus,
    ) -> bool {
        if let Some(task) = self.transfers.iter_mut().find(|task| task.id == id) {
            task.transferred_bytes = transferred_bytes;
            task.status = status;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{HostId, TransferDirection};
    use uuid::Uuid;

    fn host_id() -> HostId {
        HostId(Uuid::new_v4())
    }

    fn transfer_task(id: TransferId, host_id: HostId) -> TransferTask {
        TransferTask {
            id,
            host_id,
            direction: TransferDirection::Download,
            local_path: "C:/tmp/syslog".to_owned(),
            remote_path: "/var/log/syslog".to_owned(),
            total_bytes: Some(100),
            transferred_bytes: 0,
            status: TransferStatus::Queued,
        }
    }

    #[test]
    fn transfer_queue_can_enqueue_replace_and_update_progress() {
        let mut sessions = SessionManager::default();
        let id = TransferId(Uuid::new_v4());
        let host_id = host_id();
        let mut updated = transfer_task(id, host_id);
        updated.total_bytes = Some(200);

        sessions.enqueue_transfer(transfer_task(id, host_id));
        sessions.enqueue_transfer(updated);

        assert_eq!(sessions.transfer_count(), 1);
        assert_eq!(sessions.transfers[0].total_bytes, Some(200));

        assert!(sessions.update_transfer_progress(id, 200, TransferStatus::Completed));
        assert_eq!(sessions.transfers[0].transferred_bytes, 200);
        assert!(matches!(
            sessions.transfers[0].status,
            TransferStatus::Completed
        ));
        assert!(!sessions.update_transfer_progress(
            TransferId(Uuid::new_v4()),
            1,
            TransferStatus::Running
        ));
    }
}
