//! 启动流程共享远端路径处理。

pub(in crate::model::app_state) fn normalize_remote_dir(remote_dir: &str) -> String {
    let remote_dir = remote_dir.trim();

    if remote_dir.is_empty() {
        "/".to_owned()
    } else {
        remote_dir.to_owned()
    }
}

pub(in crate::model::app_state) fn join_remote_path(remote_dir: &str, name: &str) -> String {
    if remote_dir == "/" {
        format!("/{name}")
    } else {
        format!(
            "{}/{}",
            remote_dir.trim_end_matches('/'),
            name.trim_start_matches('/')
        )
    }
}
