//! SQLite schema migrations。
//!
//! 迁移只描述物理表结构，不包含业务转换逻辑。业务结构和表结构之间的映射放在
//! `mapper.rs`，这样后续拆表或合表时可以独立演进。

use sea_orm_migration::prelude::*;

use super::migration_common::*;
use super::migration_credentials::{
    CredentialGroups, CredentialInspections, Credentials, KnownHosts, Secrets,
    create_credential_groups, create_credential_inspections, create_credentials,
};
use super::migration_extensions::{SftpBookmarks, TunnelRules, create_extension_tables};
use super::migration_history::{CommandHistory, RecentConnections, create_history};
use super::migration_hosts::{
    ForwardAssets, HostAuth, HostGroups, HostJumps, HostNetworkForwards, HostNetworkJumpChains,
    HostNetworkProxies, HostProxy, HostTags, Hosts, JumpChainAssets, JumpChainSteps, ProxyAssets,
    create_hosts,
};
use super::migration_settings::{Settings, ThemeProfiles, WorkspaceState, create_settings};
use super::migration_snippets::{
    SnippetArguments, SnippetGroups, SnippetImplementations, SnippetSupportTargets,
    SnippetVariables, Snippets, create_snippet_groups, create_snippet_implementations,
    create_snippet_support_targets, create_snippets,
};

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        // 预览数据会清空旧库；新库只需要一个完整当前 schema 初始化迁移。
        vec![Box::new(M20260528000000CreateCoreStorageSchema)]
    }
}

struct M20260528000000CreateCoreStorageSchema;

impl MigrationName for M20260528000000CreateCoreStorageSchema {
    fn name(&self) -> &str {
        "m20260528000000_create_core_storage_schema"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for M20260528000000CreateCoreStorageSchema {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 按领域分组建表，外键依赖的父表先创建。
        create_schema_meta(manager).await?;
        create_hosts(manager).await?;
        create_credentials(manager).await?;
        create_credential_groups(manager).await?;
        create_credential_inspections(manager).await?;
        create_history(manager).await?;
        create_snippets(manager).await?;
        create_snippet_groups(manager).await?;
        create_snippet_implementations(manager).await?;
        create_snippet_support_targets(manager).await?;
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
        drop_table(manager, SnippetSupportTargets::Table).await?;
        drop_table(manager, SnippetImplementations::Table).await?;
        drop_table(manager, SnippetArguments::Table).await?;
        drop_table(manager, SnippetVariables::Table).await?;
        drop_table(manager, Snippets::Table).await?;
        drop_table(manager, SnippetGroups::Table).await?;
        drop_table(manager, RecentConnections::Table).await?;
        drop_table(manager, CommandHistory::Table).await?;
        drop_table(manager, KnownHosts::Table).await?;
        drop_table(manager, CredentialInspections::Table).await?;
        drop_table(manager, Secrets::Table).await?;
        drop_table(manager, Credentials::Table).await?;
        drop_table(manager, CredentialGroups::Table).await?;
        drop_table(manager, HostNetworkForwards::Table).await?;
        drop_table(manager, HostNetworkJumpChains::Table).await?;
        drop_table(manager, HostNetworkProxies::Table).await?;
        drop_table(manager, HostJumps::Table).await?;
        drop_table(manager, HostProxy::Table).await?;
        drop_table(manager, JumpChainSteps::Table).await?;
        drop_table(manager, JumpChainAssets::Table).await?;
        drop_table(manager, ProxyAssets::Table).await?;
        drop_table(manager, ForwardAssets::Table).await?;
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

#[derive(DeriveIden)]
enum SchemaMeta {
    Table,
    Key,
    Value,
    UpdatedAtUnixSecs,
}
