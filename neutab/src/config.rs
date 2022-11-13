//! Provides structs that define the expected configuration file.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// The root of the configuration file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Title of the new tab page.
    #[serde(default = "Config::default_title")]
    pub title: String,

    /// Theming preferences.
    #[serde(default)]
    pub theme: Theme,

    /// New tab page content.
    #[serde(default)]
    pub pages: Vec<Page>,
}

impl Config {
    /// Default value for `Config.title`
    fn default_title() -> String {
        "New Tab".into()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            title: "New Tab".into(),
            theme: Default::default(),
            pages: Default::default(),
        }
    }
}

/// Theming preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    /// Indicates if the template's dark theme should be used.
    #[serde(default = "Theme::default_dark")]
    pub dark: bool,

    /// Indicates if icons with low contrast against the template's background should be inverted
    /// preemptively.
    #[serde(default = "Theme::default_invert_low_contrast_icons")]
    pub invert_low_contrast_icons: bool,

    /// Font family, to be passed directly to the css property `font-family`.
    #[serde(default = "Theme::default_font_family")]
    pub font_family: String,

    /// Font size, in pixels.
    #[serde(default = "Theme::default_font_size")]
    pub font_size: u16,

    /// Any other values provided.
    #[serde(default, flatten)]
    pub custom: HashMap<String, tera::Value>,
}

impl Theme {
    /// Default value for `Theme.dark`
    fn default_dark() -> bool {
        true
    }

    /// Default value for `Theme.invert_low_contrast_icons`
    fn default_invert_low_contrast_icons() -> bool {
        true
    }

    /// Default value for `Theme.font_family`
    fn default_font_family() -> String {
        "sans-serif".into()
    }

    /// Default value for `Theme.font_size`
    fn default_font_size() -> u16 {
        14
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            dark: Theme::default_dark(),
            invert_low_contrast_icons: Theme::default_invert_low_contrast_icons(),
            font_family: Theme::default_font_family(),
            font_size: Theme::default_font_size(),
            custom: Default::default(),
        }
    }
}

/// New tab page content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    /// Page name.
    pub name: String,

    /// Name of the icon to use for the page.
    ///
    /// See: <https://marella.me/material-design-icons/demo/font/>
    #[serde(default = "Page::default_icon")]
    pub icon: String,

    /// Style of the icon to use for the page.
    ///
    /// Accepted values: "filled" | "outlined" | "round" | "sharp" | "two-tone"
    #[serde(default = "Page::default_icon_style")]
    pub icon_style: String,

    /// Sections of a page, containing links.
    #[serde(default)]
    pub sections: Vec<Section>,
}

impl Page {
    /// Default value for `Page.icon`
    fn default_icon() -> String {
        "image_not_supported".into()
    }

    /// Default value for `Page.icon_style`
    fn default_icon_style() -> String {
        "outlined".into()
    }
}

/// Sections of a page, containing links.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    /// Section name.
    pub name: String,

    /// Links in the sections.
    #[serde(default)]
    pub links: Vec<Link>,
}

/// A link to a website.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    /// Display name.
    pub name: String,

    /// Website URL.
    pub url: String,
}
