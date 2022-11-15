// SPDX-License-Identifier: GPL-3.0-or-later

//! Manages fetching and building site icons. Also provides some utility functions relevant to site
//! icons.
//!
//! Typically 'site icon' refers to a website's favicon, but in some cases a different icon may be
//! found.

use image::{imageops::FilterType, DynamicImage, ImageFormat, ImageOutputFormat};
use itertools::Itertools;
use thiserror::Error;
use tokio::time::Instant;
use tracing::{debug, info, span, warn, Level};

use std::{fmt, io::Cursor, path::PathBuf};

use crate::{config::Config, util};

/// Errors that may occur when fetching or building site icons.
#[derive(Error, Debug)]
pub enum SiteIconError {
    /// Occurs when writing the build output fails.
    #[error(transparent)]
    Output(#[from] fmt::Error),

    /// Occurs when no suitable place to cache icons can be found.
    #[error("failed to locate cache dir")]
    CacheDir,

    /// Occurs when writing an icon file to cache fails.
    #[error("failed to write icon @ {1} ({0})")]
    CacheWrite(#[source] image::ImageError, PathBuf),

    /// Occurs when opening a cached icon file fails.
    #[error("failed to read cached icon @ {1} ({0})")]
    CacheRead(#[source] tokio::io::Error, PathBuf),

    /// Occurs when opening a cached icon file fails.
    #[error("failed to decode cached icon @ {1} ({0})")]
    CacheDecode(#[source] image::ImageError, PathBuf),

    /// Occurs when building the [`reqwest::Client`] fails.
    #[error(transparent)]
    HttpClient(#[from] reqwest::Error),

    /// Occurs when loading a website fails.
    #[error("failed to load url: {0}")]
    UrlLoad(String),

    /// Occurs when no suitable icon could be found in a loaded website.
    #[error("failed to find icon for url: {0}")]
    IconNotFound(String),

    /// Occurs when downloading a site icon fails.
    #[error("failed to download icon for url: {1} ({0})")]
    IconRequest(#[source] reqwest::Error, String),

    /// Occurs when decoding a downloaded site icon fails.
    #[error("failed to decode icon for url: {1} ({0})")]
    IconDecode(#[source] image::ImageError, String),

    /// Occurs when re-encoding a processed site icon fails.
    #[error("failed to encode icon for url: {1} ({0})")]
    IconEncode(#[source] image::ImageError, String),
}

/// Generates a unique CSS class for a site icon, based on the provided website URL.
pub fn site_icon_class(url: &str) -> String {
    format!("ico-{}", util::sha1_base32(url.as_bytes()))
}

/// Builds site icons for each URL in the config with the following process:
///
/// 1. Locate, download, and decode a suitable icon in the webpage.
/// 2. Resize and invert (if needed) the decoded icon.
/// 3. Convert the processed icon into a [data URL][1] within a CSS class.
///
/// # Arguments
///
/// * `config` - The config to extract website URLs from.
/// * `size` - The size to resize icons to.
///
/// # Errors
///
/// Returns an error if any step in the process above fails.
///
/// # Returns
///
/// CSS containing classes with [data URL][1] background images. The classname is derived from the
/// original website URL in the config.
///
/// [1]: <https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/Data_URLs>
pub async fn build_site_icons(config: &Config, size: u32) -> Result<String, SiteIconError> {
    let _span = span!(Level::INFO, "site_icons").entered();
    info!("building site icons");
    let sw = Instant::now();

    let mut site_icons = String::default();
    let urls = config
        .pages
        .iter()
        .flat_map(|p| &p.sections)
        .flat_map(|s| &s.links)
        .map(|l| l.url.as_str())
        .collect::<Vec<&str>>();
    let http_client = reqwest::Client::builder()
        .user_agent("newtabgen (looking for icons) github.com/fr33zing/newtabgen")
        .build()?;

    for url in urls.iter().unique().cloned() {
        let mut img = icon(url, &http_client).await?;
        debug!(size, "resizing");
        img = img.resize(size, size, FilterType::Lanczos3);

        if config.theme.invert_low_contrast_icons {
            let brightness = avg_brightness(img.clone());
            let threshold = 0.25;
            if (config.theme.dark && brightness < threshold)
                || (!config.theme.dark && brightness > (1f32 - threshold))
            {
                img.invert();
                debug!(brightness, "inverting icon");
            }
        }

        let mut writer = Cursor::new(Vec::<u8>::new());
        img.write_to(&mut writer, ImageOutputFormat::Png)
            .map_err(|e| SiteIconError::IconDecode(e, url.into()))?;
        let buf = writer.into_inner();
        let bytes = buf.as_slice();
        debug!("generating data url & css class");
        let data_base64 = data_encoding::BASE64.encode(bytes);
        let class = site_icon_class(url);
        debug!("writing output");
        fmt::Write::write_fmt(
            &mut site_icons,
            format_args!(".{class}{{background-image:url(data:image/png;base64,{data_base64})}}"),
        )?;
    }

    debug!(
        elapsed_ms = sw.elapsed().as_millis(),
        "finished building site icons"
    );
    Ok(format!("<style>{site_icons}</style>"))
}

// todo: improve docs
/// Attempts to read an icon for the provided URL from the cache. Otherwise, fetches a remote icon
/// and writes it to the cache.
async fn icon(
    website_url: &str,
    http_client: &reqwest::Client,
) -> Result<DynamicImage, SiteIconError> {
    match icon_cached(website_url).await? {
        Some(icon) => Ok(icon),
        None => {
            let icon = icon_remote(website_url, http_client).await?;
            cache_icon(website_url, &icon)?;
            Ok(icon)
        }
    }
}

// todo: improve docs
/// Writes an icon to the cache.
fn cache_icon(website_url: &str, icon: &DynamicImage) -> Result<(), SiteIconError> {
    let path = util::cache_dir()
        .map_err(|_| SiteIconError::CacheDir)?
        .join("site_icons")
        .join(util::sha1_base32(website_url.as_bytes()));
    debug!(path = path.to_str(), "writing site icon to cache");
    icon.save_with_format(&path, ImageFormat::Png)
        .map_err(|e| SiteIconError::CacheWrite(e, path))?;
    Ok(())
}

/// Attempts to locate, read and decode a cached icon. If the cached icon is older than 1 week, it
/// will be deleted and `None` will be returned.
///
/// Automatic cached icon removal may not work on all platforms (due to the use of
/// std::fs::Metadata). No error will be raised if this is the case.
///
/// # Arguments
///
/// * `website_url` - Url of the website the icon was originally downloaded from.
///
/// # Errors
///
/// Returns an error if any step in the process detailed above fails.
///
/// # Returns
///
/// The read and decoded icon, in its original format.
async fn icon_cached(website_url: &str) -> Result<Option<DynamicImage>, SiteIconError> {
    let path = util::cache_subdir("site_icons")
        .map_err(|_| SiteIconError::CacheDir)?
        .join(util::sha1_base32(website_url.as_bytes()));

    if !path.exists() {
        return Ok(None);
    }

    let expired = 'x: {
        let Ok(metadata) = std::fs::metadata(&path) else { break 'x false };
        let Ok(created) = metadata.created() else { break 'x false };
        let Ok(elapsed) = created.elapsed() else { break 'x false };
        elapsed.as_secs() >= 604800 // One week
    };

    if expired {
        if tokio::fs::remove_file(&path).await.is_err() {
            warn!(
                path = path.to_str(),
                "failed to remove expired icon from cache"
            );
        }
        return Ok(None);
    }

    debug!(path = path.to_str(), "reading cached site icon");
    let icon_bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| SiteIconError::CacheRead(e, path.clone()))?;
    let cursor = Cursor::new(icon_bytes);
    let img = image::io::Reader::new(cursor)
        .with_guessed_format()
        .map_err(|e| SiteIconError::CacheRead(e, path.clone()))?
        .decode()
        .map_err(|e| SiteIconError::CacheDecode(e, path))?;

    Ok(Some(img))
}

/// Locates, downloads, and decodes a suitable icon in the webpage. This process involves sending
/// multiple HTTP requests.
///
/// # Arguments
///
/// * `website_url` - Url of the website, not an icon.
/// * `http_client` - Client to use for sending HTTP requests. Requires a valid user agent.
///
/// # Errors
///
/// Returns an error if any step in the process detailed above fails.
///
/// # Returns
///
/// The downloaded and decoded icon, in its original format.
async fn icon_remote(
    website_url: &str,
    http_client: &reqwest::Client,
) -> Result<DynamicImage, SiteIconError> {
    debug!(website_url, "locating remote site icon");
    let mut icons = site_icons::Icons::new();
    icons
        .load_website(website_url)
        .await
        .map_err(|_| SiteIconError::UrlLoad(website_url.into()))?;
    debug!("choosing site icon");
    let entries = icons.entries().await;
    let icon = {
        // Prefer favicon
        let favicon = entries
            .iter()
            .find(|i| i.url.path().contains("favicon.ico"));
        match favicon {
            Some(i) => i,
            None => entries
                .iter()
                .find(|i| !matches!(i.info, site_icons::IconInfo::SVG))
                .ok_or_else(|| SiteIconError::IconNotFound(website_url.into()))?,
        }
    };
    let icon_url = icon.url.to_string();
    let _span = span!(Level::DEBUG, "individual", icon_url).entered();
    debug!("downloading site icon");
    let icon_bytes = http_client
        .get(icon.url.to_string())
        .send()
        .await
        .map_err(|e| SiteIconError::IconRequest(e, icon.url.clone().into()))?
        .bytes()
        .await
        .map_err(|e| SiteIconError::IconRequest(e, icon.url.clone().into()))?;
    debug!(len = icon_bytes.len(), "reading downloaded site icon");
    let cursor = Cursor::new(icon_bytes);
    let mut reader = image::io::Reader::new(cursor);
    let format = match icon.info.clone() {
        site_icons::IconInfo::PNG { size: _ } => ImageFormat::Png,
        site_icons::IconInfo::JPEG { size: _ } => ImageFormat::Jpeg,
        site_icons::IconInfo::ICO { sizes: _ } => ImageFormat::Ico,
        site_icons::IconInfo::SVG => unreachable!("SVGs should be filtered out"),
    };
    reader.set_format(format);
    let img = reader
        .decode()
        .map_err(|e| SiteIconError::IconDecode(e, website_url.into()))?;
    Ok(img)
}

/// Calculates the average brightness of visible pixels in an image.
///
/// # Returns
///
/// The brightness, ranging from 0 to 1.
fn avg_brightness(img: DynamicImage) -> f32 {
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
