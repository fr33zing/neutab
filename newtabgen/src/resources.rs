// SPDX-License-Identifier: GPL-3.0-or-later

//! Handles loading resources needed for building a new tab page.

use resource::{resource, resource_str};
use tracing::{event, Level};

use std::{fs, path::PathBuf, str};

use crate::config::Config;

/// Errors that may occur when loading resources.
#[derive(thiserror::Error, Debug)]
pub enum ResourceError {
    /// Occurs when loading an override file fails.
    #[error("failed to read override file for resource: {0}")]
    Override(String),

    /// Occurs when parsing a resource fails.
    #[error("failed to parse resource: {0}")]
    Parse(String),

    /// Occurs when encoding a resource to UTF-8 fails.
    #[error("UTF-8 conversion failed for resource: {0}")]
    Utf8(String),
}

/// Contains paths to resource files.
pub struct Resources {
    /// Configuration file path.
    pub config: Option<PathBuf>,

    /// SCSS template path.
    pub scss: Option<PathBuf>,

    /// HTML template path.
    pub html: Option<PathBuf>,
}

impl Resources {
    /// Loads the configuration file.
    ///
    /// # Errors
    ///
    /// Returns an error if loading or parsing the [`Config`] fails.
    pub fn config(&self) -> Result<Config, ResourceError> {
        let src = match &self.config {
            Some(file) => load_override("config".into(), file),
            None => Ok(resource_str!("example/example.json").to_string()),
        }?;
        let config = serde_any::from_str_any::<Config>(src.as_str())
            .map_err(|_| ResourceError::Parse("config".into()))?;
        event!(Level::DEBUG, "parsed config");
        Ok(config)
    }

    /// Loads the SCSS template.
    ///
    /// # Errors
    ///
    /// Returns an error if loading the file fails.
    pub fn scss(&self) -> Result<String, ResourceError> {
        match &self.scss {
            Some(file) => load_override("css".into(), file),
            None => resource!("res/styles.scss", |src: &[u8]| utf8(
                src.to_vec(),
                "html".into()
            )),
        }
    }

    /// Loads the HTML template.
    ///
    /// # Errors
    ///
    /// Returns an error if loading the file fails.
    pub fn html(&self) -> Result<String, ResourceError> {
        match &self.html {
            Some(file) => load_override("html".into(), file),
            None => resource!("res/index.html", |src: &[u8]| utf8(
                src.to_vec(),
                "html".into()
            )),
        }
    }
}

/// Attempts to encode the provided bytes to UTF-8.
fn utf8(v: Vec<u8>, resource_name: String) -> Result<String, ResourceError> {
    Ok(str::from_utf8(v.as_slice())
        .map_err(|_| ResourceError::Utf8(resource_name))?
        .to_string())
}

/// Attempts to load an override file.
fn load_override(resource_name: String, file: &PathBuf) -> Result<String, ResourceError> {
    let src = fs::read(file).map_err(|_| ResourceError::Override(resource_name.clone()))?;
    utf8(src, resource_name)
}
