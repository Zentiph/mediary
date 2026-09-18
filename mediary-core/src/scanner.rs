//! # scanner.rs
//!
//! Functionality for file-scanning in mediary.
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
    io,
    path::PathBuf,
};

use infer::MatcherType;

use crate::{
    data::database::SqliteInsertError,
    types::{Media, MediaType},
};

/// An error that may occur during a file/directory scan.
///
/// # Variants
///
/// - `Io(io::Error)` - An I/O error.
/// - `Insert(SqliteInsertError)` - A database insert error.
#[derive(Debug)]
pub enum ScanError {
    Io(io::Error),
    Insert(SqliteInsertError),
}
impl Display for ScanError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ScanError::Io(e) => write!(f, "{}", e),
            ScanError::Insert(e) => write!(f, "{}", e),
        }
    }
}
impl Error for ScanError {}
impl From<io::Error> for ScanError {
    fn from(e: io::Error) -> Self {
        ScanError::Io(e)
    }
}
impl From<SqliteInsertError> for ScanError {
    fn from(e: SqliteInsertError) -> Self {
        ScanError::Insert(e)
    }
}

/// The result of a file/directory scan.
///
/// # Fields
///
/// - `inserted` (`Vec<Media>`) - The media items that were inserted.
/// - `skipped_duplicates` (`Vec<PathBuf>`) - The files that were skipped.
/// - `errors` (`Vec<(PathBuf, ScanError)>`) - The files that had errors.
#[derive(Debug)]
pub struct ScanResult {
    pub inserted: Vec<Media>,
    pub skipped_duplicates: Vec<PathBuf>,
    pub errors: Vec<(PathBuf, ScanError)>,
}

/// Convert a mime type to a media type.
///
/// # Arguments
///
/// - `mime` (`Option<infer`) - The mime type.
///
/// # Returns
///
/// - `MediaType` - The media type.
fn mime_to_media_type(mime: Option<infer::Type>) -> MediaType {
    match mime {
        Some(m) => {
            let kind = m.matcher_type();
            match kind {
                MatcherType::Image => MediaType::Image,
                MatcherType::Video => MediaType::Video,
                MatcherType::Audio => MediaType::Audio,
                MatcherType::Book | MatcherType::Doc | MatcherType::Text => {
                    MediaType::Document
                }
                _ => MediaType::Unknown,
            }
        }
        None => MediaType::Unknown,
    }
}
