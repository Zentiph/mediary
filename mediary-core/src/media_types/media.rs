//! # media
//!
//! The media struct.
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

use std::{path::PathBuf, time::SystemTime};

use crate::media_types::media_type::MediaType;

/// A representation of a media file stored in mediary.
///
/// # Fields
///
/// - `id` (`Option<i64>`) - The database ID of the media, if it exists.
/// - `path` (`PathBuf`) - The path to the file where the media is stored.
/// - `media_type` (`MediaType`) - The type of this media.
/// - `size_bytes` (`u64`) - The size of this media in bytes.
/// - `added_at` (`SystemTime`) - The time this media was added.
#[derive(Clone, Debug)]
pub struct Media {
    pub id: Option<i64>,
    pub path: PathBuf,
    pub media_type: MediaType,
    pub size_bytes: u64,
    pub added_at: SystemTime,
    // pub thumbnail_path: Option<PathBuf>,
}

impl PartialEq for Media {
    fn eq(&self, other: &Self) -> bool {
        // same path, must be the same file
        self.path == other.path
    }
}
impl Eq for Media {}

#[cfg(test)]
mod tests {}
