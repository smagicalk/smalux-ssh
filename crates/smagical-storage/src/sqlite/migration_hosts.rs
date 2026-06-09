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
    HostId,
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
