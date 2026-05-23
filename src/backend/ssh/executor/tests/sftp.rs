use super::super::RusshBackendExecutor;
use super::common::{assert_channel_failure, session_id};
use crate::backend::{BackendCommand, SftpRequest};
use crate::security::MemorySecretStore;
use smagical_backend_core::BackendExecutor;

#[test]
fn sftp_requires_connected_session() {
    let mut executor =
        RusshBackendExecutor::new(MemorySecretStore::new()).expect("执行器应该可以创建 runtime");
    let session_id = session_id();

    let error = executor
        .execute(BackendCommand::Sftp {
            session_id,
            request: SftpRequest::ListDir {
                remote_path: "/".to_owned(),
            },
        })
        .expect_err("未连接会话不能打开 SFTP");

    assert_channel_failure(&error, "sftp", "session is not connected");
}
