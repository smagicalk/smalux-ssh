use sea_orm_migration::prelude::*;

use super::migration_common::*;

pub(super) async fn create_snippets(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
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
                .col(nullable_string(Snippets::GroupId, 36))
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
    create_index(
        manager,
        Snippets::Table,
        "idx_snippets_group",
        [Snippets::GroupId],
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
                .col(nullable_string(SnippetArguments::ImplementationId, 36))
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
    .await?;
    create_index(
        manager,
        SnippetArguments::Table,
        "idx_snippet_arguments_implementation",
        [SnippetArguments::ImplementationId],
    )
    .await
}

pub(super) async fn create_snippet_groups(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(SnippetGroups::Table)
                .if_not_exists()
                .col(string_pk(SnippetGroups::Id, 36))
                .col(text(SnippetGroups::Name))
                .col(nullable_string(SnippetGroups::ParentId, 36))
                .col(integer(SnippetGroups::SortOrder))
                .col(timestamp(SnippetGroups::CreatedAtUnixSecs))
                .col(timestamp(SnippetGroups::UpdatedAtUnixSecs))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_snippet_groups_parent")
                        .from(SnippetGroups::Table, SnippetGroups::ParentId)
                        .to(SnippetGroups::Table, SnippetGroups::Id)
                        .on_delete(ForeignKeyAction::SetNull),
                )
                .to_owned(),
        )
        .await?;
    create_index(
        manager,
        SnippetGroups::Table,
        "idx_snippet_groups_parent",
        [SnippetGroups::ParentId],
    )
    .await
}

pub(super) async fn create_snippet_implementations(
    manager: &SchemaManager<'_>,
) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(SnippetImplementations::Table)
                .if_not_exists()
                .col(string_pk(SnippetImplementations::Id, 36))
                .col(string(SnippetImplementations::SnippetId, 36))
                .col(text(SnippetImplementations::Name))
                .col(text(SnippetImplementations::Shell))
                .col(nullable_text(SnippetImplementations::ShellCustom))
                .col(text(SnippetImplementations::CommandTemplate))
                .col(nullable_text(SnippetImplementations::Notes))
                .col(integer(SnippetImplementations::SortOrder))
                .col(timestamp(SnippetImplementations::CreatedAtUnixSecs))
                .col(timestamp(SnippetImplementations::UpdatedAtUnixSecs))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_snippet_implementations_snippet")
                        .from(
                            SnippetImplementations::Table,
                            SnippetImplementations::SnippetId,
                        )
                        .to(Snippets::Table, Snippets::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;
    create_index(
        manager,
        SnippetImplementations::Table,
        "idx_snippet_implementations_snippet",
        [SnippetImplementations::SnippetId],
    )
    .await
}

pub(super) async fn create_snippet_support_targets(
    manager: &SchemaManager<'_>,
) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(SnippetSupportTargets::Table)
                .if_not_exists()
                .col(string_pk(SnippetSupportTargets::Id, 36))
                .col(string(SnippetSupportTargets::SnippetId, 36))
                .col(text(SnippetSupportTargets::TargetKey))
                .col(text(SnippetSupportTargets::DisplayName))
                .col(string(SnippetSupportTargets::ImplementationId, 36))
                .col(integer(SnippetSupportTargets::SortOrder))
                .col(timestamp(SnippetSupportTargets::CreatedAtUnixSecs))
                .col(timestamp(SnippetSupportTargets::UpdatedAtUnixSecs))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_snippet_support_targets_snippet")
                        .from(
                            SnippetSupportTargets::Table,
                            SnippetSupportTargets::SnippetId,
                        )
                        .to(Snippets::Table, Snippets::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_snippet_support_targets_implementation")
                        .from(
                            SnippetSupportTargets::Table,
                            SnippetSupportTargets::ImplementationId,
                        )
                        .to(SnippetImplementations::Table, SnippetImplementations::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;
    create_unique_index(
        manager,
        SnippetSupportTargets::Table,
        "idx_snippet_support_targets_unique",
        [
            SnippetSupportTargets::SnippetId,
            SnippetSupportTargets::TargetKey,
        ],
    )
    .await
}

#[derive(DeriveIden)]
pub(super) enum Snippets {
    Table,
    Id,
    Name,
    Description,
    CommandTemplate,
    ScopeKind,
    ScopeTargetId,
    GroupId,
    SortOrder,
    CreatedAtUnixSecs,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
pub(super) enum SnippetGroups {
    Table,
    Id,
    Name,
    ParentId,
    SortOrder,
    CreatedAtUnixSecs,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
pub(super) enum SnippetImplementations {
    Table,
    Id,
    SnippetId,
    Name,
    Shell,
    ShellCustom,
    CommandTemplate,
    Notes,
    SortOrder,
    CreatedAtUnixSecs,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
pub(super) enum SnippetSupportTargets {
    Table,
    Id,
    SnippetId,
    TargetKey,
    DisplayName,
    ImplementationId,
    SortOrder,
    CreatedAtUnixSecs,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
pub(super) enum SnippetVariables {
    Table,
    Id,
    SnippetId,
    Name,
    DefaultValue,
    Required,
    SortOrder,
}

#[derive(DeriveIden)]
pub(super) enum SnippetArguments {
    Table,
    Id,
    SnippetId,
    ImplementationId,
    Name,
    Value,
    SortOrder,
}
