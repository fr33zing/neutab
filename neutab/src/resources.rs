use resource::{resource, resource_str};
use tracing::{event, Level};

use std::{fs, path::PathBuf, str};

use crate::config::Config;

#[derive(thiserror::Error, Debug)]
pub enum ResourceError {
    #[error("failed to read override file for resource: {0}")]
    Override(String),

    #[error("failed to preprocess resource: {0} ({1})")]
    Preprocess(String, String),

    #[error("failed to parse resource: {0}")]
    Resource(String),

    #[error("UTF-8 conversion failed for resource: {0}")]
    Utf8(String),

    #[error("failed to parse url: {1} ({0})")]
    UrlParse(#[source] url::ParseError, String),

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

#[derive(Clone, Copy)]
pub struct ScssOptions<'a> {
    pub mobile: bool,
    pub dark: bool,
    pub accent: &'a str,
    pub font_family: &'a str,
    pub font_size: u16,
}

pub struct Resources {
    pub config: Option<PathBuf>,
    pub css: Option<PathBuf>,
    pub html: Option<PathBuf>,
}

impl Resources {
    pub fn config(&self) -> Result<Config, ResourceError> {
        let src = match &self.config {
            Some(file) => load_override_raw("config".into(), file),
            None => Ok(resource_str!("example/example.json").to_string()),
        }?;
        let config = serde_any::from_str_any::<Config>(src.as_str())
            .map_err(|_| ResourceError::Resource("config".into()))?;
        event!(Level::DEBUG, "parsed config");
        Ok(config)
    }

    pub fn scss(&self) -> Result<String, ResourceError> {
        match &self.css {
            Some(file) => load_override("css".into(), file, |src: &[u8]| {
                utf8(src.to_vec(), "html".into())
            }),
            None => resource!("res/styles.scss", |src: &[u8]| utf8(
                src.to_vec(),
                "html".into()
            )),
        }
    }

    pub fn html(&self) -> Result<String, ResourceError> {
        match &self.html {
            Some(file) => load_override_raw("html".into(), file),
            None => resource!("res/index.html", |src: &[u8]| utf8(
                src.to_vec(),
                "html".into()
            )),
        }
    }
}

fn utf8(v: Vec<u8>, resource_name: String) -> Result<String, ResourceError> {
    Ok(str::from_utf8(v.as_slice())
        .map_err(|_| ResourceError::Utf8(resource_name))?
        .to_string())
}

fn load_override(
    resource_name: String,
    file: &PathBuf,
    preprocessor: impl Fn(&[u8]) -> Result<String, ResourceError>,
) -> Result<String, ResourceError> {
    let src = fs::read(file).map_err(|_| ResourceError::Override(resource_name))?;
    preprocessor(src.as_slice())
}

fn load_override_raw(resource_name: String, file: &PathBuf) -> Result<String, ResourceError> {
    let src = fs::read(file).map_err(|_| ResourceError::Override(resource_name.clone()))?;
    utf8(src, resource_name)
}
