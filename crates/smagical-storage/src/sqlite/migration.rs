//! SQLite schema migrations。
//!
//! 迁移只描述物理表结构，不包含业务转换逻辑。业务结构和表结构之间的映射放在
//! `mapper.rs`，这样后续拆表或合表时可以独立演进。

use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        // SeaORM 按顺序执行迁移。新增 schema 变更时追加新的 migration，不修改旧迁移。
        vec![Box::new(M20260528000000CreateCoreStorageSchema)]
    }
}

#[derive(DeriveMigrationName)]
struct M20260528000000CreateCoreStorageSchema;

#[async_trait::async_trait]
impl MigrationTrait for M20260528000000CreateCoreStorageSchema {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 按领域分组建表，外键依赖的父表先创建。
        create_schema_meta(manager).await?;
        create_hosts(manager).await?;
        create_credentials(manager).await?;
        create_history(manager).await?;
        create_snippets(manager).await?;
        create_settings(manager).await?;
        create_extension_tables(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // down 反向删除，先删子表再删父表，避免外键约束失败。
        drop_table(manager, TunnelRules::Table).await?;
        drop_table(manager, SftpBookmarks::Table).await?;
        drop_table(manager, WorkspaceState::Table).await?;
        drop_table(manager, ThemeProfiles::Table).await?;
        drop_table(manager, Settings::Table).await?;
        drop_table(manager, SnippetArguments::Table).await?;
        drop_table(manager, SnippetVariables::Table).await?;
        drop_table(manager, Snippets::Table).await?;
        drop_table(manager, RecentConnections::Table).await?;
        drop_table(manager, CommandHistory::Table).await?;
        drop_table(manager, KnownHosts::Table).await?;
        drop_table(manager, Secrets::Table).await?;
        drop_table(manager, Credentials::Table).await?;
        drop_table(manager, HostJumps::Table).await?;
        drop_table(manager, HostProxy::Table).await?;
        drop_table(manager, HostAuth::Table).await?;
        drop_table(manager, HostTags::Table).await?;
        drop_table(manager, Hosts::Table).await?;
        drop_table(manager, HostGroups::Table).await?;
        drop_table(manager, SchemaMeta::Table).await?;
        Ok(())
    }
}

async fn create_schema_meta(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    // schema_meta 保存核心 schema 版本和以后迁移元数据。
    manager
        .create_table(
            Table::create()
                .table(SchemaMeta::Table)
                .if_not_exists()
                .col(string_pk(SchemaMeta::Key, 128))
                .col(text(SchemaMeta::Value))
                .col(timestamp(SchemaMeta::UpdatedAtUnixSecs))
                .to_owned(),
        )
        .await
}

