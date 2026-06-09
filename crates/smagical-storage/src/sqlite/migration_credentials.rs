use sea_orm_migration::prelude::*;

use super::migration_common::*;

pub(super) async fn create_credentials(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Credentials::Table)
                .if_not_exists()
                .col(string_pk(Credentials::Name, 128))
                .col(nullable_string(Credentials::Id, 36))
                .col(text(Credentials::Kind))
                .col(nullable_string(Credentials::GroupId, 36))
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
    create_unique_index(
        manager,
        Credentials::Table,
        "idx_credentials_id",
        [Credentials::Id],
    )
    .await?;
    create_index(
        manager,
        Credentials::Table,
        "idx_credentials_group",
        [Credentials::GroupId],
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

pub(super) async fn create_credential_inspections(
    manager: &SchemaManager<'_>,
) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(CredentialInspections::Table)
                .if_not_exists()
                .col(string_pk(CredentialInspections::CredentialId, 36))
                .col(text(CredentialInspections::Kind))
                .col(text(CredentialInspections::PayloadHash))
                .col(integer(CredentialInspections::ParserVersion))
                .col(nullable_text(CredentialInspections::ParseError))
                .col(nullable_text(CredentialInspections::KeyAlgorithm))
                .col(nullable_text(CredentialInspections::KeyAlgorithmRaw))
                .col(nullable_text(CredentialInspections::Fingerprint))
                .col(nullable_text(CredentialInspections::PublicKey))
                .col(nullable_text(CredentialInspections::Comment))
                .col(nullable_boolean(CredentialInspections::Encrypted))
                .col(nullable_integer(CredentialInspections::PasswordLength))
                .col(nullable_text(CredentialInspections::CertType))
                .col(nullable_text(CredentialInspections::Serial))
                .col(nullable_text(CredentialInspections::KeyId))
                .col(nullable_text(CredentialInspections::PrincipalsText))
                .col(nullable_timestamp(
                    CredentialInspections::ValidAfterUnixSecs,
                ))
                .col(nullable_timestamp(
                    CredentialInspections::ValidBeforeUnixSecs,
                ))
                .col(nullable_text(CredentialInspections::CaFingerprint))
                .col(nullable_text(CredentialInspections::SubjectFingerprint))
                .col(nullable_text(CredentialInspections::CriticalOptionsJson))
                .col(nullable_text(CredentialInspections::ExtensionsJson))
                .col(timestamp(CredentialInspections::CreatedAtUnixSecs))
                .col(timestamp(CredentialInspections::UpdatedAtUnixSecs))
                .to_owned(),
        )
        .await
}

pub(super) async fn create_credential_groups(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(CredentialGroups::Table)
                .if_not_exists()
                .col(string_pk(CredentialGroups::Id, 36))
                .col(text(CredentialGroups::Name))
                .col(text_with_default(
                    CredentialGroups::Kind,
                    credential_kind_private_key(),
                ))
                .col(nullable_string(CredentialGroups::ParentId, 36))
                .col(integer(CredentialGroups::SortOrder))
                .col(timestamp(CredentialGroups::CreatedAtUnixSecs))
                .col(timestamp(CredentialGroups::UpdatedAtUnixSecs))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_credential_groups_parent")
                        .from(CredentialGroups::Table, CredentialGroups::ParentId)
                        .to(CredentialGroups::Table, CredentialGroups::Id)
                        .on_delete(ForeignKeyAction::SetNull),
                )
                .to_owned(),
        )
        .await?;
    create_index(
        manager,
        CredentialGroups::Table,
        "idx_credential_groups_parent",
        [CredentialGroups::ParentId],
    )
    .await
}

pub(super) fn credential_kind_private_key() -> &'static str {
    "PrivateKey"
}

#[derive(DeriveIden)]
pub(super) enum Credentials {
    Table,
    Id,
    Name,
    Kind,
    GroupId,
    Username,
    SecretRef,
    KeyAlgorithm,
    KeyAlgorithmRaw,
    Fingerprint,
    CreatedAtUnixSecs,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
pub(super) enum CredentialInspections {
    Table,
    CredentialId,
    Kind,
    PayloadHash,
    ParserVersion,
    ParseError,
    KeyAlgorithm,
    KeyAlgorithmRaw,
    Fingerprint,
    PublicKey,
    Comment,
    Encrypted,
    PasswordLength,
    CertType,
    Serial,
    KeyId,
    PrincipalsText,
    ValidAfterUnixSecs,
    ValidBeforeUnixSecs,
    CaFingerprint,
    SubjectFingerprint,
    CriticalOptionsJson,
    ExtensionsJson,
    CreatedAtUnixSecs,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
pub(super) enum CredentialGroups {
    Table,
    Id,
    Name,
    Kind,
    ParentId,
    SortOrder,
    CreatedAtUnixSecs,
    UpdatedAtUnixSecs,
}

#[derive(DeriveIden)]
pub(super) enum Secrets {
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
pub(super) enum KnownHosts {
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
