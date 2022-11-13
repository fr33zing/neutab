pub(crate) mod site_icons;
pub(crate) mod svg_icons;

use serde::Serialize;
use std::{
    io::Write,
    str::{from_utf8, Utf8Error},
};
use tera::{Context, Tera};
use thiserror::Error;
use tokio::time::Instant;
use tracing::{debug, info, span, Level};

use crate::{
    config::Config,
    resources::{ResourceError, Resources},
    tera_filters, tera_functions,
};

use self::site_icons::SiteIconError;

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("failed to load resource ({0})")]
    Resource(#[from] ResourceError),

    #[error("failed to render template ({0})")]
    Template(#[from] tera::Error),

    #[error("failed to compile scss ({0})")]
    ScssCompile(#[from] rsass::Error),

    #[error("failed to encode to UTF-8 ({0})")]
    EncodeUtf8(#[from] Utf8Error),

    #[error("failed to build site icons ({0})")]
    SiteIcon(#[from] SiteIconError),
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
    let config = resources.config()?;
    let src_html = resources.html()?;
    let src_scss = resources.scss()?;

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
    let out_site_icons = site_icons::build_site_icons(&config, 24).await?;
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
