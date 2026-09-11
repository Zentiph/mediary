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

use std::{
    error::Error,
    time::{Duration, SystemTime, SystemTimeError, UNIX_EPOCH},
};

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
/// - `Result<SystemTime, Box<dyn Error>>` - The system time.
///
/// # Errors
///
/// If the timestamp is negative.
pub fn unix_timestamp_to_system_time(
    timestamp: i64,
) -> Result<SystemTime, Box<dyn Error + Send + Sync>> {
    if timestamp < 0 {
        Err("Unix timestamp cannot be negative".into())
    } else {
        Ok(SystemTime::UNIX_EPOCH + Duration::from_secs(timestamp as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sys_time_to_unix_roundtrip_preserves_seconds() {
        let sys_time = SystemTime::now();
        let unix_time = system_time_to_unix_timestamp(sys_time).unwrap();
        let sys_time_from_unix =
            unix_timestamp_to_system_time(unix_time).unwrap();
        let unix_time_from_sys =
            system_time_to_unix_timestamp(sys_time_from_unix).unwrap();
        assert_eq!(unix_time, unix_time_from_sys);
    }

    #[test]
    fn sys_time_to_unix_epoch_converts_to_zero() {
        let sys_time = SystemTime::UNIX_EPOCH;
        let unix_time = system_time_to_unix_timestamp(sys_time).unwrap();
        assert_eq!(unix_time, 0);
    }

    #[test]
    fn unix_to_sys_time_errors_on_invalid_timestamp() {
        let result = unix_timestamp_to_system_time(-1);
        assert!(result.is_err());
    }
}
