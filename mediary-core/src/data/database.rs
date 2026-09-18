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

use rusqlite::{Connection, Row, params};
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
// TODO: Look into making media + tag deletions cascade so that tag-media
//       relations don't point to non-existent items
// TODO: See if it's worth it to heavily modularize this; there are many
//       functions that share similar code but it's mostly just between 2
//       functions

/// An error that may occur on SQL insert instructions.
///
/// # Variants
///
/// - `Duplicate` - The item already exists.
/// - `MissingRelationMember` - A relation member does not exist.
/// - `SqliteError(rusqlite::Error)` - An SQL error.
/// - `Other(Box<dyn Error>)` - Any other error.
#[derive(Debug)]
pub enum SqliteInsertError {
    Duplicate,
    MissingRelationMember,
    SqliteError(rusqlite::Error),
    Other(Box<dyn Error + Send + Sync>),
}
impl Display for SqliteInsertError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SqliteInsertError::Duplicate => {
                write!(
                    f,
                    "Tried to insert an item that already exists into a table that does not allow duplicates"
                )
            }
            SqliteInsertError::MissingRelationMember => {
                write!(
                    f,
                    "Tried to insert a relation containing a member that does not exist"
                )
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
/// - `Disallowed` - The item is not allowed to be deleted.
/// - `SqliteError(rusqlite::Error)` - An SQL error.
/// - `Other(Box<dyn Error + Send + Sync>)` - Any other error.
#[derive(Debug)]
pub enum SqliteDeleteError {
    NotFound,
    Disallowed,
    SqliteError(rusqlite::Error),
    Other(Box<dyn Error + Send + Sync>),
}
impl Display for SqliteDeleteError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SqliteDeleteError::NotFound => {
                write!(f, "Item to delete already does not exist")
            }
            SqliteDeleteError::Disallowed => {
                write!(f, "Item to delete is not allowed to be deleted")
            }
            SqliteDeleteError::SqliteError(e) => write!(f, "{e}"),
            SqliteDeleteError::Other(e) => write!(f, "{e}"),
        }
    }
}
impl Error for SqliteDeleteError {}

/// Get the ID of a tag.
///
/// # Arguments
///
/// - `tag` (`&Tag`) - The tag.
///
/// # Returns
///
/// - `Option<i64>` - The tag ID.
fn get_tag_id(tag: &Tag) -> Option<i64> {
    match tag {
        Tag::Builtin { id, .. } => *id,
        Tag::Custom { id, .. } => *id,
    }
}

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

/// Get the IDs of the builtin tags.
///
/// # Arguments
///
/// - `conn` (`&Connection`) - The DB connection.
///
/// # Returns
///
/// - `rusqlite::Result<Vec<i64>>` - The IDs.
///
/// # Errors
///
/// If the IDs fail to be fetched.
fn get_builtin_tag_ids(conn: &Connection) -> rusqlite::Result<Vec<i64>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id
        FROM tags
        WHERE is_builtin = TRUE
        "#,
    )?;
    let mut rows = stmt.query([])?;
    let mut res = Vec::new();
    while let Some(row) = rows.next()? {
        res.push(row.get(0)?);
    }
    Ok(res)
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

/// Check if an item exists in a table.
///
/// # Arguments
///
/// - `conn` (`&Connection`) - The DB connection.
/// - `table` (`&str`) - The table the item exists in.
/// - `id` (`i64`) - The ID of the item.
///
/// # Returns
///
/// - `rusqlite::Result<bool>` - Whether the item exists.
///
/// # Errors
///
/// If the query fails.
fn item_exists(
    conn: &Connection,
    table: &str,
    id: i64,
) -> rusqlite::Result<bool> {
    let query = format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id = ?1)");
    conn.query_row(&query, params![id], |row| row.get(0))
}

