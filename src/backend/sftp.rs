//! SFTP 后端请求模型。

use crate::model::{TransferDirection, TransferId};

/// 后端 SFTP 操作请求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SftpRequest {
    ListDir {
        remote_path: String,
    },
    Upload {
        id: TransferId,
        local_path: String,
        remote_path: String,
    },
    Download {
        id: TransferId,
        remote_path: String,
        local_path: String,
    },
    RemoveFile {
        remote_path: String,
    },
    CreateDir {
        remote_path: String,
    },
}

impl SftpRequest {
    /// 返回传输方向；非传输类请求返回 `None`。
    pub fn transfer_direction(&self) -> Option<TransferDirection> {
        match self {
            Self::Upload { .. } => Some(TransferDirection::Upload),
            Self::Download { .. } => Some(TransferDirection::Download),
            _ => None,
        }
    }

    /// 返回请求涉及的主要远端路径，便于日志和状态展示。
    pub fn remote_path(&self) -> &str {
        match self {
            Self::ListDir { remote_path }
            | Self::Upload { remote_path, .. }
            | Self::Download { remote_path, .. }
            | Self::RemoveFile { remote_path }
            | Self::CreateDir { remote_path } => remote_path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn sftp_request_reports_transfer_direction_and_path() {
        let upload = SftpRequest::Upload {
            id: TransferId(Uuid::new_v4()),
            local_path: "C:/tmp/app.tar.gz".to_owned(),
            remote_path: "/tmp/app.tar.gz".to_owned(),
        };
        let list = SftpRequest::ListDir {
            remote_path: "/home/ops".to_owned(),
        };

        assert_eq!(upload.transfer_direction(), Some(TransferDirection::Upload));
        assert_eq!(upload.remote_path(), "/tmp/app.tar.gz");
        assert_eq!(list.transfer_direction(), None);
        assert_eq!(list.remote_path(), "/home/ops");
    }
}
