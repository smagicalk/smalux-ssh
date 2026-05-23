#[derive(Debug, PartialEq, Eq)]
pub(in crate::backend::ssh::executor) struct CachedSessionSubresources<TShell, TSftp> {
    pub(in crate::backend::ssh::executor) shell: Option<TShell>,
    pub(in crate::backend::ssh::executor) sftp: Option<TSftp>,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::backend::ssh::executor) struct CachedSessionResources<TShell, TSftp, TConnection> {
    pub(in crate::backend::ssh::executor) shell: Option<TShell>,
    pub(in crate::backend::ssh::executor) sftp: Option<TSftp>,
    pub(in crate::backend::ssh::executor) connection: Option<TConnection>,
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::backend::ssh::executor) struct CachedSessionRuntimeResources<
    TShell,
    TSftp,
    TConnection,
    TTunnel,
> {
    pub(in crate::backend::ssh::executor) cached_resources:
        CachedSessionResources<TShell, TSftp, TConnection>,
    pub(in crate::backend::ssh::executor) tunnels: Vec<TTunnel>,
}