/// Read a media item from a row.
///
/// # Arguments
///
/// - `row` (`&Row`) - The row.
///
/// # Returns
///
/// - `Result<Media, rusqlite::Error>` - The media.
///
/// # Errors
///
/// If the row cannot be parsed.
fn read_media_from_row(row: &Row) -> Result<Media, rusqlite::Error> {
    let id: i64 = row.get(0)?;
    let path: String = row.get(1)?;
    let media_type: String = row.get(2)?;
    let size_bytes: i64 = row.get(3)?;
    let added_at: i64 = row.get(4)?;
    Ok(Media {
        id: Some(id),
        path: PathBuf::from(path),
        media_type: MediaType::from_str(&media_type).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                Box::new(e),
            )
        })?,
        size_bytes: {
            if size_bytes < 0 {
                Err(rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Integer,
                    "size_bytes cannot be negative".into(),
                ))
            } else {
                Ok(size_bytes as u64)
            }
        }?,
        added_at: unix_timestamp_to_system_time(added_at).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Integer,
                e,
            )
        })?,
    })
}

/// Read a tag from a row.
///
/// # Arguments
///
/// - `row` (`&Row`) - The DB row.
///
/// # Returns
///
/// - `Result<Tag, rusqlite::Error>` - The tag.
///
/// # Errors
///
/// If the row cannot be parsed.
fn read_tag_from_row(row: &Row) -> Result<Tag, rusqlite::Error> {
    let id: i64 = row.get(0)?;
    let name: String = row.get(1)?;
    let is_builtin: bool = row.get(2)?;
    Ok(match is_builtin {
        true => Tag::Builtin {
            id: Some(id),
            media_type: MediaType::from_str(&name).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?,
        },
        false => Tag::Custom { id: Some(id), name },
    })
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
/// - `Result<Media, SqliteInsertError>` - The inserted item.
///
/// # Errors
///
/// If the item already exists.
/// If the insert fails.
/// If there is an issue serializing the data.
pub fn insert_media(
    conn: &Connection,
    media: &Media,
) -> Result<Media, SqliteInsertError> {
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
        Ok(_) => Ok({
            let mut m = media.clone();
            m.id = Some(conn.last_insert_rowid());
            m
        }),
        Err(rusqlite::Error::SqliteFailure(err, _))
            if err.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            Err(SqliteInsertError::Duplicate)
        }
        Err(e) => Err(SqliteInsertError::SqliteError(e)),
    }
}

