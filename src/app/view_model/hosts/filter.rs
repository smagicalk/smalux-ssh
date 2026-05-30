//! 主机列表搜索过滤。
//!
//! 搜索使用展示层语义：名称、地址、端口、认证方式和 tag 都可以命中。这里不负责排序或
//! 分组，只回答“这个主机是否应该出现在当前查询结果中”。

use crate::model::{AuthProfile, Host};

pub(super) fn host_matches_query(host: &Host, query: &str) -> bool {
    // query 由调用方统一 trim/lowercase；这里保持纯谓词，方便测试。
    query.is_empty()
        || host.name.to_lowercase().contains(query)
        || host.address.to_lowercase().contains(query)
        || host.port.to_string().contains(query)
        || auth_matches_query(&host.auth, query)
        || host
            .tags
            .iter()
            .any(|tag| tag.to_lowercase().contains(query))
}

fn auth_matches_query(auth: &AuthProfile, query: &str) -> bool {
    // 认证方式允许中英文关键词命中，但这里仍返回布尔值，不把文案泄漏给 UI。
    match auth {
        AuthProfile::Password { .. } => "password 密码".contains(query),
        AuthProfile::Key { .. } => "key private-key 密钥 私钥".contains(query),
        AuthProfile::Agent { .. } => "agent ssh-agent".contains(query),
        AuthProfile::Certificate { .. } => "certificate cert 证书".contains(query),
    }
}
