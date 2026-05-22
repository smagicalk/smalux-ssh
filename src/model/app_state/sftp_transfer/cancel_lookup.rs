//! SFTP 取消传输任务定位。

use crate::model::{TransferId, TransferTask};

pub(super) fn unique_transfer_task(
    tasks: &[TransferTask],
    transfer_id: TransferId,
) -> TransferLookup {
    let mut matches = tasks.iter().filter(|task| task.id == transfer_id);
    let Some(task) = matches.next() else {
        return TransferLookup::Missing;
    };
    if matches.next().is_some() {
        return TransferLookup::Ambiguous;
    }

    TransferLookup::Found(task.clone())
}

pub(super) enum TransferLookup {
    Found(TransferTask),
    Missing,
    Ambiguous,
}
