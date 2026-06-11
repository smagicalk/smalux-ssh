use sea_orm_migration::prelude::*;

use super::migration_common::*;

pub(super) async fn create_hosts(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
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
                .col(string_pk(HostProxy::Id, 80))
                .col(string(HostProxy::HostId, 36))
                .col(integer(HostProxy::SortOrder))
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
    create_index(
        manager,
        HostProxy::Table,
        "idx_host_proxy_host",
        [HostProxy::HostId],
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
    .await?;

    manager
        .create_table(
            Table::create()
                .table(ProxyAssets::Table)
                .if_not_exists()
                .col(string_pk(ProxyAssets::Id, 36))
                .col(text(ProxyAssets::Name))
                .col(text_with_default(ProxyAssets::TagsToml, "items = []\n"))
                .col(text(ProxyAssets::ProxyKind))
                .col(text(ProxyAssets::ProxyHost))
                .col(integer(ProxyAssets::ProxyPort))
                .col(integer(ProxyAssets::SortOrder))
                .to_owned(),
        )
        .await?;

    manager
        .create_table(
            Table::create()
                .table(JumpChainAssets::Table)
                .if_not_exists()
                .col(string_pk(JumpChainAssets::Id, 36))
                .col(text(JumpChainAssets::Name))
                .col(integer(JumpChainAssets::SortOrder))
                .to_owned(),
        )
        .await?;

    manager
        .create_table(
            Table::create()
                .table(JumpChainSteps::Table)
                .if_not_exists()
                .col(string_pk(JumpChainSteps::Id, 80))
                .col(string(JumpChainSteps::ChainId, 36))
                .col(string(JumpChainSteps::JumpHostId, 36))
                .col(integer(JumpChainSteps::SortOrder))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_jump_chain_steps_chain")
                        .from(JumpChainSteps::Table, JumpChainSteps::ChainId)
                        .to(JumpChainAssets::Table, JumpChainAssets::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_jump_chain_steps_host")
                        .from(JumpChainSteps::Table, JumpChainSteps::JumpHostId)
                        .to(Hosts::Table, Hosts::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;
    create_index(
        manager,
        JumpChainSteps::Table,
        "idx_jump_chain_steps_chain",
        [JumpChainSteps::ChainId],
    )
    .await?;

    manager
        .create_table(
            Table::create()
                .table(ForwardAssets::Table)
                .if_not_exists()
                .col(string_pk(ForwardAssets::Id, 36))
                .col(text(ForwardAssets::Name))
                .col(text_with_default(ForwardAssets::TagsToml, "items = []\n"))
                .col(text(ForwardAssets::Kind))
                .col(text(ForwardAssets::BindHost))
                .col(integer(ForwardAssets::BindPort))
                .col(text(ForwardAssets::TargetHost))
                .col(integer(ForwardAssets::TargetPort))
                .col(boolean(ForwardAssets::AutoStart))
                .col(integer(ForwardAssets::SortOrder))
                .to_owned(),
        )
        .await?;

    manager
        .create_table(
            Table::create()
                .table(HostNetworkProxies::Table)
                .if_not_exists()
                .col(string_pk(HostNetworkProxies::Id, 80))
                .col(string(HostNetworkProxies::HostId, 36))
                .col(string(HostNetworkProxies::ProxyId, 36))
                .col(integer(HostNetworkProxies::SortOrder))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_host_network_proxies_host")
                        .from(HostNetworkProxies::Table, HostNetworkProxies::HostId)
                        .to(Hosts::Table, Hosts::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_host_network_proxies_proxy")
                        .from(HostNetworkProxies::Table, HostNetworkProxies::ProxyId)
                        .to(ProxyAssets::Table, ProxyAssets::Id)
                        .on_delete(ForeignKeyAction::Restrict),
                )
                .to_owned(),
        )
        .await?;
    create_index(
        manager,
        HostNetworkProxies::Table,
        "idx_host_network_proxies_host",
        [HostNetworkProxies::HostId],
    )
    .await?;

    manager
        .create_table(
            Table::create()
                .table(HostNetworkJumpChains::Table)
                .if_not_exists()
                .col(string_pk(HostNetworkJumpChains::Id, 80))
                .col(string(HostNetworkJumpChains::HostId, 36))
                .col(string(HostNetworkJumpChains::ChainId, 36))
                .col(integer(HostNetworkJumpChains::SortOrder))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_host_network_jump_chains_host")
                        .from(HostNetworkJumpChains::Table, HostNetworkJumpChains::HostId)
                        .to(Hosts::Table, Hosts::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_host_network_jump_chains_chain")
                        .from(HostNetworkJumpChains::Table, HostNetworkJumpChains::ChainId)
                        .to(JumpChainAssets::Table, JumpChainAssets::Id)
                        .on_delete(ForeignKeyAction::Restrict),
                )
                .to_owned(),
        )
        .await?;
    create_index(
        manager,
        HostNetworkJumpChains::Table,
        "idx_host_network_jump_chains_host",
        [HostNetworkJumpChains::HostId],
    )
    .await?;

    manager
        .create_table(
            Table::create()
                .table(HostNetworkForwards::Table)
                .if_not_exists()
                .col(string_pk(HostNetworkForwards::Id, 80))
                .col(string(HostNetworkForwards::HostId, 36))
                .col(string(HostNetworkForwards::ForwardId, 36))
                .col(integer(HostNetworkForwards::SortOrder))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_host_network_forwards_host")
                        .from(HostNetworkForwards::Table, HostNetworkForwards::HostId)
                        .to(Hosts::Table, Hosts::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_host_network_forwards_forward")
                        .from(HostNetworkForwards::Table, HostNetworkForwards::ForwardId)
                        .to(ForwardAssets::Table, ForwardAssets::Id)
                        .on_delete(ForeignKeyAction::Restrict),
                )
                .to_owned(),
        )
        .await?;
    create_index(
        manager,
        HostNetworkForwards::Table,
        "idx_host_network_forwards_host",
        [HostNetworkForwards::HostId],
    )
    .await
}

#[derive(DeriveIden)]
pub(super) enum HostGroups {
    Table,
    Id,
    Name,
    ParentId,
    SortOrder,
    CreatedAtUnixSecs,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
pub(super) enum Hosts {
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
pub(super) enum HostTags {
    Table,
    Id,
    HostId,
    Tag,
    SortOrder,
}

#[derive(DeriveIden)]
pub(super) enum HostAuth {
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
pub(super) enum HostProxy {
    Table,
    Id,
    HostId,
    SortOrder,
    ProxyKind,
    ProxyHost,
    ProxyPort,
}

#[derive(DeriveIden)]
pub(super) enum HostJumps {
    Table,
    Id,
    HostId,
    JumpHostId,
    SortOrder,
}

#[derive(DeriveIden)]
pub(super) enum ProxyAssets {
    Table,
    Id,
    Name,
    TagsToml,
    ProxyKind,
    ProxyHost,
    ProxyPort,
    SortOrder,
}

#[derive(DeriveIden)]
pub(super) enum JumpChainAssets {
    Table,
    Id,
    Name,
    SortOrder,
}

#[derive(DeriveIden)]
pub(super) enum JumpChainSteps {
    Table,
    Id,
    ChainId,
    JumpHostId,
    SortOrder,
}

#[derive(DeriveIden)]
pub(super) enum ForwardAssets {
    Table,
    Id,
    Name,
    TagsToml,
    Kind,
    BindHost,
    BindPort,
    TargetHost,
    TargetPort,
    AutoStart,
    SortOrder,
}

#[derive(DeriveIden)]
pub(super) enum HostNetworkProxies {
    Table,
    Id,
    HostId,
    ProxyId,
    SortOrder,
}

#[derive(DeriveIden)]
pub(super) enum HostNetworkJumpChains {
    Table,
    Id,
    HostId,
    ChainId,
    SortOrder,
}

#[derive(DeriveIden)]
pub(super) enum HostNetworkForwards {
    Table,
    Id,
    HostId,
    ForwardId,
    SortOrder,
}
