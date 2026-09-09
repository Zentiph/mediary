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

use std::path::Path;

use rusqlite::Connection;
use strum::IntoEnumIterator;

use crate::{data::app_data::get_app_data_file, types::MediaType};

const DB_NAME: &str = "mediary.db";
const DB_SCHEMA: &str = include_str!("schema.sql");

// TODO: REPLACE ALL PANICS/.expect()s WITH PROPER ERROR PROPAGATION

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
            "INSERT OR IGNORE INTO tags (name, is_builtin) VALUES (?1, TRUE)",
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

#[cfg(test)]
mod tests {}
