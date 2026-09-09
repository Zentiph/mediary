//! # conversion.rs
//!
//! Functionality for converting between data types used in mediary.
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

use std::time::{Duration, SystemTime, SystemTimeError, UNIX_EPOCH};

/// Convert a system time to a Unix timestamp.
///
/// # Arguments
///
/// - `time` (`SystemTime`) - The system time to convert.
///
/// # Returns
///
/// - `Result<i64, SystemTimeError>` - The Unix timestamp.
///
/// # Errors
///
/// If the time could not be converted (e.g. it was before Unix epoch).
pub fn system_time_to_unix_timestamp(
    time: SystemTime,
) -> Result<i64, SystemTimeError> {
    time.duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64)
}

/// Convert a Unix timestamp to a system time.
///
/// # Arguments
///
/// - `timestamp` (`i64`) - The Unix timestamp.
///
/// # Returns
///
/// - `SystemTime` - The system time.
pub fn unix_timestamp_to_system_time(timestamp: i64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(timestamp as u64)
}

#[cfg(test)]
mod tests {}
