//! Umwandlung zwischen Fachwerten und SQLite-Spalten sowie kleine SQL-Helfer.

use chrono::DateTime;
use serde::Serialize;
use serde::de::DeserializeOwned;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqliteConnection};
use uuid::Uuid;
use vereinsakte_core::ids::MemberId;
use vereinsakte_core::money::Money;
use vereinsakte_core::time::Timestamp;

use crate::db::StorageError;

/// Ein Spaltenwert zum Schreiben.
#[derive(Clone, Debug)]
pub(crate) enum Val {
    Int(Option<i64>),
    Real(Option<f64>),
    Text(Option<String>),
}

pub(crate) fn int(value: impl Into<i64>) -> Val {
    Val::Int(Some(value.into()))
}

pub(crate) fn opt_int<T: Into<i64>>(value: Option<T>) -> Val {
    Val::Int(value.map(Into::into))
}

pub(crate) fn boolean(value: bool) -> Val {
    Val::Int(Some(i64::from(value)))
}

pub(crate) fn real(value: f64) -> Val {
    Val::Real(Some(value))
}

pub(crate) fn opt_real(value: Option<f64>) -> Val {
    Val::Real(value)
}

pub(crate) fn text(value: impl ToString) -> Val {
    Val::Text(Some(value.to_string()))
}

pub(crate) fn opt_text<T: ToString>(value: Option<T>) -> Val {
    Val::Text(value.map(|v| v.to_string()))
}

pub(crate) fn ts(value: Timestamp) -> Val {
    Val::Text(Some(value.to_rfc3339()))
}

pub(crate) fn opt_ts(value: Option<Timestamp>) -> Val {
    Val::Text(value.map(|t| t.to_rfc3339()))
}

pub(crate) fn member(id: MemberId) -> Val {
    int(id.get())
}

pub(crate) fn opt_member(id: Option<MemberId>) -> Val {
    opt_int(id.map(MemberId::get))
}

pub(crate) fn cents(amount: Money) -> i64 {
    // Money ist auf genau zwei Nachkommastellen normiert: Mantisse = Cent.
    i64::try_from(amount.euros().mantissa()).expect("Betrag passt in i64")
}

pub(crate) fn money(amount: Money) -> Val {
    int(cents(amount))
}

pub(crate) fn opt_money(amount: Option<Money>) -> Val {
    Val::Int(amount.map(cents))
}

/// Auswahlwert in seiner JSON-Schreibweise (snake_case), z. B. `"child_of_member"`.
pub(crate) fn choice<T: Serialize>(value: &T) -> Val {
    let json = serde_json::to_value(value).expect("Auswahlwert serialisierbar");
    Val::Text(Some(
        json.as_str()
            .expect("Auswahlwert ist eine Zeichenkette")
            .to_string(),
    ))
}

// --- Lesen -----------------------------------------------------------------

fn corrupt(column: &str, error: impl std::fmt::Display) -> StorageError {
    StorageError::Corrupt(format!("{column}: {error}"))
}

pub(crate) fn get<'r, T>(row: &'r SqliteRow, column: &str) -> Result<T, StorageError>
where
    T: sqlx::Decode<'r, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite>,
{
    row.try_get(column).map_err(|e| corrupt(column, e))
}

pub(crate) fn get_bool(row: &SqliteRow, column: &str) -> Result<bool, StorageError> {
    Ok(get::<i64>(row, column)? != 0)
}

pub(crate) fn parse_ts(column: &str, value: &str) -> Result<Timestamp, StorageError> {
    DateTime::parse_from_rfc3339(value).map_err(|e| corrupt(column, e))
}

pub(crate) fn get_ts(row: &SqliteRow, column: &str) -> Result<Timestamp, StorageError> {
    parse_ts(column, &get::<String>(row, column)?)
}

pub(crate) fn get_opt_ts(row: &SqliteRow, column: &str) -> Result<Option<Timestamp>, StorageError> {
    get::<Option<String>>(row, column)?
        .map(|v| parse_ts(column, &v))
        .transpose()
}

fn to_member(column: &str, value: i64) -> Result<MemberId, StorageError> {
    u32::try_from(value)
        .ok()
        .and_then(MemberId::new)
        .ok_or_else(|| corrupt(column, format!("ungültige Mitgliedsnummer {value}")))
}

pub(crate) fn get_member(row: &SqliteRow, column: &str) -> Result<MemberId, StorageError> {
    to_member(column, get(row, column)?)
}

pub(crate) fn get_opt_member(
    row: &SqliteRow,
    column: &str,
) -> Result<Option<MemberId>, StorageError> {
    get::<Option<i64>>(row, column)?
        .map(|v| to_member(column, v))
        .transpose()
}

pub(crate) fn get_id<T>(
    row: &SqliteRow,
    column: &str,
    wrap: fn(Uuid) -> T,
) -> Result<T, StorageError> {
    let value: String = get(row, column)?;
    Uuid::parse_str(&value)
        .map(wrap)
        .map_err(|e| corrupt(column, e))
}

pub(crate) fn get_opt_id<T>(
    row: &SqliteRow,
    column: &str,
    wrap: fn(Uuid) -> T,
) -> Result<Option<T>, StorageError> {
    get::<Option<String>>(row, column)?
        .map(|v| {
            Uuid::parse_str(&v)
                .map(wrap)
                .map_err(|e| corrupt(column, e))
        })
        .transpose()
}

pub(crate) fn get_money(row: &SqliteRow, column: &str) -> Result<Money, StorageError> {
    Ok(Money::from_cents(get(row, column)?))
}

