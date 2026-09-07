//! # app_data.rs
//!
//! Setup/fetching functionality for mediary's app data directory.
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

use std::{fs, io, path::PathBuf};

use directories::ProjectDirs;

/// Get the app data directory for mediary.
///
/// # Returns
///
/// - `io::Result<PathBuf>` - Mediary's app data directory.
///
/// # Errors
///
/// If the app data directory could not be determined.
/// If the app data directory could not be created.
fn get_app_data_dir() -> io::Result<PathBuf> {
    let app_data_dir = ProjectDirs::from("com", "mediary-app", "mediary")
        .map(|proj_dirs| proj_dirs.data_local_dir().to_path_buf())
        .expect("Could not determine app data directory");
    fs::create_dir_all(&app_data_dir)?;
    Ok(app_data_dir)
}

/// Get the path to a file in mediary's app data directory.
///
/// # Arguments
///
/// - `file_name` (`&str`) - The name of the file.
///
/// # Returns
///
/// - `io::Result<PathBuf>` - The path to the file.
///
/// # Errors
///
/// If the app data directory could not be determined.
/// If the app data directory could not be created.
pub fn get_app_data_file(file_name: &str) -> io::Result<PathBuf> {
    let app_data_dir = get_app_data_dir()?;
    Ok(app_data_dir.join(file_name))
}
