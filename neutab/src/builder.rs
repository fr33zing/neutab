//! Manages the full build process.

pub(crate) mod site_icons;
pub(crate) mod svg_icons;

use std::{
    io::{self, Write},
    str::{from_utf8, Utf8Error},
};
use tera::{Context, Tera};
use thiserror::Error;
use tokio::time::Instant;
use tracing::{debug, info, span, Level};

use crate::{
    resources::{ResourceError, Resources},
    tera_filters, tera_functions,
};

use self::{site_icons::SiteIconError, svg_icons::SvgIconError};

/// Errors that may occur when building a new tab page.
#[derive(Error, Debug)]
pub enum BuildError {
    /// Occurs when writing the built output fails.
    #[error(transparent)]
    Output(#[from] io::Error),

    /// Occurs when encoding to UTF-8 fails.
    #[error(transparent)]
    EncodeUtf8(#[from] Utf8Error),

    /// Occurs when the resource loader encounters an error.
    #[error("failed to load resource ({0})")]
    Resource(#[from] ResourceError),

    /// Occurs when the template renderer encounters an error.
    #[error("failed to render template ({0})")]
    Template(#[from] tera::Error),

    /// Occurs when the SCSS compiler encounters an error.
    #[error("failed to compile scss ({0})")]
    ScssCompile(#[from] rsass::Error),

    /// Occurs when building the site icons fails.
    #[error("failed to build site icons ({0})")]
    SiteIcon(#[from] SiteIconError),

    /// Occurs when building the svg icons fails.
    #[error("failed to build svg icons ({0})")]
    SvgIcon(#[from] SvgIconError),
}

/// Builds a new tab page.
///
/// # Arguments
///
/// * `resources` - External [Resources] used to build the new tab page.
/// * `output` - Where to write the build output. Generally, stdout or a file.
///
/// # Errors
///
/// Returns an error if any step in the build process fails.
///
/// # Example
///
/// Assuming resource paths are read from command line arguments:
///
/// ```rust
/// let resources = Resources {
///     config: args.config.clone(),
///     css: args.scss.clone(),
///     html: args.html.clone(),
/// };
///
/// let mut output = io::stdout().lock();
/// builder::build(resources, false, &mut output).await?;
/// ```
pub async fn build(resources: Resources, output: &mut impl Write) -> Result<(), BuildError> {
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
    tera.register_function("svg_icon_href", tera_functions::SvgIconHref);
    tera.register_function(
        "count_links_in_page",
        tera_functions::CountLinksInPage(config.clone()),
    );

    let mut context = Context::new();
    context.insert("config", &config);

    // Build svg icon svg symbol defs
    let out_svg_icons = svg_icons::build_svg_icons(&config)?;
    context.insert("include_svg_icons", &out_svg_icons);

    // Build site icon css styles
    let out_site_icons = site_icons::build_site_icons(&config, 24).await?;
    context.insert("include_site_icons", &out_site_icons);

    // Build css
    let out_css = build_css(src_scss, &mut tera, &context)?;
    context.insert("include_styles", &out_css);

    // Build html
    let out_html = build_html(src_html, &mut tera, &context)?;

    output.write_all(out_html.as_slice())?;
    Ok(())
}

/// Renders the SCSS template, then compiles the rendered SCSS into minified CSS.
///
/// # Arguments
///
/// * `src_scss` - The SCSS template to compile.
/// * `tera` - The template renderer to use.
/// * `ctx` - The build context, used to provide information to the template.
///
/// # Errors
///
/// Returns an error if rendering the template, compiling the rendered SCSS, or encoding the
/// compiled CSS into UTF-8 fails.
///
/// # Returns
///
/// Minified CSS.
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
    Ok(format!("<style>{encoded}</style>"))
}

/// Renders the HTML template, then minifies the rendered HTML.
///
/// # Arguments
///
/// * `src_html` - The HTML template to compile.
/// * `tera` - The template renderer to use.
/// * `ctx` - The build context, used to provide information to the template.
///
/// # Errors
///
/// Returns an error if rendering the template fails.
///
/// # Returns
///
/// Minified HTML.
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
