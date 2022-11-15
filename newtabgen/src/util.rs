// SPDX-License-Identifier: GPL-3.0-or-later

//! Utility functions.

use std::{env, fs, io, path::PathBuf};

use sha1::{Digest, Sha1};

/// Finds a suitable cache directory.
fn cache_dir_base() -> Result<PathBuf, io::Error> {
    let dir = match dirs::cache_dir() {
        Some(d) => d,
        None => env::current_dir()?,
    };
    Ok(dir.join("newtabgen"))
}

/// Finds a suitable cache directory. The cache directory will be created if needed.
///
/// # Errors
///
/// Returns an error if a suitable cache directory cannot be found.
pub fn cache_dir() -> Result<PathBuf, io::Error> {
    let cache_dir = cache_dir_base()?;
    fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

/// Finds a suitable cache directory. The cache directory and the requested subdirectory will be
/// created if needed.
///
/// # Errors
///
/// Returns an error if a suitable cache directory cannot be found.
pub fn cache_subdir(subdir: &str) -> Result<PathBuf, io::Error> {
    let cache_dir = cache_dir_base()?.join(subdir);
    fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

/// Returns a base32-encoded SHA1 hash of the provided bytes.
pub fn sha1_base32(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    let hash_base32 = data_encoding::BASE32HEX_NOPAD.encode(&hash);
    hash_base32.to_lowercase()[..8].into()
}
