use sea_orm_migration::prelude::*;

use super::migration_common::*;

pub(super) async fn create_settings(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
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

#[derive(DeriveIden)]
pub(super) enum Settings {
    Table,
    Key,
    ValueToml,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
pub(super) enum ThemeProfiles {
    Table,
    Name,
    ProfileToml,
    Builtin,
    CreatedAtUnixSecs,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
pub(super) enum WorkspaceState {
    Table,
    WorkspaceKey,
    StateToml,
    UpdatedAtUnixSecs,
}
