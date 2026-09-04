//! # tag.rs
//!
//! The tag struct.
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

use std::fmt::Display;

use crate::types::media_type::MediaType;

/// A tag that can be applied to media files.
///
/// # Variants
///
/// - `Builtin { id, media_type }` - A builtin tag matching the possible media types.
/// - `Custom { id, name }` - A user-defined tag.
#[derive(Debug, Clone)]
pub enum Tag {
    Builtin {
        id: Option<i64>,
        media_type: MediaType,
    },
    Custom {
        id: Option<i64>,
        name: String,
        // TODO:
        // color: Option<(u8, u8, u8)>,
    },
}

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Tag::Builtin {
                    media_type: mt1, ..
                },
                Tag::Builtin {
                    media_type: mt2, ..
                },
            ) => mt1 == mt2,
            (Tag::Custom { name: n1, .. }, Tag::Custom { name: n2, .. }) => {
                n1 == n2
            }
            _ => false,
        }
    }
}
impl Eq for Tag {}

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tag::Builtin { media_type: mt, .. } => write!(f, "{}", mt),
            Tag::Custom { name, .. } => write!(f, "{}", name),
        }
    }
}

#[cfg(test)]
mod tests {}