async fn create_hosts(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    // 主机相关表拆分为分组、主机本体、标签、认证、代理和跳板机。
    manager
        .create_table(
            Table::create()
                .table(HostGroups::Table)
                .if_not_exists()
                .col(string_pk(HostGroups::Id, 36))
                .col(text(HostGroups::Name))
                .col(nullable_string(HostGroups::ParentId, 36))
                .col(integer(HostGroups::SortOrder))
                .col(timestamp(HostGroups::CreatedAtUnixSecs))
                .col(timestamp(HostGroups::UpdatedAtUnixSecs))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_host_groups_parent")
                        .from(HostGroups::Table, HostGroups::ParentId)
                        .to(HostGroups::Table, HostGroups::Id)
                        .on_delete(ForeignKeyAction::SetNull),
                )
                .to_owned(),
        )
        .await?;
    create_index(
        manager,
        HostGroups::Table,
        "idx_host_groups_parent",
        [HostGroups::ParentId],
    )
    .await?;

    manager
        .create_table(
            Table::create()
                .table(Hosts::Table)
                .if_not_exists()
                .col(string_pk(Hosts::Id, 36))
                .col(text(Hosts::Name))
                .col(nullable_string(Hosts::GroupId, 36))
                .col(text(Hosts::IconKey))
                .col(text(Hosts::Address))
                .col(integer(Hosts::Port))
                .col(nullable_text(Hosts::ThemeOverrideToml))
                .col(nullable_text(Hosts::BackgroundOverrideToml))
                .col(integer(Hosts::SortOrder))
                .col(timestamp(Hosts::CreatedAtUnixSecs))
                .col(timestamp(Hosts::UpdatedAtUnixSecs))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_hosts_group")
                        .from(Hosts::Table, Hosts::GroupId)
                        .to(HostGroups::Table, HostGroups::Id)
                        .on_delete(ForeignKeyAction::SetNull),
                )
                .to_owned(),
        )
        .await?;
    create_index(manager, Hosts::Table, "idx_hosts_group", [Hosts::GroupId]).await?;
    create_index(manager, Hosts::Table, "idx_hosts_address", [Hosts::Address]).await?;

    manager
        .create_table(
            Table::create()
                .table(HostTags::Table)
                .if_not_exists()
                .col(string_pk(HostTags::Id, 80))
                .col(string(HostTags::HostId, 36))
                .col(text(HostTags::Tag))
                .col(integer(HostTags::SortOrder))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_host_tags_host")
                        .from(HostTags::Table, HostTags::HostId)
                        .to(Hosts::Table, Hosts::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;
    create_unique_index(
        manager,
        HostTags::Table,
        "idx_host_tags_unique",
        [HostTags::HostId, HostTags::Tag],
    )
    .await?;

    manager
        .create_table(
            Table::create()
                .table(HostAuth::Table)
                .if_not_exists()
                .col(string_pk(HostAuth::HostId, 36))
                .col(text(HostAuth::AuthKind))
                .col(text(HostAuth::Username))
                .col(nullable_text(HostAuth::PasswordSecretRef))
                .col(nullable_text(HostAuth::KeySecretRef))
                .col(nullable_text(HostAuth::PassphraseSecretRef))
                .col(nullable_text(HostAuth::CertificateSecretRef))
                .col(nullable_text(HostAuth::AgentSource))
                .col(nullable_text(HostAuth::AgentPipe))
                .col(nullable_text(HostAuth::KeyHint))
                .col(timestamp(HostAuth::UpdatedAtUnixSecs))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_host_auth_host")
                        .from(HostAuth::Table, HostAuth::HostId)
                        .to(Hosts::Table, Hosts::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;

    manager
        .create_table(
            Table::create()
                .table(HostProxy::Table)
                .if_not_exists()
                .col(string_pk(HostProxy::HostId, 36))
                .col(text(HostProxy::ProxyKind))
                .col(text(HostProxy::ProxyHost))
                .col(integer(HostProxy::ProxyPort))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_host_proxy_host")
                        .from(HostProxy::Table, HostProxy::HostId)
                        .to(Hosts::Table, Hosts::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;

    manager
        .create_table(
            Table::create()
                .table(HostJumps::Table)
                .if_not_exists()
                .col(string_pk(HostJumps::Id, 80))
                .col(string(HostJumps::HostId, 36))
                .col(string(HostJumps::JumpHostId, 36))
                .col(integer(HostJumps::SortOrder))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_host_jumps_host")
                        .from(HostJumps::Table, HostJumps::HostId)
                        .to(Hosts::Table, Hosts::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;
    create_index(
        manager,
        HostJumps::Table,
        "idx_host_jumps_host",
        [HostJumps::HostId],
    )
    .await
}

async fn create_credentials(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Credentials::Table)
                .if_not_exists()
                .col(string_pk(Credentials::Name, 128))
                .col(text(Credentials::Kind))
                .col(nullable_text(Credentials::Username))
                .col(nullable_text(Credentials::SecretRef))
                .col(nullable_text(Credentials::KeyAlgorithm))
                .col(nullable_text(Credentials::KeyAlgorithmRaw))
                .col(nullable_text(Credentials::Fingerprint))
                .col(timestamp(Credentials::CreatedAtUnixSecs))
                .col(timestamp(Credentials::UpdatedAtUnixSecs))
                .to_owned(),
        )
        .await?;

    manager
        .create_table(
            Table::create()
                .table(Secrets::Table)
                .if_not_exists()
                .col(string_pk(Secrets::SecretRef, 256))
                .col(text(Secrets::SecretKind))
                .col(integer(Secrets::EncryptionVersion))
                .col(nullable_text(Secrets::Kdf))
                .col(nullable_text(Secrets::KdfParamsToml))
                .col(nullable_blob(Secrets::Salt))
                .col(nullable_blob(Secrets::Nonce))
                .col(nullable_blob(Secrets::EncryptedPayload))
                .col(nullable_text(Secrets::ExternalStore))
                .col(nullable_text(Secrets::ExternalKey))
                .col(timestamp(Secrets::CreatedAtUnixSecs))
                .col(timestamp(Secrets::UpdatedAtUnixSecs))
                .to_owned(),
        )
        .await?;

    manager
        .create_table(
            Table::create()
                .table(KnownHosts::Table)
                .if_not_exists()
                .col(string_pk(KnownHosts::Id, 200))
                .col(text(KnownHosts::Host))
                .col(integer(KnownHosts::Port))
                .col(text(KnownHosts::KeyAlgorithm))
                .col(nullable_text(KnownHosts::KeyAlgorithmRaw))
                .col(text(KnownHosts::Fingerprint))
                .col(boolean(KnownHosts::Trusted))
                .col(timestamp(KnownHosts::CreatedAtUnixSecs))
                .col(timestamp(KnownHosts::UpdatedAtUnixSecs))
                .to_owned(),
        )
        .await?;
    create_unique_index(
        manager,
        KnownHosts::Table,
        "idx_known_hosts_unique",
        [KnownHosts::Host, KnownHosts::Port],
    )
    .await
}

async fn create_history(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(CommandHistory::Table)
                .if_not_exists()
                .col(string_pk(CommandHistory::Id, 36))
                .col(nullable_string(CommandHistory::HostId, 36))
                .col(text(CommandHistory::Command))
                .col(nullable_text(CommandHistory::WorkingDirectory))
                .col(ColumnDef::new(CommandHistory::ExitCode).integer())
                .col(timestamp(CommandHistory::StartedAtUnixSecs))
                .col(ColumnDef::new(CommandHistory::DurationMs).big_integer())
                .col(timestamp(CommandHistory::CreatedAtUnixSecs))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_command_history_host")
                        .from(CommandHistory::Table, CommandHistory::HostId)
                        .to(Hosts::Table, Hosts::Id)
                        .on_delete(ForeignKeyAction::SetNull),
                )
                .to_owned(),
        )
        .await?;
    create_index(
        manager,
        CommandHistory::Table,
        "idx_command_history_host",
        [CommandHistory::HostId],
    )
    .await?;
    create_index(
        manager,
        CommandHistory::Table,
        "idx_command_history_started",
        [CommandHistory::StartedAtUnixSecs],
    )
    .await?;

    manager
        .create_table(
            Table::create()
                .table(RecentConnections::Table)
                .if_not_exists()
                .col(string_pk(RecentConnections::HostId, 36))
                .col(text(RecentConnections::Label))
                .col(timestamp(RecentConnections::ConnectedAtUnixSecs))
                .col(timestamp(RecentConnections::UpdatedAtUnixSecs))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_recent_connections_host")
                        .from(RecentConnections::Table, RecentConnections::HostId)
                        .to(Hosts::Table, Hosts::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await
}

async fn create_snippets(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Snippets::Table)
                .if_not_exists()
                .col(string_pk(Snippets::Id, 36))
                .col(text(Snippets::Name))
                .col(nullable_text(Snippets::Description))
                .col(text(Snippets::CommandTemplate))
                .col(text(Snippets::ScopeKind))
                .col(nullable_string(Snippets::ScopeTargetId, 36))
                .col(integer(Snippets::SortOrder))
                .col(timestamp(Snippets::CreatedAtUnixSecs))
                .col(timestamp(Snippets::UpdatedAtUnixSecs))
                .to_owned(),
        )
        .await?;
    create_index(
        manager,
        Snippets::Table,
        "idx_snippets_scope",
        [Snippets::ScopeKind],
    )
    .await?;

    manager
        .create_table(
            Table::create()
                .table(SnippetVariables::Table)
                .if_not_exists()
                .col(string_pk(SnippetVariables::Id, 120))
                .col(string(SnippetVariables::SnippetId, 36))
                .col(text(SnippetVariables::Name))
                .col(nullable_text(SnippetVariables::DefaultValue))
                .col(boolean(SnippetVariables::Required))
                .col(integer(SnippetVariables::SortOrder))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_snippet_variables_snippet")
                        .from(SnippetVariables::Table, SnippetVariables::SnippetId)
                        .to(Snippets::Table, Snippets::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;
    create_unique_index(
        manager,
        SnippetVariables::Table,
        "idx_snippet_variables_unique",
        [SnippetVariables::SnippetId, SnippetVariables::Name],
    )
    .await?;

    manager
        .create_table(
            Table::create()
                .table(SnippetArguments::Table)
                .if_not_exists()
                .col(string_pk(SnippetArguments::Id, 120))
                .col(string(SnippetArguments::SnippetId, 36))
                .col(text(SnippetArguments::Name))
                .col(text(SnippetArguments::Value))
                .col(integer(SnippetArguments::SortOrder))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_snippet_arguments_snippet")
                        .from(SnippetArguments::Table, SnippetArguments::SnippetId)
                        .to(Snippets::Table, Snippets::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;
    create_unique_index(
        manager,
        SnippetArguments::Table,
        "idx_snippet_arguments_unique",
        [SnippetArguments::SnippetId, SnippetArguments::Name],
    )
    .await
}

async fn create_settings(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Settings::Table)
                .if_not_exists()
                .col(string_pk(Settings::Key, 128))
                .col(text(Settings::ValueToml))
                .col(timestamp(Settings::UpdatedAtUnixSecs))
                .to_owned(),
        )
        .await?;

    manager
        .create_table(
            Table::create()
                .table(ThemeProfiles::Table)
                .if_not_exists()
                .col(string_pk(ThemeProfiles::Name, 128))
                .col(text(ThemeProfiles::ProfileToml))
                .col(boolean(ThemeProfiles::Builtin))
                .col(timestamp(ThemeProfiles::CreatedAtUnixSecs))
                .col(timestamp(ThemeProfiles::UpdatedAtUnixSecs))
                .to_owned(),
        )
        .await?;

    manager
        .create_table(
            Table::create()
                .table(WorkspaceState::Table)
                .if_not_exists()
                .col(string_pk(WorkspaceState::WorkspaceKey, 128))
                .col(text(WorkspaceState::StateToml))
                .col(timestamp(WorkspaceState::UpdatedAtUnixSecs))
                .to_owned(),
        )
        .await
}

async fn create_extension_tables(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(SftpBookmarks::Table)
                .if_not_exists()
                .col(string_pk(SftpBookmarks::Id, 220))
                .col(string(SftpBookmarks::HostId, 36))
                .col(text(SftpBookmarks::Label))
                .col(text(SftpBookmarks::RemotePath))
                .col(integer(SftpBookmarks::SortOrder))
                .col(timestamp(SftpBookmarks::CreatedAtUnixSecs))
                .col(timestamp(SftpBookmarks::UpdatedAtUnixSecs))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_sftp_bookmarks_host")
                        .from(SftpBookmarks::Table, SftpBookmarks::HostId)
                        .to(Hosts::Table, Hosts::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;
    create_unique_index(
        manager,
        SftpBookmarks::Table,
        "idx_sftp_bookmarks_unique",
        [SftpBookmarks::HostId, SftpBookmarks::RemotePath],
    )
    .await?;

    manager
        .create_table(
            Table::create()
                .table(TunnelRules::Table)
                .if_not_exists()
                .col(string_pk(TunnelRules::Name, 128))
                .col(text(TunnelRules::Kind))
                .col(text(TunnelRules::BindHost))
                .col(integer(TunnelRules::BindPort))
                .col(text(TunnelRules::TargetHost))
                .col(integer(TunnelRules::TargetPort))
                .col(boolean(TunnelRules::AutoStart))
                .col(integer(TunnelRules::SortOrder))
                .col(timestamp(TunnelRules::CreatedAtUnixSecs))
                .col(timestamp(TunnelRules::UpdatedAtUnixSecs))
                .to_owned(),
        )
        .await
}

fn string_pk<T>(name: T, len: u32) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.string_len(len).not_null().primary_key();
    column
}

fn string<T>(name: T, len: u32) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.string_len(len).not_null();
    column
}

fn nullable_string<T>(name: T, len: u32) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.string_len(len);
    column
}

fn text<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.text().not_null();
    column
}

fn nullable_text<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.text();
    column
}

fn integer<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.integer().not_null();
    column
}

