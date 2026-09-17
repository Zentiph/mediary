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
    str::FromStr,
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
    types::{Media, MediaType, Tag},
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
    Other(Box<dyn Error + Send + Sync>),
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
/// - `NotFound` - The item does not exist.
/// - `SqliteError(rusqlite::Error)` - An SQL error.
/// - `Other(Box<dyn Error>)` - Any other error.
#[derive(Debug)]
pub enum SqliteSelectError {
    SqliteError(rusqlite::Error),
    Other(Box<dyn Error + Send + Sync>),
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

/// An error that may occur on SQL delete instructions.
///
/// # Variants
///
/// - `NotFound` - The item does not exist.
/// - `SqliteError(rusqlite::Error)` - An SQL error.
/// - `Other(Box<dyn Error + Send + Sync>)` - Any other error.
#[derive(Debug)]
pub enum SqliteDeleteError {
    NotFound,
    SqliteError(rusqlite::Error),
    Other(Box<dyn Error + Send + Sync>),
}
impl Display for SqliteDeleteError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SqliteDeleteError::NotFound => write!(f, "Item does not exist"),
            SqliteDeleteError::SqliteError(e) => write!(f, "{e}"),
            SqliteDeleteError::Other(e) => write!(f, "{e}"),
        }
    }
}
impl Error for SqliteDeleteError {}

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
    // TODO: simplify to use query_row since paths are unique
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

/// Delete a media item from a file path.
///
/// # Arguments
///
/// - `conn` (`&Connection`) - The DB connection.
/// - `path` (`&str`) - The path to the media.
///
/// # Returns
///
/// - `Result<(), SqliteDeleteError>` - The result of the operation.
///
/// # Errors
///
/// If the media already did not exist.
/// If the delete fails.
pub fn delete_media_from_path(
    conn: &Connection,
    path: &str,
) -> Result<(), SqliteDeleteError> {
    let res = conn.execute(
        r#"
        DELETE FROM media
        WHERE path = ?1
        "#,
        params![path],
    );
    match res {
        Ok(0) => Err(SqliteDeleteError::NotFound),
        Ok(_) => Ok(()),
        Err(e) => Err(SqliteDeleteError::SqliteError(e)),
    }
}

