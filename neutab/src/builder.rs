use base64ct::Encoding;
use image::{imageops::FilterType, ImageFormat, ImageOutputFormat};
use itertools::Itertools;
use serde::Serialize;
use std::{
    io::{Cursor, Write},
    str::{from_utf8, Utf8Error},
};
use tera::{Context, Tera};
use thiserror::Error;
use tokio::time::Instant;
use tracing::{debug, info, span, Level};

use crate::{
    config::Config,
    resources::{ResourceError, Resources},
    tera_filters, tera_functions, util,
};

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("failed to load resource ({0})")]
    Resource(#[from] ResourceError),

    #[error("failed to render template")]
    Template(#[from] tera::Error),

    #[error("failed to compile scss: {0}")]
    ScssCompile(#[from] rsass::Error),

    #[error("failed to encode to UTF-8: {0}")]
    EncodeUtf8(#[from] Utf8Error),

    #[error("failed to load url: {0}")]
    UrlLoad(String),

    #[error("failed to find icon for url: {0}")]
    IconNotFound(String),

    #[error("failed to download icon for url: {1} ({0})")]
    IconRequest(#[source] reqwest::Error, String),

    #[error("failed to decode icon for url: {1} ({0})")]
    IconDecode(#[source] image::ImageError, String),

    #[error("failed to encode icon for url: {1} ({0})")]
    IconEncode(#[source] image::ImageError, String),
}

#[derive(Serialize)]
pub struct BuildContext {
    config: Config,
    mobile: bool,
}

pub async fn build(
    resources: Resources,
    mobile: bool,
    output: &mut impl Write,
) -> Result<(), BuildError> {
    let _span = span!(Level::INFO, "build").entered();

    // Load and preprocess resources
    let config = resources.config().map_err(BuildError::Resource)?;
    let src_html = resources.html().map_err(BuildError::Resource)?;
    let src_scss = resources.scss().map_err(BuildError::Resource)?;

    // Setup tera
    let mut tera = Tera::default();
    tera.register_filter("hash", tera_filters::Hash);
    tera.register_filter("site_icon", tera_filters::SiteIcon);
    tera.register_function("len", tera_functions::Len);
    tera.register_function(
        "count_links_in_page",
        tera_functions::CountLinksInPage(config.clone()),
    );

    let mut context = Context::new();
    context.insert("config", &config);
    context.insert("mobile", &mobile);

    // Build site icon css styles
    let out_site_icons = build_site_icons(&config, 24).await?;
    context.insert("site_icons", &out_site_icons);

    // Build css
    let out_css = build_css(src_scss, &mut tera, &context)?;
    context.insert("styles", &out_css);

    // Build html
    let out_html = build_html(src_html, &mut tera, &context)?;

    output
        .write_all(out_html.as_slice())
        .expect("stdout not worky");

    Ok(())
}

async fn build_site_icons(config: &Config, size: u32) -> Result<String, BuildError> {
    let _span = span!(Level::INFO, "site_icons").entered();
    info!("building site icons");
    let sw = Instant::now();

    let mut output = String::default();
    let urls = config
        .pages
        .iter()
        .flat_map(|p| &p.sections)
        .flat_map(|s| &s.links)
        .map(|l| l.url.as_str())
        .collect::<Vec<&str>>();
    let http_client = reqwest::Client::builder()
        .user_agent("haven't decided on a name yet, sorry")
        .build()
        .expect("failed to build http client");

    for url in urls.iter().unique().cloned() {
        debug!(url, "locating site icon");

        let mut icons = site_icons::Icons::new();
        icons
            .load_website(url)
            .await
            .map_err(|_| BuildError::UrlLoad(url.into()))?;

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
                    .ok_or_else(|| BuildError::IconNotFound(url.into()))?,
            }
        };
        let icon_url = icon.url.to_string();

        let _span = span!(Level::DEBUG, "individual", icon_url).entered();
        debug!("downloading site icon");

        let icon_bytes = http_client
            .get(icon.url.to_string())
            .send()
            .await
            .map_err(|e| BuildError::IconRequest(e, icon.url.clone().into()))?
            .bytes()
            .await
            .map_err(|e| BuildError::IconRequest(e, icon.url.clone().into()))?;

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
            .map_err(|e| BuildError::IconDecode(e, url.into()))?
            .resize(size, size, FilterType::Lanczos3);

        if config.theme.invert_low_contrast_icons {
            let brightness = util::avg_brightness(img.clone());
            let threshold = 0.25;
            if (config.theme.dark && brightness < threshold)
                || (!config.theme.dark && brightness > (1f32 - threshold))
            {
                img.invert();
                tracing::warn!(brightness, icon_url, "inverting icon");
            }
        }

        let mut writer = Cursor::new(Vec::<u8>::new());
        img.write_to(&mut writer, ImageOutputFormat::Png)
            .map_err(|e| BuildError::IconDecode(e, url.into()))?;
        let buf = writer.into_inner();
        let bytes = buf.as_slice();

        debug!("generating data url & css class");

        let data_base64 = base64ct::Base64::encode_string(bytes);
        let class = util::site_icon_class(url)
            .unwrap_or_else(|_| panic!("failed to get site icon class for url: '{url}'"));

        debug!("writing output");

        std::fmt::Write::write_fmt(
            &mut output,
            format_args!(".{class}{{background-image:url(data:image/png;base64,{data_base64})}}"),
        )
        .unwrap_or_else(|_| unreachable!());
    }

    debug!(
        elapsed_ms = sw.elapsed().as_millis(),
        "finished building site icons"
    );
    Ok(output)
}

fn build_css(src_scss: String, tera: &mut Tera, ctx: &Context) -> Result<String, BuildError> {
    let _span = span!(Level::INFO, "css").entered();
    info!("building css");
    let sw = Instant::now();

    let format = rsass::output::Format {
        style: rsass::output::Style::Compressed,
        ..Default::default()
    };
    let rendered = tera
        .render_str(src_scss.as_str(), ctx)
        .map_err(BuildError::Template)?;
    let compiled =
        rsass::compile_scss(rendered.as_bytes(), format).map_err(BuildError::ScssCompile)?;
    let encoded = from_utf8(compiled.as_slice()).map_err(BuildError::EncodeUtf8)?;

    debug!(
        elapsed_ms = sw.elapsed().as_millis(),
        "finished building css"
    );
    Ok(encoded.to_string())
}

fn build_html(src_html: String, tera: &mut Tera, ctx: &Context) -> Result<Vec<u8>, BuildError> {
    let _span = span!(Level::INFO, "html").entered();
    info!("building html");
    let sw = Instant::now();

    let rendered = tera
        .render_str(src_html.as_str(), ctx)
        .map_err(BuildError::Template)?;
    let cfg = &minify_html::Cfg::default();
    let minified = minify_html::minify(rendered.as_bytes(), cfg);

    debug!(
        elapsed_ms = sw.elapsed().as_millis(),
        "finished building html"
    );
    Ok(minified)
}

// fn build_css()
