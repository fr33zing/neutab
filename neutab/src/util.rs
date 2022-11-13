//! Utility functions.

use sha1::{Digest, Sha1};

/// Returns a base32-encoded SHA1 hash of the provided bytes.
pub fn sha1_base32(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    let hash_base32 = data_encoding::BASE32HEX_NOPAD.encode(&hash);
    hash_base32.to_lowercase()[..8].into()
}
