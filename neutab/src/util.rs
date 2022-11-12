use image::DynamicImage;
use sha1::{Digest, Sha1};

pub fn sha1_base32(bytes: &[u8]) -> Result<String, base16ct::Error> {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    let hash_base32 = data_encoding::BASE32HEX_NOPAD.encode(&hash);
    Ok(hash_base32.to_lowercase()[..8].into())
}

pub fn site_icon_class(url: &str) -> Result<String, base16ct::Error> {
    let url_hash = sha1_base32(url.as_bytes())?;
    Ok(format!("ico-{url_hash}"))
}

/// Calculates the average brightness of visible pixels in an image.
///
/// # Returns
///
/// The brightness, ranging from 0 to 1.
pub fn avg_brightness(img: DynamicImage) -> f32 {
    let rgba = img.into_rgba8();
    let avgs = rgba.pixels().filter_map(|p| {
        if p[3] > 32 {
            Some(p[0] / 3 + p[1] / 3 + p[2] / 3)
        } else {
            None
        }
    });
    let len = avgs.clone().count() as f32;
    avgs.fold(0f32, |avg, val| avg + f32::from(val) / len / 255f32)
}
