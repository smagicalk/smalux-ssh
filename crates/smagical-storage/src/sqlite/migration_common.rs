use sea_orm_migration::prelude::*;

pub(super) fn string_pk<T>(name: T, len: u32) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.string_len(len).not_null().primary_key();
    column
}

pub(super) fn string<T>(name: T, len: u32) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.string_len(len).not_null();
    column
}

pub(super) fn nullable_string<T>(name: T, len: u32) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.string_len(len);
    column
}

pub(super) fn text<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.text().not_null();
    column
}

pub(super) fn nullable_text<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.text();
    column
}

pub(super) fn text_with_default<T>(name: T, default_value: &str) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.text().not_null().default(default_value);
    column
}

pub(super) fn integer<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.integer().not_null();
    column
}

pub(super) fn nullable_integer<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.integer();
    column
}

pub(super) fn boolean<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.boolean().not_null();
    column
}

pub(super) fn boolean_with_default<T>(name: T, default_value: bool) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.boolean().not_null().default(default_value);
    column
}

pub(super) fn nullable_boolean<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.boolean();
    column
}

pub(super) fn timestamp<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.big_integer().not_null();
    column
}

pub(super) fn nullable_timestamp<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.big_integer();
    column
}

pub(super) fn nullable_blob<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    let mut column = ColumnDef::new(name);
    column.binary();
    column
}

pub(super) async fn create_index<I, C>(
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

pub(super) async fn create_unique_index<I, C>(
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

pub(super) async fn drop_table<I>(manager: &SchemaManager<'_>, table: I) -> Result<(), DbErr>
where
    I: IntoIden,
{
    manager
        .drop_table(Table::drop().table(table).if_exists().to_owned())
        .await
}
