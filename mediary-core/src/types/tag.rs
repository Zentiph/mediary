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

use std::fmt::{Display, Formatter};

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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Tag::Builtin { media_type: mt, .. } => write!(f, "{}", mt),
            Tag::Custom { name, .. } => write!(f, "{}", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_eq_builtin_only_checks_media_type() {
        let tag1 = Tag::Builtin {
            id: None,
            media_type: MediaType::Image,
        };
        let tag2 = Tag::Builtin {
            id: Some(1),
            media_type: MediaType::Image,
        };
        let tag3 = Tag::Builtin {
            id: None,
            media_type: MediaType::Audio,
        };
        assert_eq!(tag1, tag2);
        assert_ne!(tag1, tag3);
    }

    #[test]
    fn test_tag_eq_custom_only_checks_name() {
        let tag1 = Tag::Custom {
            id: None,
            name: "Funny".into(),
        };
        let tag2 = Tag::Custom {
            id: Some(1),
            name: "Funny".into(),
        };
        let tag3 = Tag::Custom {
            id: None,
            name: "Serious".into(),
        };
        assert_eq!(tag1, tag2);
        assert_ne!(tag1, tag3);
    }

    #[test]
    fn test_tag_eq_builtin_and_custom_never_true() {
        let tag1 = Tag::Builtin {
            id: None,
            media_type: MediaType::Image,
        };
        let tag2 = Tag::Custom {
            id: None,
            name: "Image".into(),
        };
        assert_ne!(tag1, tag2);
        assert_ne!(tag2, tag1);
    }

    #[test]
    fn test_tag_display_builtin_uses_media_type_display() {
        let tag = Tag::Builtin {
            id: None,
            media_type: MediaType::Image,
        };
        assert_eq!(tag.to_string(), "Image");
    }

    #[test]
    fn test_tag_display_custom_uses_name() {
        let tag = Tag::Custom {
            id: None,
            name: "Funny".into(),
        };
        assert_eq!(tag.to_string(), "Funny");
    }
}
