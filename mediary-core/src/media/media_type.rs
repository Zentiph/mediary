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