/// Insert a custom tag into the database.
///
/// # Arguments
///
/// - `conn` (`&Connection`) - The DB connection.
/// - `tag` (`&Tag`) - The tag.
///
/// # Returns
///
/// - `Result<i64, SqliteInsertError>` - The row ID of the inserted item.
///
/// # Errors
///
/// If the tag already exists.
/// If the insert fails.
pub fn insert_custom_tag(
    conn: &Connection,
    tag: &Tag,
) -> Result<i64, SqliteInsertError> {
    match tag {
        Tag::Builtin { .. } => Err(SqliteInsertError::AlreadyExists),
        Tag::Custom { name, .. } => {
            let res = conn.execute(
                r#"
                INSERT INTO tags (name)
                VALUES (?1)
                "#,
                params![name],
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
    }
}

/// Get a tag from its name.
///
/// # Arguments
///
/// - `conn` (`&Connection`) - The DB connection.
/// - `name` (`&str`) - The name of the tag.
///
/// # Returns
///
/// - `Result<Tag, SqliteSelectError>` - The tag.
///
/// # Errors
///
/// If the tag does not exist.
///
/// # Examples
///
/// ```
/// use crate::...;
///
/// let _ = get_tag_from_name();
/// ```
pub fn get_tag_from_name(
    conn: &Connection,
    name: &str,
) -> Result<Option<Tag>, SqliteSelectError> {
    let res = conn.query_row(
        r#"
        SELECT id, name, is_builtin FROM tags
        WHERE name = ?1
        "#,
        params![name],
        |row| {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let is_builtin: bool = row.get(2)?;
            Ok(match is_builtin {
                true => Tag::Builtin {
                    id: Some(id),
                    media_type: MediaType::from_str(&name).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?,
                },
                false => Tag::Custom { id: Some(id), name },
            })
        },
    );
    match res {
        Ok(tag) => Ok(Some(tag)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(SqliteSelectError::SqliteError(e)),
    }
}

pub fn delete_tag_from_name(
    conn: &Connection,
    name: &str,
) -> Result<(), SqliteDeleteError> {
    if MediaType::iter().any(|mt| mt.to_string() == name) {
        Err(SqliteDeleteError::Other(
            // TODO: maybe make this a bit more semantic, maybe its own error?
            "Cannot delete a builtin tag".into(),
        ))
    } else {
        let res = conn.execute(
            r#"
            DELETE FROM tags
            WHERE name = ?1
            "#,
            params![name],
        );
        match res {
            Ok(0) => Err(SqliteDeleteError::NotFound),
            Ok(_) => Ok(()),
            Err(e) => Err(SqliteDeleteError::SqliteError(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use tempfile::NamedTempFile;

    fn get_tag_id(tag: &Tag) -> Option<i64> {
        match tag {
            Tag::Builtin { id, .. } => *id,
            Tag::Custom { id, .. } => *id,
        }
    }

    fn get_custom_tag_name(tag: &Tag) -> String {
        match tag {
            Tag::Builtin { .. } => panic!("Expected custom tag"),
            Tag::Custom { name, .. } => name.to_string(),
        }
    }

    // This also returns the temp file to keep it in scope so that
    // it doesn't get deleted after leaving this function's scope
    fn temp_db_conn() -> (NamedTempFile, Connection) {
        let tmp = NamedTempFile::new().unwrap();
        let conn = connect_at(tmp.path()).unwrap();
        (tmp, conn)
    }

    #[test]
    fn connect_at_enables_foreign_keys() {
        let (_tmp, conn) = temp_db_conn();
        let fk_enabled: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fk_enabled, 1);
    }

    #[test]
    fn create_schema_is_idempotent() {
        let (_tmp, conn) = temp_db_conn();
        create_schema(&conn).unwrap();
        create_schema(&conn).unwrap();
    }

    #[test]
    fn foreign_keys_are_enforced() {
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

    #[test]
    fn insert_media_inserts_correctly() {
        let (_tmp, conn) = temp_db_conn();
        create_schema(&conn).unwrap();
        let media = Media {
            id: None,
            path: PathBuf::from("test"),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };
        let id = insert_media(&conn, &media).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn insert_media_errors_on_duplicate() {
        let (_tmp, conn) = temp_db_conn();
        create_schema(&conn).unwrap();
        let media = Media {
            id: None,
            path: PathBuf::from("test"),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };
        insert_media(&conn, &media).unwrap();
        let result = insert_media(&conn, &media);
        assert!(matches!(result, Err(SqliteInsertError::AlreadyExists)));
    }

    #[test]
    fn insert_get_media_round_trip() {
        let (_tmp, conn) = temp_db_conn();
        create_schema(&conn).unwrap();
        let media = Media {
            id: None,
            path: PathBuf::from("test"),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };
        insert_media(&conn, &media).unwrap();
        let result =
            get_media_from_path(&conn, &media.path.to_string_lossy()).unwrap();
        let unwrapped = result.unwrap();
        assert_eq!(media, unwrapped);
        assert!(unwrapped.id.is_some());
        assert_eq!(media.media_type, unwrapped.media_type);
        assert_eq!(media.size_bytes, unwrapped.size_bytes);
        assert_eq!(
            system_time_to_unix_timestamp(media.added_at).unwrap(),
            system_time_to_unix_timestamp(unwrapped.added_at).unwrap()
        );
    }

    #[test]
    fn get_media_returns_none_on_nonexistent_path() {
        let (_tmp, conn) = temp_db_conn();
        create_schema(&conn).unwrap();
        let result = get_media_from_path(&conn, "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn delete_media_from_path_errors_on_nonexistent_path() {
        let (_tmp, conn) = temp_db_conn();
        create_schema(&conn).unwrap();
        let result = delete_media_from_path(&conn, "nonexistent");
        assert!(matches!(result, Err(SqliteDeleteError::NotFound)));
    }

    #[test]
    fn delete_media_from_path_deletes_correctly() {
        let (_tmp, conn) = temp_db_conn();
        create_schema(&conn).unwrap();
        let media = Media {
            id: None,
            path: PathBuf::from("test"),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };
        insert_media(&conn, &media).unwrap();
        delete_media_from_path(&conn, &media.path.to_string_lossy()).unwrap();
        let result =
            get_media_from_path(&conn, &media.path.to_string_lossy()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn insert_tag_inserts_correctly() {
        let (_tmp, conn) = temp_db_conn();
        create_schema(&conn).unwrap();
        let tag = Tag::Custom {
            id: None,
            name: String::from("test"),
        };
        let id = insert_custom_tag(&conn, &tag).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn insert_tag_errors_on_duplicate() {
        let (_tmp, conn) = temp_db_conn();
        create_schema(&conn).unwrap();
        let tag = Tag::Custom {
            id: None,
            name: String::from("test"),
        };
        insert_custom_tag(&conn, &tag).unwrap();
        let result = insert_custom_tag(&conn, &tag);
        assert!(matches!(result, Err(SqliteInsertError::AlreadyExists)));
    }

    #[test]
    fn insert_get_tag_round_trip() {
        let (_tmp, conn) = temp_db_conn();
        create_schema(&conn).unwrap();
        let tag = Tag::Custom {
            id: None,
            name: String::from("test"),
        };
        insert_custom_tag(&conn, &tag).unwrap();
        let result =
            get_tag_from_name(&conn, &get_custom_tag_name(&tag)).unwrap();
        let unwrapped = result.unwrap();
        assert_eq!(tag, unwrapped);
        assert!(get_tag_id(&unwrapped).is_some());
    }

    #[test]
    fn get_tag_returns_none_on_nonexistent_name() {
        let (_tmp, conn) = temp_db_conn();
        create_schema(&conn).unwrap();
        let result = get_tag_from_name(&conn, "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn delete_tag_from_name_errors_on_nonexistent_name() {
        let (_tmp, conn) = temp_db_conn();
        create_schema(&conn).unwrap();
        let result = delete_tag_from_name(&conn, "nonexistent");
        assert!(matches!(result, Err(SqliteDeleteError::NotFound)));
    }

    #[test]
    fn delete_tag_from_name_deletes_correctly() {
        let (_tmp, conn) = temp_db_conn();
        create_schema(&conn).unwrap();
        let tag = Tag::Custom {
            id: None,
            name: String::from("test"),
        };
        insert_custom_tag(&conn, &tag).unwrap();
        delete_tag_from_name(&conn, &get_custom_tag_name(&tag)).unwrap();
        let result =
            get_tag_from_name(&conn, &get_custom_tag_name(&tag)).unwrap();
        assert!(result.is_none());
    }
}
