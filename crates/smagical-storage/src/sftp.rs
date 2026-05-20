//! SFTP 书签的内存索引操作。

use smagical_core::{HostId, SftpBookmark};

use super::StorageManager;

impl StorageManager {
    /// 保存或更新 SFTP 书签。
    pub fn upsert_sftp_bookmark(&mut self, bookmark: SftpBookmark) {
        if let Some(existing) = self.sftp_bookmarks.iter_mut().find(|existing| {
            existing.host_id == bookmark.host_id && existing.remote_path == bookmark.remote_path
        }) {
            *existing = bookmark;
        } else {
            self.sftp_bookmarks.push(bookmark);
        }
    }

    /// 查询某台主机的 SFTP 书签。
    pub fn sftp_bookmarks_for_host(&self, host_id: HostId) -> Vec<&SftpBookmark> {
        self.sftp_bookmarks
            .iter()
            .filter(|bookmark| bookmark.host_id == host_id)
            .collect()
    }

    /// 删除指定主机和路径的 SFTP 书签。
    pub fn remove_sftp_bookmark(&mut self, host_id: HostId, remote_path: &str) -> bool {
        let before = self.sftp_bookmarks.len();
        self.sftp_bookmarks
            .retain(|bookmark| bookmark.host_id != host_id || bookmark.remote_path != remote_path);
        before != self.sftp_bookmarks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn sftp_bookmarks_can_be_upserted_filtered_and_removed() {
        let mut storage = StorageManager::default();
        let host_id = HostId(Uuid::new_v4());
        let other_host_id = HostId(Uuid::new_v4());

        storage.upsert_sftp_bookmark(SftpBookmark {
            host_id,
            label: "home".to_owned(),
            remote_path: "/home/ops".to_owned(),
        });
        storage.upsert_sftp_bookmark(SftpBookmark {
            host_id,
            label: "home updated".to_owned(),
            remote_path: "/home/ops".to_owned(),
        });
        storage.upsert_sftp_bookmark(SftpBookmark {
            host_id: other_host_id,
            label: "logs".to_owned(),
            remote_path: "/var/log".to_owned(),
        });

        let bookmarks = storage.sftp_bookmarks_for_host(host_id);

        assert_eq!(storage.sftp_bookmark_count(), 2);
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].label, "home updated");
        assert!(storage.remove_sftp_bookmark(host_id, "/home/ops"));
        assert!(!storage.remove_sftp_bookmark(host_id, "/home/ops"));
        assert_eq!(storage.sftp_bookmarks_for_host(host_id).len(), 0);
    }
}