pub(crate) fn get_opt_money(row: &SqliteRow, column: &str) -> Result<Option<Money>, StorageError> {
    Ok(get::<Option<i64>>(row, column)?.map(Money::from_cents))
}

pub(crate) fn parse_choice<T: DeserializeOwned>(
    column: &str,
    value: &str,
) -> Result<T, StorageError> {
    serde_json::from_value(serde_json::Value::String(value.to_string()))
        .map_err(|e| corrupt(column, e))
}

pub(crate) fn get_choice<T: DeserializeOwned>(
    row: &SqliteRow,
    column: &str,
) -> Result<T, StorageError> {
    parse_choice(column, &get::<String>(row, column)?)
}

pub(crate) fn get_u8(row: &SqliteRow, column: &str) -> Result<u8, StorageError> {
    let value: i64 = get(row, column)?;
    u8::try_from(value).map_err(|e| corrupt(column, e))
}

pub(crate) fn get_opt_num<T: TryFrom<i64>>(
    row: &SqliteRow,
    column: &str,
) -> Result<Option<T>, StorageError>
where
    T::Error: std::fmt::Display,
{
    get::<Option<i64>>(row, column)?
        .map(|v| T::try_from(v).map_err(|e| corrupt(column, e)))
        .transpose()
}

// --- Schreiben ---------------------------------------------------------------

pub(crate) fn bind_all<'q>(
    mut query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    values: impl IntoIterator<Item = Val>,
) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
    for value in values {
        query = match value {
            Val::Int(v) => query.bind(v),
            Val::Real(v) => query.bind(v),
            Val::Text(v) => query.bind(v),
        };
    }
    query
}

/// Fügt eine Zeile ein und liefert ihre `rowid`.
pub(crate) async fn insert(
    conn: &mut SqliteConnection,
    table: &str,
    columns: Vec<(&str, Val)>,
) -> Result<i64, StorageError> {
    let names: Vec<&str> = columns.iter().map(|(n, _)| *n).collect();
    let sql = format!(
        "INSERT INTO {table} ({}) VALUES ({})",
        names.join(", "),
        vec!["?"; names.len()].join(", ")
    );
    let result = bind_all(sqlx::query(&sql), columns.into_iter().map(|(_, v)| v))
        .execute(conn)
        .await?;
    Ok(result.last_insert_rowid())
}

/// Fügt eine Zeile einer Wurzeltabelle ein oder aktualisiert sie (ohne sie zu
/// löschen, damit abhängige Zeilen erhalten bleiben) und erhöht ihre Version.
pub(crate) async fn upsert(
    conn: &mut SqliteConnection,
    table: &str,
    keys: &[&str],
    columns: Vec<(&str, Val)>,
) -> Result<(), StorageError> {
    let names: Vec<&str> = columns.iter().map(|(n, _)| *n).collect();
    let mut updates: Vec<String> = names
        .iter()
        .filter(|n| !keys.contains(n))
        .map(|n| format!("{n} = excluded.{n}"))
        .collect();
    // Upserts betreffen nur Wurzeltabellen; jede Änderung erhöht deren Version.
    updates.push("version = version + 1".to_string());
    let sql = format!(
        "INSERT INTO {table} ({}) VALUES ({}) ON CONFLICT ({}) DO UPDATE SET {}",
        names.join(", "),
        vec!["?"; names.len()].join(", "),
        keys.join(", "),
        updates.join(", ")
    );
    bind_all(sqlx::query(&sql), columns.into_iter().map(|(_, v)| v))
        .execute(conn)
        .await?;
    Ok(())
}

pub(crate) fn where_clause(key: &[(&str, Val)]) -> String {
    key.iter()
        .map(|(n, _)| format!("{n} = ?"))
        .collect::<Vec<_>>()
        .join(" AND ")
}

pub(crate) async fn delete_where(
    conn: &mut SqliteConnection,
    table: &str,
    key: Vec<(&str, Val)>,
) -> Result<(), StorageError> {
    let sql = format!("DELETE FROM {table} WHERE {}", where_clause(&key));
    bind_all(sqlx::query(&sql), key.into_iter().map(|(_, v)| v))
        .execute(conn)
        .await?;
    Ok(())
}

pub(crate) async fn set_position(
    conn: &mut SqliteConnection,
    table: &str,
    key: Vec<(&str, Val)>,
    position: i64,
) -> Result<(), StorageError> {
    let sql = format!(
        "UPDATE {table} SET position = ? WHERE {}",
        where_clause(&key)
    );
    bind_all(
        sqlx::query(&sql),
        std::iter::once(int(position)).chain(key.into_iter().map(|(_, v)| v)),
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Alle Zeilen einer Tabelle in gespeicherter Reihenfolge.
pub(crate) async fn rows(
    conn: &mut SqliteConnection,
    sql: &str,
    binds: &[Val],
) -> Result<Vec<SqliteRow>, StorageError> {
    Ok(bind_all(sqlx::query(sql), binds.iter().cloned())
        .fetch_all(conn)
        .await?)
}

/// Bedingung für die Wurzeltabelle eines gezielten Ladevorgangs, z. B.
/// `WHERE id = ?`; leer = alle Zeilen.
#[derive(Clone, Debug, Default)]
pub(crate) struct Filter {
    pub cond: String,
    pub binds: Vec<Val>,
}

impl Filter {
    pub fn all() -> Self {
        Self::default()
    }

    pub fn new(cond: &str, binds: Vec<Val>) -> Self {
        Self {
            cond: cond.to_string(),
            binds,
        }
    }
}
