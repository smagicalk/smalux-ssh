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
        total_bytes: Option<u64>,
        transferred_bytes: u64,
        status: TransferStatus,
    ) -> bool {
        if let Some(task) = self.transfers.iter_mut().find(|task| task.id == id) {
            if transfer_status_is_terminal(&task.status) {
                return false;
            }
            if total_bytes.is_some() {
                task.total_bytes = total_bytes;
            }
            task.transferred_bytes =
                normalized_transferred_bytes(task.total_bytes, transferred_bytes, &status);
            task.status = status;
            true
        } else {
            false
        }
    }

    /// 取消仍在队列中的传输任务。
    pub fn cancel_queued_transfer(&mut self, id: TransferId) -> bool {
        if let Some(task) = self.transfers.iter_mut().find(|task| task.id == id) {
            if !matches!(task.status, TransferStatus::Queued) {
                return false;
            }

            task.status = TransferStatus::Cancelled;
            true
        } else {
            false
        }
    }
}

fn transfer_status_is_terminal(status: &TransferStatus) -> bool {
    matches!(
        status,
        TransferStatus::Completed | TransferStatus::Failed { .. } | TransferStatus::Cancelled
    )
}

fn normalized_transferred_bytes(
    total_bytes: Option<u64>,
    transferred_bytes: u64,
    status: &TransferStatus,
) -> u64 {
    if matches!(status, TransferStatus::Completed) {
        total_bytes.unwrap_or(transferred_bytes)
    } else {
        transferred_bytes
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

        assert!(sessions.update_transfer_progress(id, Some(200), 200, TransferStatus::Completed));
        assert_eq!(sessions.transfers[0].total_bytes, Some(200));
        assert_eq!(sessions.transfers[0].transferred_bytes, 200);
        assert!(matches!(
            sessions.transfers[0].status,
            TransferStatus::Completed
        ));
        assert!(!sessions.update_transfer_progress(
            TransferId(Uuid::new_v4()),
            None,
            1,
            TransferStatus::Running
        ));
    }

    #[test]
    fn cancel_queued_transfer_only_updates_queued_tasks() {
        let mut sessions = SessionManager::default();
        let queued_id = TransferId(Uuid::new_v4());
        let running_id = TransferId(Uuid::new_v4());
        let host_id = host_id();
        let mut running = transfer_task(running_id, host_id);
        running.status = TransferStatus::Running;

        sessions.enqueue_transfer(transfer_task(queued_id, host_id));
        sessions.enqueue_transfer(running);

        assert!(sessions.cancel_queued_transfer(queued_id));
        assert!(matches!(
            sessions.transfers[0].status,
            TransferStatus::Cancelled
        ));
        assert!(!sessions.cancel_queued_transfer(running_id));
        assert!(matches!(
            sessions.transfers[1].status,
            TransferStatus::Running
        ));
        assert!(!sessions.cancel_queued_transfer(TransferId(Uuid::new_v4())));
    }

    #[test]
    fn completed_transfer_uses_known_total_as_final_progress() {
        let mut sessions = SessionManager::default();
        let id = TransferId(Uuid::new_v4());
        let host_id = host_id();

        sessions.enqueue_transfer(transfer_task(id, host_id));

        assert!(sessions.update_transfer_progress(id, Some(100), 80, TransferStatus::Completed));
        assert_eq!(sessions.transfers[0].total_bytes, Some(100));
        assert_eq!(sessions.transfers[0].transferred_bytes, 100);
        assert!(matches!(
            sessions.transfers[0].status,
            TransferStatus::Completed
        ));
    }

    #[test]
    fn terminal_transfer_status_ignores_late_progress() {
        let mut sessions = SessionManager::default();
        let cancelled_id = TransferId(Uuid::new_v4());
        let completed_id = TransferId(Uuid::new_v4());
        let failed_id = TransferId(Uuid::new_v4());
        let host_id = host_id();

        sessions.enqueue_transfer(transfer_task(cancelled_id, host_id));
        sessions.enqueue_transfer(transfer_task(completed_id, host_id));
        sessions.enqueue_transfer(transfer_task(failed_id, host_id));
        assert!(sessions.cancel_queued_transfer(cancelled_id));
        assert!(sessions.update_transfer_progress(
            completed_id,
            Some(100),
            80,
            TransferStatus::Completed
        ));
        assert!(sessions.update_transfer_progress(
            failed_id,
            None,
            0,
            TransferStatus::Failed {
                reason: "network".to_owned(),
            }
        ));

        assert!(!sessions.update_transfer_progress(
            cancelled_id,
            Some(100),
            50,
            TransferStatus::Running
        ));
        assert!(!sessions.update_transfer_progress(
            completed_id,
            Some(100),
            50,
            TransferStatus::Running
        ));
        assert!(!sessions.update_transfer_progress(
            failed_id,
            Some(100),
            100,
            TransferStatus::Completed
        ));

        assert!(matches!(
            sessions.transfers[0].status,
            TransferStatus::Cancelled
        ));
        assert_eq!(sessions.transfers[0].total_bytes, Some(100));
        assert_eq!(sessions.transfers[0].transferred_bytes, 0);
        assert!(matches!(
            sessions.transfers[1].status,
            TransferStatus::Completed
        ));
        assert_eq!(sessions.transfers[1].total_bytes, Some(100));
        assert_eq!(sessions.transfers[1].transferred_bytes, 100);
        assert!(matches!(
            &sessions.transfers[2].status,
            TransferStatus::Failed { reason } if reason == "network"
        ));
        assert_eq!(sessions.transfers[2].total_bytes, Some(100));
        assert_eq!(sessions.transfers[2].transferred_bytes, 0);
    }
}