/// Get a media item from its ID.
///
/// # Arguments
///
/// - `conn` (`&Connection`) - The DB connection.
/// - `id` (`i64`) - The ID.
///
/// # Returns
///
/// - `Result<Option<Media>, SqliteSelectError>` - The media.
///
/// # Errors
///
/// If the query fails.
pub fn get_media_from_id(
    conn: &Connection,
    id: i64,
) -> Result<Option<Media>, SqliteSelectError> {
    let res = conn.query_row(
        r#"
        SELECT id, path, media_type, size_bytes, added_at
        FROM media
        WHERE id = ?1
        "#,
        params![id],
        read_media_from_row,
    );

    match res {
        Ok(media) => Ok(Some(media)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(SqliteSelectError::SqliteError(e)),
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
    let res = conn.query_row(
        r#"
            SELECT id, path, media_type, size_bytes, added_at
            FROM media
            WHERE path = ?1
            "#,
        params![path],
        read_media_from_row,
    );

    match res {
        Ok(media) => Ok(Some(media)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(SqliteSelectError::SqliteError(e)),
    }
}

/// Delete a media item from its ID.
///
/// # Arguments
///
/// - `conn` (`&Connection`) - The DB connection.
/// - `id` (`i64`) - The ID.
///
/// # Returns
///
/// - `Result<(), SqliteDeleteError>` - The result of the operation.
///
/// # Errors
///
/// If the media already did not exist.
/// If the delete fails.
pub fn delete_media_from_id(
    conn: &Connection,
    id: i64,
) -> Result<(), SqliteDeleteError> {
    let res = conn.execute(
        r#"
        DELETE FROM media
        WHERE id = ?1
        "#,
        params![id],
    );

    match res {
        Ok(0) => Err(SqliteDeleteError::NotFound),
        Ok(_) => Ok(()),
        Err(e) => Err(SqliteDeleteError::SqliteError(e)),
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
/// - `Result<Tag, SqliteInsertError>` - The inserted item.
///
/// # Errors
///
/// If the tag already exists.
/// If the insert fails.
pub fn insert_tag(
    conn: &Connection,
    tag: &Tag,
) -> Result<Tag, SqliteInsertError> {
    match tag {
        Tag::Builtin { .. } => Err(SqliteInsertError::Duplicate),
        Tag::Custom { name, .. } => {
            let res = conn.execute(
                r#"
                INSERT INTO tags (name)
                VALUES (?1)
                "#,
                params![name],
            );

            match res {
                Ok(_) => Ok(Tag::Custom {
                    id: Some(conn.last_insert_rowid()),
                    name: name.clone(),
                }),
                Err(rusqlite::Error::SqliteFailure(err, _))
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    Err(SqliteInsertError::Duplicate)
                }
                Err(e) => Err(SqliteInsertError::SqliteError(e)),
            }
        }
    }
}

/// Get a tag from an ID.
///
/// # Arguments
///
/// - `conn` (`&Connection`) - The DB connection.
/// - `id` (`i64`) - The tag ID.
///
/// # Returns
///
/// - `Result<Option<Tag>, SqliteSelectError>` - The tag.
///
/// # Errors
///
/// If the query fails.
pub fn get_tag_from_id(
    conn: &Connection,
    id: i64,
) -> Result<Option<Tag>, SqliteSelectError> {
    let res = conn.query_row(
        r#"
        SELECT id, name, is_builtin
        FROM tags
        WHERE id = ?1
        "#,
        params![id],
        read_tag_from_row,
    );

    match res {
        Ok(tag) => Ok(Some(tag)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(SqliteSelectError::SqliteError(e)),
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
/// If the select fails.
pub fn get_tag_from_name(
    conn: &Connection,
    name: &str,
) -> Result<Option<Tag>, SqliteSelectError> {
    let res = conn.query_row(
        r#"
        SELECT id, name, is_builtin
        FROM tags
        WHERE name = ?1
        "#,
        params![name],
        read_tag_from_row,
    );

    match res {
        Ok(tag) => Ok(Some(tag)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(SqliteSelectError::SqliteError(e)),
    }
}

/// Delete a tag from its ID.
///
/// # Arguments
///
/// - `conn` (`&Connection`) - The DB connection.
/// - `id` (`i64`) - The ID of the tag.
///
/// # Returns
///
/// - `Result<(), SqliteDeleteError>` - The result of the operation.
///
/// # Errors
///
/// If the tag does not exist.
/// If the delete fails.
pub fn delete_tag_from_id(
    conn: &Connection,
    id: i64,
) -> Result<(), SqliteDeleteError> {
    let builtin_ids = get_builtin_tag_ids(conn)
        .map_err(|e| SqliteDeleteError::Other(Box::new(e)))?;
    if builtin_ids.contains(&id) {
        return Err(SqliteDeleteError::Disallowed);
    }

    let res = conn.execute(
        r#"
        DELETE FROM tags
        WHERE id = ?1
        "#,
        params![id],
    );

    match res {
        Ok(0) => Err(SqliteDeleteError::NotFound),
        Ok(_) => Ok(()),
        Err(e) => Err(SqliteDeleteError::SqliteError(e)),
    }
}

/// Delete a tag from its name.
///
/// # Arguments
///
/// - `conn` (`&Connection`) - The DB connection.
/// - `name` (`&str`) - The name of the tag.
///
/// # Returns
///
/// - `Result<(), SqliteDeleteError>` - The result of the operation.
///
/// # Errors
///
/// If the tag does not exist.
/// If the delete fails.
pub fn delete_tag_from_name(
    conn: &Connection,
    name: &str,
) -> Result<(), SqliteDeleteError> {
    if MediaType::iter().any(|mt| mt.to_string() == name) {
        return Err(SqliteDeleteError::Disallowed);
    }

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

/// Tag a media item.
///
/// # Arguments
///
/// - `conn` (`&Connection`) - The DB connection.
/// - `media` (`&Media`) - The media to tag.
/// - `tag` (`&Tag`) - The tag to add.
///
/// # Returns
///
/// - `Result<i64, SqliteInsertError>` - The row ID of the inserted item.
///
/// # Errors
///
/// If the tag already exists on the media.
/// If the insert fails.
pub fn tag_media(
    conn: &Connection,
    media: &Media,
    tag: &Tag,
) -> Result<i64, SqliteInsertError> {
    let media_id = match media.id {
        Some(id) => id,
        None => return Err(SqliteInsertError::MissingRelationMember),
    };
    let tag_id = match get_tag_id(tag) {
        Some(id) => id,
        None => return Err(SqliteInsertError::MissingRelationMember),
    };

    let media_exists = item_exists(conn, "media", media_id)
        .map_err(SqliteInsertError::SqliteError)?;
    let tag_exists = item_exists(conn, "tags", tag_id)
        .map_err(SqliteInsertError::SqliteError)?;

    if !media_exists || !tag_exists {
        return Err(SqliteInsertError::MissingRelationMember);
    }

    let res = conn.execute(
        r#"
        INSERT INTO media_tags (media_id, tag_id)
        VALUES (?1, ?2)
        "#,
        params![media.id, get_tag_id(tag).unwrap()],
    );

    match res {
        Ok(_) => Ok(conn.last_insert_rowid()),
        Err(rusqlite::Error::SqliteFailure(err, _))
            if err.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            Err(SqliteInsertError::Duplicate)
        }
        Err(e) => Err(SqliteInsertError::SqliteError(e)),
    }
}

/// Get the tags for a media item.
///
/// # Arguments
///
/// - `conn` (`&Connection`) - The DB connection.
/// - `media` (`&Media`) - The media to get tags for.
///
/// # Returns
///
/// - `Result<Option<Vec<Tag>>, SqliteSelectError>` - The tags for the media.
///
/// # Errors
///
/// If the select fails.
pub fn get_tags_for_media(
    conn: &Connection,
    media: &Media,
) -> Result<Option<Vec<Tag>>, SqliteSelectError> {
    let media_id = match media.id {
        Some(id) => id,
        None => return Ok(None),
    };

    if !item_exists(conn, "media", media_id)
        .map_err(SqliteSelectError::SqliteError)?
    {
        return Ok(None);
    }

    let mut stmt = conn
        .prepare(
            r#"
            SELECT t.id, t.name, t.is_builtin
            FROM media_tags mt
            JOIN tags t ON mt.tag_id = t.id
            WHERE mt.media_id = ?1
            "#,
        )
        .map_err(SqliteSelectError::SqliteError)?;

    let tags_iter = stmt
        .query_map(params![media_id], read_tag_from_row)
        .map_err(SqliteSelectError::SqliteError)?;

    let mut tags = Vec::new();
    for tag_result in tags_iter {
        tags.push(tag_result.map_err(SqliteSelectError::SqliteError)?);
    }

    Ok(Some(tags))
}

/// Get the media for a tag.
///
/// # Arguments
///
/// - `conn` (`&Connection`) - The DB connection.
/// - `tag` (`&Tag`) - The tag to get media for.
///
/// # Returns
///
/// - `Result<Option<Vec<Media>>, SqliteSelectError>` - The media for the tag.
///
/// # Errors
///
/// If the select fails.
pub fn get_media_for_tag(
    conn: &Connection,
    tag: &Tag,
) -> Result<Option<Vec<Media>>, SqliteSelectError> {
    let tag_id = match get_tag_id(tag) {
        Some(id) => id,
        None => return Ok(None),
    };

    // check if tag exists
    if !item_exists(conn, "tags", tag_id)
        .map_err(SqliteSelectError::SqliteError)?
    {
        return Ok(None);
    }

    let mut stmt = conn
        .prepare(
            r#"
        SELECT m.id, m.path, m.media_type, m.size_bytes, m.added_at
        FROM media_tags mt
        JOIN media m ON mt.media_id = m.id
        WHERE mt.tag_id = ?1
        "#,
        )
        .map_err(SqliteSelectError::SqliteError)?;

    let media_iter = stmt
        .query_map(params![tag_id], read_media_from_row)
        .map_err(SqliteSelectError::SqliteError)?;

    let mut media = Vec::new();
    for media_result in media_iter {
        media.push(media_result.map_err(SqliteSelectError::SqliteError)?);
    }

    Ok(Some(media))
}

/// Untag a media item.
///
/// # Arguments
///
/// - `conn` (`&Connection`) - The DB connection.
/// - `media` (`&Media`) - The media to untag.
/// - `tag` (`&Tag`) - The tag to remove.
///
/// # Returns
///
/// - `Result<(), SqliteDeleteError>` - The result of the operation.
///
/// # Errors
///
/// If the media-tag relationship already did not exist.
/// If the delete fails.
pub fn untag_media(
    conn: &Connection,
    media: &Media,
    tag: &Tag,
) -> Result<(), SqliteDeleteError> {
    let res = conn.execute(
        r#"
        DELETE FROM media_tags
        WHERE media_id = ?1 AND tag_id = ?2
        "#,
        params![media.id, get_tag_id(tag).unwrap()],
    );

    match res {
        Ok(0) => Err(SqliteDeleteError::NotFound),
        Ok(_) => Ok(()),
        Err(e) => Err(SqliteDeleteError::SqliteError(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use tempfile::NamedTempFile;

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

    fn temp_db_init() -> (NamedTempFile, Connection) {
        let tmp = NamedTempFile::new().unwrap();
        let conn = init_db_at(tmp.path()).unwrap();
        (tmp, conn)
    }

    #[test]
    fn create_schema_is_idempotent() {
        // setup
        let (_tmp, conn) = temp_db_conn();

        // invoke + check
        create_schema(&conn).unwrap();
        create_schema(&conn).unwrap();
    }

    #[test]
    fn builtin_tags_seeded_without_duplicates() {
        // setup
        let (_tmp, conn) = temp_db_init();

        // invoke
        ensure_builtin_tags_exist_in_db(&conn).unwrap();
        ensure_builtin_tags_exist_in_db(&conn).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tags WHERE is_builtin = TRUE",
                [],
                |row| row.get(0),
            )
            .unwrap();

        // check
        assert_eq!(count, MediaType::iter().count() as i64);
    }

    #[test]
    fn init_db_enables_foreign_keys() {
        // setup
        let (_tmp, conn) = temp_db_init();

        // invoke
        let fk_enabled: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();

        // check
        assert_eq!(fk_enabled, 1);
    }

    #[test]
    fn foreign_keys_are_enforced() {
        // setup
        let (_tmp, conn) = temp_db_init();

        // invoke
        let result = conn.execute(
            "INSERT INTO media_tags (media_id, tag_id) VALUES (1, 999)",
            [],
        );

        // check
        assert!(result.is_err());
    }

    #[test]
    fn init_db_at_runs_full_pipeline() {
        // setup
        let tmp = NamedTempFile::new().unwrap();

        // invoke
        let conn = init_db_at(tmp.path()).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tags", [], |row| row.get(0))
            .unwrap();

        // check
        assert_eq!(count, MediaType::iter().count() as i64);
    }

    #[test]
    fn insert_media_inserts_correctly() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let media = Media {
            id: None,
            path: "path".into(),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };

        // invoke
        let media = insert_media(&conn, &media).unwrap();

        // check
        assert!(media.id.is_some());
    }

    #[test]
    fn insert_media_fails_if_media_already_exists() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let media = Media {
            id: None,
            path: "path".into(),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };

        // invoke
        let media = insert_media(&conn, &media).unwrap();

        // check
        assert!(matches!(
            insert_media(&conn, &media).unwrap_err(),
            SqliteInsertError::Duplicate
        ));
    }

    #[test]
    fn get_media_from_id_returns_correct_media() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let media = Media {
            id: None,
            path: "path".into(),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };
        let id = insert_media(&conn, &media).unwrap().id.unwrap();

        // invoke
        let result = get_media_from_id(&conn, id).unwrap();

        // check
        assert_eq!(result.unwrap().id.unwrap(), id);
    }

    #[test]
    fn get_media_from_id_returns_none_if_media_does_not_exist() {
        // setup
        let (_tmp, conn) = temp_db_init();

        // invoke
        let result = get_media_from_id(&conn, 9999).unwrap();

        // check
        assert!(result.is_none());
    }

    #[test]
    fn get_media_from_path_returns_correct_media() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let media = Media {
            id: None,
            path: "path".into(),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };
        let media = insert_media(&conn, &media).unwrap();

        // invoke
        let result =
            get_media_from_path(&conn, media.path.to_str().unwrap()).unwrap();

        // check
        assert_eq!(result.unwrap().path, media.path);
    }

    #[test]
    fn get_media_from_path_returns_none_if_media_does_not_exist() {
        // setup
        let (_tmp, conn) = temp_db_init();

        // invoke
        let result = get_media_from_path(&conn, "path").unwrap();

        // check
        assert!(result.is_none());
    }

    #[test]
    fn delete_media_from_id_deletes_correct_media() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let media = Media {
            id: None,
            path: "path".into(),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };
        let id = insert_media(&conn, &media).unwrap().id.unwrap();

        // invoke
        delete_media_from_id(&conn, id).unwrap();

        // check
        assert!(get_media_from_id(&conn, id).unwrap().is_none());
    }

    #[test]
    fn delete_media_from_id_fails_if_media_does_not_exist() {
        // setup
        let (_tmp, conn) = temp_db_init();

        // invoke + check
        assert!(matches!(
            delete_media_from_id(&conn, 9999).unwrap_err(),
            SqliteDeleteError::NotFound
        ));
    }

    #[test]
    fn delete_media_from_path_deletes_correct_media() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let media = Media {
            id: None,
            path: "path".into(),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };
        let media = insert_media(&conn, &media).unwrap();

        // invoke
        delete_media_from_path(&conn, media.path.to_str().unwrap()).unwrap();

        // check
        assert!(
            get_media_from_path(&conn, media.path.to_str().unwrap())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn delete_media_from_path_fails_if_media_does_not_exist() {
        // setup
        let (_tmp, conn) = temp_db_init();

        // invoke + check
        assert!(matches!(
            delete_media_from_path(&conn, "path").unwrap_err(),
            SqliteDeleteError::NotFound
        ));
    }

    #[test]
    fn insert_tag_inserts_correctly() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let tag = Tag::Custom {
            id: None,
            name: "tag".into(),
        };

        // invoke
        let tag = insert_tag(&conn, &tag).unwrap();

        // check
        assert!(get_tag_id(&tag).is_some());
    }

    #[test]
    fn insert_tag_fails_if_tag_already_exists() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let tag = Tag::Custom {
            id: None,
            name: "tag".into(),
        };

        // invoke
        let tag = insert_tag(&conn, &tag).unwrap();

        // check
        assert!(matches!(
            insert_tag(&conn, &tag).unwrap_err(),
            SqliteInsertError::Duplicate
        ));
    }

    #[test]
    fn get_tag_from_id_returns_correct_tag() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let tag = Tag::Custom {
            id: None,
            name: "tag".into(),
        };
        let id = get_tag_id(&insert_tag(&conn, &tag).unwrap()).unwrap();

        // invoke
        let result = get_tag_from_id(&conn, id).unwrap();

        // check
        assert_eq!(get_tag_id(&result.unwrap()).unwrap(), id);
    }

    #[test]
    fn get_tag_from_id_returns_none_if_tag_does_not_exist() {
        // setup
        let (_tmp, conn) = temp_db_init();

        // invoke
        let result = get_tag_from_id(&conn, 9999).unwrap();

        // check
        assert!(result.is_none());
    }

    #[test]
    fn get_tag_from_name_returns_correct_tag() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let tag = Tag::Custom {
            id: None,
            name: "tag".into(),
        };
        insert_tag(&conn, &tag).unwrap();

        // invoke
        let result = get_tag_from_name(&conn, "tag").unwrap();

        // check
        assert_eq!(get_custom_tag_name(&result.unwrap()), "tag");
    }

    #[test]
    fn get_tag_from_name_returns_none_if_tag_does_not_exist() {
        // setup
        let (_tmp, conn) = temp_db_init();

        // invoke
        let result = get_tag_from_name(&conn, "tag").unwrap();

        // check
        assert!(result.is_none());
    }

    #[test]
    fn delete_tag_from_id_deletes_correct_tag() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let tag = Tag::Custom {
            id: None,
            name: "tag".into(),
        };
        let id = get_tag_id(&insert_tag(&conn, &tag).unwrap()).unwrap();

        // invoke
        delete_tag_from_id(&conn, id).unwrap();

        // check
        assert!(get_tag_from_id(&conn, id).unwrap().is_none());
    }

    #[test]
    fn delete_tag_from_id_fails_if_tag_does_not_exist() {
        // setup
        let (_tmp, conn) = temp_db_init();

        // invoke + check
        assert!(matches!(
            delete_tag_from_id(&conn, 9999).unwrap_err(),
            SqliteDeleteError::NotFound
        ));
    }

    #[test]
    fn delete_tag_from_name_deletes_correct_tag() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let tag = Tag::Custom {
            id: None,
            name: "tag".into(),
        };
        insert_tag(&conn, &tag).unwrap();

        // invoke
        delete_tag_from_name(&conn, "tag").unwrap();

        // check
        assert!(get_tag_from_name(&conn, "tag").unwrap().is_none());
    }

    #[test]
    fn delete_tag_from_name_fails_if_tag_does_not_exist() {
        // setup
        let (_tmp, conn) = temp_db_init();

        // invoke + check
        assert!(matches!(
            delete_tag_from_name(&conn, "tag").unwrap_err(),
            SqliteDeleteError::NotFound
        ));
    }

    #[test]
    fn tag_media_tags_correct_media() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let media = Media {
            id: None,
            path: "path".into(),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };
        let media = insert_media(&conn, &media).unwrap();
        let tag = Tag::Custom {
            id: None,
            name: "tag".into(),
        };
        let tag = insert_tag(&conn, &tag).unwrap();

        // invoke
        tag_media(&conn, &media, &tag).unwrap();

        // check
        assert!(
            get_tags_for_media(&conn, &media)
                .unwrap()
                .unwrap()
                .iter()
                .any(|t| { get_custom_tag_name(t) == "tag" })
        );
    }

    #[test]
    fn tag_media_fails_if_media_does_not_exist() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let media = Media {
            id: None,
            path: "path".into(),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };
        let tag = Tag::Custom {
            id: None,
            name: "tag".into(),
        };
        let tag = insert_tag(&conn, &tag).unwrap();

        // invoke + check
        assert!(matches!(
            tag_media(&conn, &media, &tag).unwrap_err(),
            SqliteInsertError::MissingRelationMember
        ));
    }

    #[test]
    fn tag_media_fails_if_tag_does_not_exist() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let media = Media {
            id: None,
            path: "path".into(),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };
        let media = insert_media(&conn, &media).unwrap();

        // invoke + check
        assert!(matches!(
            tag_media(
                &conn,
                &media,
                &Tag::Custom {
                    id: None,
                    name: "tag".into()
                }
            )
            .unwrap_err(),
            SqliteInsertError::MissingRelationMember
        ));
    }

    #[test]
    fn tag_media_fails_if_media_is_already_tagged_with_tag() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let media = Media {
            id: None,
            path: "path".into(),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };
        let media = insert_media(&conn, &media).unwrap();
        let tag = Tag::Custom {
            id: None,
            name: "tag".into(),
        };
        let tag = insert_tag(&conn, &tag).unwrap();
        tag_media(&conn, &media, &tag).unwrap();

        // invoke + check
        assert!(matches!(
            tag_media(&conn, &media, &tag).unwrap_err(),
            SqliteInsertError::Duplicate
        ));
    }

    #[test]
    fn get_tags_for_media_returns_correct_tags() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let media = Media {
            id: None,
            path: "path".into(),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };
        let media = insert_media(&conn, &media).unwrap();
        let tag = Tag::Custom {
            id: None,
            name: "tag".into(),
        };
        let tag = insert_tag(&conn, &tag).unwrap();
        tag_media(&conn, &media, &tag).unwrap();

        // invoke
        let result = get_tags_for_media(&conn, &media).unwrap();

        // check
        assert!(
            result
                .unwrap()
                .iter()
                .any(|t| { get_custom_tag_name(t) == "tag" })
        );
    }

    #[test]
    fn get_tags_for_media_returns_empty_vec_if_media_has_no_tags() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let media = Media {
            id: None,
            path: "path".into(),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };
        let media = insert_media(&conn, &media).unwrap();

        // invoke
        let result = get_tags_for_media(&conn, &media).unwrap();

        // check
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn get_tags_for_media_returns_none_if_media_does_not_exist() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let media = Media {
            id: None,
            path: "path".into(),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };

        // invoke + check
        assert!(get_tags_for_media(&conn, &media).unwrap().is_none());
    }

    #[test]
    fn get_media_for_tag_returns_correct_media() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let media = Media {
            id: None,
            path: "path".into(),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };
        let media = insert_media(&conn, &media).unwrap();
        let tag = Tag::Custom {
            id: None,
            name: "tag".into(),
        };
        let tag = insert_tag(&conn, &tag).unwrap();
        tag_media(&conn, &media, &tag).unwrap();

        // invoke
        let result = get_media_for_tag(&conn, &tag).unwrap();

        // check
        assert!(result.unwrap().contains(&media));
    }

    #[test]
    fn get_media_for_tag_returns_empty_vec_if_tag_has_no_media() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let tag = Tag::Custom {
            id: None,
            name: "tag".into(),
        };
        let tag = insert_tag(&conn, &tag).unwrap();

        // invoke
        let result = get_media_for_tag(&conn, &tag).unwrap();

        // check
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn get_media_for_tag_returns_none_if_tag_does_not_exist() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let tag = Tag::Custom {
            id: None,
            name: "tag".into(),
        };

        // invoke + check
        assert!(get_media_for_tag(&conn, &tag).unwrap().is_none());
    }

    #[test]
    fn untag_media_correctly_deletes() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let media = Media {
            id: None,
            path: "path".into(),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };
        let media = insert_media(&conn, &media).unwrap();
        let tag = Tag::Custom {
            id: None,
            name: "tag".into(),
        };
        let tag = insert_tag(&conn, &tag).unwrap();
        tag_media(&conn, &media, &tag).unwrap();

        // invoke
        untag_media(&conn, &media, &tag).unwrap();

        // check
        assert!(
            get_tags_for_media(&conn, &media)
                .unwrap()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn untag_media_errors_if_relation_does_not_exist() {
        // setup
        let (_tmp, conn) = temp_db_init();
        let media = Media {
            id: None,
            path: "path".into(),
            media_type: MediaType::Image,
            size_bytes: 0,
            added_at: SystemTime::now(),
        };
        let media = insert_media(&conn, &media).unwrap();
        let tag = Tag::Custom {
            id: None,
            name: "tag".into(),
        };
        let tag = insert_tag(&conn, &tag).unwrap();

        // invoke + check
        assert!(matches!(
            untag_media(&conn, &media, &tag).unwrap_err(),
            SqliteDeleteError::NotFound
        ));
    }
}
