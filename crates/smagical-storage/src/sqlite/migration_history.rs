use sea_orm_migration::prelude::*;

use super::migration_common::*;
use super::migration_hosts::Hosts;

pub(super) async fn create_history(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
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

#[derive(DeriveIden)]
pub(super) enum CommandHistory {
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
pub(super) enum RecentConnections {
    Table,
    HostId,
    Label,
    ConnectedAtUnixSecs,
    UpdatedAtUnixSecs,
}