fn boolean<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.boolean().not_null();
    column
}

fn timestamp<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.big_integer().not_null();
    column
}

fn nullable_blob<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.binary();
    column
}

async fn create_index<I, C>(
    manager: &SchemaManager<'_>,
    table: I,
    name: &str,
    columns: C,
) -> Result<(), DbErr>
where
    I: IntoIden,
    C: IntoIterator,
    C::Item: IntoIden,
{
    let mut statement = Index::create();
    statement.name(name).table(table).if_not_exists();
    for column in columns {
        statement.col(column);
    }
    manager.create_index(statement.to_owned()).await
}

async fn create_unique_index<I, C>(
    manager: &SchemaManager<'_>,
    table: I,
    name: &str,
    columns: C,
) -> Result<(), DbErr>
where
    I: IntoIden,
    C: IntoIterator,
    C::Item: IntoIden,
{
    let mut statement = Index::create();
    statement.name(name).table(table).unique().if_not_exists();
    for column in columns {
        statement.col(column);
    }
    manager.create_index(statement.to_owned()).await
}

async fn drop_table<I>(manager: &SchemaManager<'_>, table: I) -> Result<(), DbErr>
where
    I: IntoIden,
{
    manager
        .drop_table(Table::drop().table(table).if_exists().to_owned())
        .await
}

#[derive(DeriveIden)]
enum SchemaMeta {
    Table,
    Key,
    Value,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
enum HostGroups {
    Table,
    Id,
    Name,
    ParentId,
    SortOrder,
    CreatedAtUnixSecs,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
enum Hosts {
    Table,
    Id,
    Name,
    GroupId,
    IconKey,
    Address,
    Port,
    ThemeOverrideToml,
    BackgroundOverrideToml,
    SortOrder,
    CreatedAtUnixSecs,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
enum HostTags {
    Table,
    Id,
    HostId,
    Tag,
    SortOrder,
}

#[derive(DeriveIden)]
enum HostAuth {
    Table,
    HostId,
    AuthKind,
    Username,
    PasswordSecretRef,
    KeySecretRef,
    PassphraseSecretRef,
    CertificateSecretRef,
    AgentSource,
    AgentPipe,
    KeyHint,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
enum HostProxy {
    Table,
    HostId,
    ProxyKind,
    ProxyHost,
    ProxyPort,
}

#[derive(DeriveIden)]
enum HostJumps {
    Table,
    Id,
    HostId,
    JumpHostId,
    SortOrder,
}

#[derive(DeriveIden)]
enum Credentials {
    Table,
    Name,
    Kind,
    Username,
    SecretRef,
    KeyAlgorithm,
    KeyAlgorithmRaw,
    Fingerprint,
    CreatedAtUnixSecs,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
enum Secrets {
    Table,
    SecretRef,
    SecretKind,
    EncryptionVersion,
    Kdf,
    KdfParamsToml,
    Salt,
    Nonce,
    EncryptedPayload,
    ExternalStore,
    ExternalKey,
    CreatedAtUnixSecs,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
enum KnownHosts {
    Table,
    Id,
    Host,
    Port,
    KeyAlgorithm,
    KeyAlgorithmRaw,
    Fingerprint,
    Trusted,
    CreatedAtUnixSecs,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
enum CommandHistory {
    Table,
    Id,
    HostId,
    Command,
    WorkingDirectory,
    ExitCode,
    StartedAtUnixSecs,
    DurationMs,
    CreatedAtUnixSecs,
}

#[derive(DeriveIden)]
enum RecentConnections {
    Table,
    HostId,
    Label,
    ConnectedAtUnixSecs,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
enum Snippets {
    Table,
    Id,
    Name,
    Description,
    CommandTemplate,
    ScopeKind,
    ScopeTargetId,
    SortOrder,
    CreatedAtUnixSecs,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
enum SnippetVariables {
    Table,
    Id,
    SnippetId,
    Name,
    DefaultValue,
    Required,
    SortOrder,
}

#[derive(DeriveIden)]
enum SnippetArguments {
    Table,
    Id,
    SnippetId,
    Name,
    Value,
    SortOrder,
}

#[derive(DeriveIden)]
enum Settings {
    Table,
    Key,
    ValueToml,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
enum ThemeProfiles {
    Table,
    Name,
    ProfileToml,
    Builtin,
    CreatedAtUnixSecs,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
enum WorkspaceState {
    Table,
    WorkspaceKey,
    StateToml,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
enum SftpBookmarks {
    Table,
    Id,
    HostId,
    Label,
    RemotePath,
    SortOrder,
    CreatedAtUnixSecs,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
enum TunnelRules {
    Table,
    Name,
    Kind,
    BindHost,
    BindPort,
    TargetHost,
    TargetPort,
    AutoStart,
    SortOrder,
    CreatedAtUnixSecs,
    UpdatedAtUnixSecs,
}
