use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "Config::default_title")]
    pub title: String,

    #[serde(default)]
    pub theme: Theme,

    #[serde(default)]
    pub build: Build,

    #[serde(default)]
    pub pages: Vec<Page>,
}

impl Config {
    fn default_title() -> String {
        "New Tab".into()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            title: "New Tab".into(),
            theme: Default::default(),
            build: Default::default(),
            pages: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    #[serde(default = "Theme::default_dark")]
    pub dark: bool,

    #[serde(default = "Theme::default_invert_low_contrast_icons")]
    pub invert_low_contrast_icons: bool,

    #[serde(default = "Theme::default_font_family")]
    pub font_family: String,

    #[serde(default = "Theme::default_font_size")]
    pub font_size: u16,

    #[serde(default, flatten)]
    pub custom: HashMap<String, tera::Value>,
}

impl Theme {
    fn default_dark() -> bool {
        true
    }

    fn default_invert_low_contrast_icons() -> bool {
        true
    }

    fn default_font_family() -> String {
        "sans-serif".into()
    }

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Build {
    pub desktop: bool,
    pub mobile: bool,
}

impl Default for Build {
    fn default() -> Self {
        Self {
            desktop: true,
            mobile: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub name: String,

    #[serde(default = "Page::default_icon")]
    pub icon: String,

    #[serde(default = "Page::default_icon_style")]
    pub icon_style: String,

    #[serde(default)]
    pub sections: Vec<Section>,
}

impl Page {
    fn default_icon() -> String {
        "image_not_supported".into()
    }

    fn default_icon_style() -> String {
        "outlined".into()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub name: String,

    #[serde(default)]
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub name: String,
    pub url: String,
}
