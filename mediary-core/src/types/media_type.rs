//! # media_type.rs
//!
//! The media type enum.
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

use strum::{AsRefStr, Display, EnumIter, EnumString, IntoStaticStr};

/// A type of media that can be represented in mediary.
///
/// # Variants
///
/// - `#[strum(ascii_case_insensitive)] Image` - An image (e.g. .jpg).
/// - `#[strum(ascii_case_insensitive)] Video` - A video (e.g. .mp4).
/// - `#[strum(ascii_case_insensitive)] Audio` - An audio (e.g. .mp3).
/// - `#[strum(ascii_case_insensitive)] Document` - A document (e.g. .txt).
/// - `#[strum(ascii_case_insensitive)] Unknown` - An unknown/unsupported media
///   type.
/// ```
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    AsRefStr,
    Display,
    EnumIter,
    EnumString,
    IntoStaticStr,
)]
pub enum MediaType {
    #[strum(ascii_case_insensitive)]
    Image,

    #[strum(ascii_case_insensitive)]
    Video,

    #[strum(ascii_case_insensitive)]
    Audio,

    #[strum(ascii_case_insensitive)]
    Document,

    #[strum(ascii_case_insensitive)]
    Unknown,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_media_type_display() {
        assert_eq!(MediaType::Image.to_string(), "Image");
        assert_eq!(MediaType::Video.to_string(), "Video");
        assert_eq!(MediaType::Audio.to_string(), "Audio");
        assert_eq!(MediaType::Document.to_string(), "Document");
        assert_eq!(MediaType::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn test_media_type_from_str_case_insensitive() {
        assert_eq!(MediaType::from_str("image").unwrap(), MediaType::Image);
        assert_eq!(MediaType::from_str("IMAGE").unwrap(), MediaType::Image);
        assert_eq!(MediaType::from_str("Image").unwrap(), MediaType::Image);
        assert_eq!(MediaType::from_str("iMaGe").unwrap(), MediaType::Image);

        assert_eq!(MediaType::from_str("video").unwrap(), MediaType::Video);
        assert_eq!(MediaType::from_str("VIDEO").unwrap(), MediaType::Video);
        assert_eq!(MediaType::from_str("Video").unwrap(), MediaType::Video);
        assert_eq!(MediaType::from_str("vIdEo").unwrap(), MediaType::Video);

        assert_eq!(MediaType::from_str("audio").unwrap(), MediaType::Audio);
        assert_eq!(MediaType::from_str("AUDIO").unwrap(), MediaType::Audio);
        assert_eq!(MediaType::from_str("Audio").unwrap(), MediaType::Audio);
        assert_eq!(MediaType::from_str("aUdIo").unwrap(), MediaType::Audio);

        assert_eq!(
            MediaType::from_str("document").unwrap(),
            MediaType::Document
        );
        assert_eq!(
            MediaType::from_str("DOCUMENT").unwrap(),
            MediaType::Document
        );
        assert_eq!(
            MediaType::from_str("Document").unwrap(),
            MediaType::Document
        );
        assert_eq!(
            MediaType::from_str("dOcUmEnT").unwrap(),
            MediaType::Document
        );

        assert_eq!(MediaType::from_str("unknown").unwrap(), MediaType::Unknown);
        assert_eq!(MediaType::from_str("UNKNOWN").unwrap(), MediaType::Unknown);
        assert_eq!(MediaType::from_str("Unknown").unwrap(), MediaType::Unknown);
        assert_eq!(MediaType::from_str("uNkNoWn").unwrap(), MediaType::Unknown);
    }

    #[test]
    fn test_media_type_from_str_fails_invalid_string() {
        assert!(MediaType::from_str("NotAMediaType").is_err());
    }
}
