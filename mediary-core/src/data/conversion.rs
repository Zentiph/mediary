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
    fn system_time_to_unix_timestamp_using_now_is_greater_than_zero() {
        // setup
        let sys_time = SystemTime::now();

        // invoke
        let unix_time = system_time_to_unix_timestamp(sys_time).unwrap();

        // check
        assert!(unix_time > 0);
    }

    #[test]
    fn system_time_to_unix_timestamp_converts_epoch_to_zero() {
        // setup
        let sys_time = SystemTime::UNIX_EPOCH;

        // invoke
        let unix_time = system_time_to_unix_timestamp(sys_time).unwrap();

        // check
        assert_eq!(unix_time, 0);
    }

    #[test]
    fn unix_timestamp_to_system_time_using_now_is_ok() {
        // setup
        let sys_time = SystemTime::now();

        // invoke
        let unix_time = system_time_to_unix_timestamp(sys_time).unwrap();
        let result = unix_timestamp_to_system_time(unix_time).unwrap();

        // check
        assert_eq!(result, sys_time);
    }

    #[test]
    fn unix_timestamp_to_system_time_converts_zero_to_epoch() {
        // invoke
        let result = unix_timestamp_to_system_time(0);

        // check
        assert_eq!(result.unwrap(), SystemTime::UNIX_EPOCH);
    }

    #[test]
    fn unix_timestamp_to_system_time_errors_on_negative_timestamp() {
        // invoke
        let result = unix_timestamp_to_system_time(-1);

        // check
        assert!(result.is_err());
    }

    #[test]
    fn system_time_to_unix_timestamp_roundtrip_preserves_seconds() {
        // setup
        let sys_time = SystemTime::now();

        // invoke
        let unix_time = system_time_to_unix_timestamp(sys_time).unwrap();
        let sys_time_from_unix =
            unix_timestamp_to_system_time(unix_time).unwrap();
        let unix_time_from_sys =
            system_time_to_unix_timestamp(sys_time_from_unix).unwrap();

        // check
        assert_eq!(unix_time, unix_time_from_sys);
    }
}
