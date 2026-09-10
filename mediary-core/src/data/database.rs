//! # database.rs
//!
//! Functionality for handling DB operations for mediary.
//!
//! Copyright (C) 2026  Gavin Borne
//!
//! This program is free software: you can redistribute it and/or modify
//! it under the terms of the GNU General Public License as published by
//! the Free Software Foundation, either version 3 of the License, or
//! (at your option) any later version.
//!
//! This program is distributed in the hope that it will be useful,
//! but WITHOUT ANY WARRANTY; without even the implied warranty of
//! MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//! GNU General Public License for more details.
//!
//! You should have received a copy of the GNU General Public License
//! along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    path::{Path, PathBuf},
};

use rusqlite::{Connection, params};
use strum::IntoEnumIterator;

use crate::{
    data::{
        app_data::get_app_data_file,
        conversion::{
            system_time_to_unix_timestamp, unix_timestamp_to_system_time,
        },
    },
    types::{Media, MediaType},
};

const DB_NAME: &str = "mediary.db";
const DB_SCHEMA: &str = include_str!("schema.sql");

// TODO: REPLACE ALL PANICS/.expect()s WITH PROPER ERROR PROPAGATION

/// An error that may occur on SQL insert instructions.
///
/// # Variants
///
/// - `AlreadyExists` - The item already exists.
/// - `SqliteError(rusqlite::Error)` - An SQL error.
/// - `Other(Box<dyn Error>)` - Any other error.
#[derive(Debug)]
pub enum SqliteInsertError {
    AlreadyExists,
    SqliteError(rusqlite::Error),
    Other(Box<dyn Error>),
}
impl Display for SqliteInsertError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SqliteInsertError::AlreadyExists => {
                write!(f, "Item with this path already exists")
            }
            SqliteInsertError::SqliteError(e) => write!(f, "{e}"),
            SqliteInsertError::Other(e) => write!(f, "{e}"),
        }
    }
}
impl Error for SqliteInsertError {}

/// An error that may occur on SQL select instructions.
///
/// # Variants
///
/// - `DoesNotExist` - The item does not exist.
/// - `SqliteError(rusqlite::Error)` - An SQL error.
/// - `Other(Box<dyn Error>)` - Any other error.
#[derive(Debug)]
pub enum SqliteSelectError {
    SqliteError(rusqlite::Error),
    Other(Box<dyn Error>),
}
impl Display for SqliteSelectError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SqliteSelectError::SqliteError(e) => write!(f, "{e}"),
            SqliteSelectError::Other(e) => write!(f, "{e}"),
        }
    }
}
impl Error for SqliteSelectError {}

/// Connect to the database at the given path.
///
/// # Arguments
///
/// - `path` (`&Path`) - The path to the DB.
///
/// # Returns
///
/// - `rusqlite::Result<Connection>` - The DB connection
///
/// # Errors
///
/// If the DB connection fails.
fn connect_at(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    Ok(conn)
}

/// Creates the DB schema.
///
/// # Arguments
///
/// - `conn` (`&Connection`) - The DB connection.
///
/// # Returns
///
/// - `rusqlite::Result<()>` - The result of the operation.
///
/// # Errors
///
/// If the schema fails to write to the DB.
fn create_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(DB_SCHEMA)?;
    Ok(())
}

/// Ensures the builtin tags exist in the database.
///
/// # Arguments
///
/// - `conn` (`&Connection`) - The DB connection.
///
/// # Returns
///
/// - `rusqlite::Result<()>` - The result of the operation.
///
/// # Errors
///
/// If the builtin tags fail to write to the DB.
fn ensure_builtin_tags_exist_in_db(conn: &Connection) -> rusqlite::Result<()> {
    for media_type in MediaType::iter() {
        conn.execute(
            r#"
            INSERT OR IGNORE INTO tags (name, is_builtin)
            VALUES (?1, TRUE)
            "#,
            [media_type.to_string().as_str()],
        )?;
    }
    Ok(())
}

/// Initialize the database at the given path.
///
/// # Arguments
///
/// - `path` (`&Path`) - The path to the DB.
///
/// # Returns
///
/// - `rusqlite::Result<Connection>` - The DB connection.
///
/// # Errors
///
/// If the DB connection fails.
/// If the schema fails to write to the DB.
fn init_db_at(path: &Path) -> rusqlite::Result<Connection> {
    let conn = connect_at(path)?;
    create_schema(&conn)?;
    ensure_builtin_tags_exist_in_db(&conn)?;
    Ok(conn)
}

/// Initialize the database.
///
/// # Returns
///
/// - `rusqlite::Result<Connection>` - The result DB connection.
///
/// # Errors
///
/// If the DB connection fails.
/// If the schema fails to write to the DB.
pub fn init_db() -> rusqlite::Result<Connection> {
    init_db_at(
        &get_app_data_file(DB_NAME)
            .expect("Failed to write to the app data directory."),
    )
}

