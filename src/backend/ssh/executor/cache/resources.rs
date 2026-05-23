//! SSH executor 会话资源缓存操作。

#[path = "resources/replace.rs"]
mod replace;
#[path = "resources/take.rs"]
mod take;
#[path = "resources/types.rs"]
mod types;

pub(in crate::backend::ssh::executor) use replace::{replace_cached_sftp, replace_cached_shell};
#[cfg(test)]
pub(in crate::backend::ssh::executor) use take::take_cached_session_resources;
pub(in crate::backend::ssh::executor) use take::{
    take_cached_session_runtime_resources, take_cached_session_subresources,
};
pub(in crate::backend::ssh::executor) use types::{
    CachedSessionResources, CachedSessionSubresources,
};

#[cfg(test)]
#[path = "resources_tests.rs"]
mod tests;
