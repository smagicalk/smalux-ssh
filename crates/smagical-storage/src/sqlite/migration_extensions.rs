use sea_orm_migration::prelude::*;

use super::migration_common::*;
use super::migration_hosts::Hosts;

pub(super) async fn create_extension_tables(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
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
                .col(boolean_with_default(TunnelRules::ExitOnFailure, false))
                .col(integer(TunnelRules::SortOrder))
                .col(timestamp(TunnelRules::CreatedAtUnixSecs))
                .col(timestamp(TunnelRules::UpdatedAtUnixSecs))
                .to_owned(),
        )
        .await
}

#[derive(DeriveIden)]
pub(super) enum SftpBookmarks {
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
pub(super) enum TunnelRules {
    Table,
    Name,
    Kind,
    BindHost,
    BindPort,
    TargetHost,
    TargetPort,
    AutoStart,
    ExitOnFailure,
    SortOrder,
    CreatedAtUnixSecs,
    UpdatedAtUnixSecs,
}
