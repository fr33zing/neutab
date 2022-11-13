//! Manages fetching and building site icons. Also provides some utility functions relevant to site
//! icons.
//!
//! Typically 'site icon' refers to a website's favicon, but in some cases a different icon may be
//! found.

use base64ct::Encoding;
use image::{imageops::FilterType, DynamicImage, ImageFormat, ImageOutputFormat};
use itertools::Itertools;
use thiserror::Error;
use tokio::time::Instant;
use tracing::{debug, info, span, Level};

use std::{fmt, io::Cursor};

use crate::{config::Config, util};

/// Errors that may occur when fetching or building site icons.
#[derive(Error, Debug)]
pub enum SiteIconError {
    /// Occurs when writing the build output fails.
    #[error(transparent)]
    Output(#[from] fmt::Error),

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
        .user_agent("neutab (looking for icons) github.com/fr33zing/neutab")
        .build()?;

    for url in urls.iter().unique().cloned() {
        debug!(url, "locating site icon");

        let mut icons = site_icons::Icons::new();
        icons
            .load_website(url)
            .await
            .map_err(|_| SiteIconError::UrlLoad(url.into()))?;

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
                    .ok_or_else(|| SiteIconError::IconNotFound(url.into()))?,
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

        debug!(size, "resizing");

        let mut img = reader
            .decode()
            .map_err(|e| SiteIconError::IconDecode(e, url.into()))?
            .resize(size, size, FilterType::Lanczos3);

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

        let data_base64 = base64ct::Base64::encode_string(bytes);
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
