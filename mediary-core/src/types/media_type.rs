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

use strum::{AsRefStr, Display, EnumString, IntoStaticStr};

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
mod tests {}