/// Insert a media item into the database.
///
/// # Arguments
///
/// - `conn` (`&Connection`) - The DB connection.
/// - `media` (`&Media`) - The media to insert.
///
/// # Returns
///
/// - `Result<i64, SqliteInsertError>` - The row ID of the inserted item.
///
/// # Errors
///
/// If the item already exists.
/// If the insert fails.
/// If there is an issue serializing the data.
pub fn insert_media(
    conn: &Connection,
    media: &Media,
) -> Result<i64, SqliteInsertError> {
    let path = media.path.to_string_lossy().to_string();
    let media_type = media.media_type.to_string();
    let size_bytes = media.size_bytes as i64;
    let added_at = system_time_to_unix_timestamp(media.added_at)
        .map_err(|e| SqliteInsertError::Other(Box::new(e)))?;

    let res = conn.execute(
        r#"
        INSERT INTO media (path, media_type, size_bytes, added_at)
        VALUES (?1, ?2, ?3, ?4)
        "#,
        params![path, media_type, size_bytes, added_at],
    );
    match res {
        Ok(_) => Ok(conn.last_insert_rowid()),
        Err(rusqlite::Error::SqliteFailure(err, _))
            if err.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            Err(SqliteInsertError::AlreadyExists)
        }
        Err(e) => Err(SqliteInsertError::SqliteError(e)),
    }
}

/// Get a media item from a file path.
///
/// # Arguments
///
/// - `conn` (`&Connection`) - The DB connection.
/// - `path` (`&str`) - The path to the media.
///
/// # Returns
///
/// - `Result<Option<Media>, SqliteSelectError>` - The media.
///
/// # Errors
///
/// If the select fails.
/// If there is an issue deserializing the data.
pub fn get_media_from_path(
    conn: &Connection,
    path: &str,
) -> Result<Option<Media>, SqliteSelectError> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, path, media_type, size_bytes, added_at
            FROM media
            WHERE path = ?1
            "#,
        )
        .map_err(SqliteSelectError::SqliteError)?;

    let mut media_iter = stmt
        .query_map(params![path], |row| {
            Ok(Media {
                id: Some(row.get(0)?),
                path: {
                    let raw: String = row.get(1)?;
                    PathBuf::from(raw)
                },
                media_type: {
                    let raw: String = row.get(2)?;
                    raw.parse::<MediaType>().map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?
                },
                size_bytes: {
                    let raw: i64 = row.get(3)?;
                    if raw < 0 {
                        Err(rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Integer,
                            "Size cannot be negative".into(),
                        ))
                    } else {
                        Ok(raw as u64)
                    }
                }?,
                added_at: {
                    let raw: i64 = row.get(4)?;
                    unix_timestamp_to_system_time(raw).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
                            rusqlite::types::Type::Integer,
                            e,
                        )
                    })?
                },
            })
        })
        .map_err(SqliteSelectError::SqliteError)?;

    match media_iter.next() {
        Some(Ok(media)) => Ok(Some(media)),
        Some(Err(e)) => Err(SqliteSelectError::SqliteError(e)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    // This also returns the temp file to keep it in scope so that
    // it doesn't get deleted after leaving this function's scope
    fn temp_db_conn() -> (NamedTempFile, Connection) {
        let tmp = NamedTempFile::new().unwrap();
        let conn = connect_at(tmp.path()).unwrap();
        (tmp, conn)
    }

    #[test]
    fn test_connect_at_enables_foreign_keys() {
        let (_tmp, conn) = temp_db_conn();
        let fk_enabled: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fk_enabled, 1);
    }

    #[test]
    fn test_create_schema_is_idempotent() {
        let (_tmp, conn) = temp_db_conn();
        create_schema(&conn).unwrap();
        create_schema(&conn).unwrap();
    }

    #[test]
    fn test_foreign_keys_are_enforced() {
        let (_tmp, conn) = temp_db_conn();
        create_schema(&conn).unwrap();
        let result = conn.execute(
            "INSERT INTO media_tags (media_id, tag_id) VALUES (1, 999)",
            [],
        );
        assert!(result.is_err());
    }

    #[test]
    fn builtin_tags_seeded_without_duplicates() {
        let (_tmp, conn) = temp_db_conn();
        create_schema(&conn).unwrap();
        ensure_builtin_tags_exist_in_db(&conn).unwrap();
        ensure_builtin_tags_exist_in_db(&conn).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tags WHERE is_builtin = TRUE",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, MediaType::iter().count() as i64);
    }

    #[test]
    fn init_db_at_runs_full_pipeline() {
        let tmp = NamedTempFile::new().unwrap();
        let conn = init_db_at(tmp.path()).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tags", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, MediaType::iter().count() as i64);
    }
}
