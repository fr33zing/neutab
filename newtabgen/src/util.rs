//! Utility functions.

use std::{env, fs, io, path::PathBuf};

use sha1::{Digest, Sha1};

/// Finds a suitable cache directory. The directory will be created if needed.
///
/// # Errors
///
/// Returns an error if a suitable cache directory cannot be found.
pub fn cache_dir() -> Result<PathBuf, io::Error> {
    let cache_dir = match dirs::cache_dir() {
        Some(d) => Ok(d),
        None => env::current_dir(),
    }?
    .join("newtabgen");

    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir)?;
    }

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
