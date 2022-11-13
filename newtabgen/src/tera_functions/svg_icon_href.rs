//! Provides the `svg_icon_href` Tera function.

use std::collections::HashMap;

use tera::{to_value, Error, Result, Value};

use crate::builder::svg_icons::svg_icon_id;

/// SVG icon href filter for use in Tera templates. Converts an icon reference (name and style) to
/// the href of the corresponding SVG symbol.
///
/// # Example
///
/// ```html
/// <svg>
///     <use href="{{ svg_icon_href(icon = page.icon, style = page.icon_style) }}" />
/// </svg>
/// ```
pub struct SvgIconHref;

impl tera::Function for SvgIconHref {
    fn call(&self, args: &HashMap<String, Value>) -> Result<Value> {
        let icon = args
            .get("icon")
            .ok_or_else(|| Error::msg("svg_icon_href requires argument `icon`"))?;

        let style = args
            .get("style")
            .ok_or_else(|| Error::msg("svg_icon_href requires argument `style`"))?;

        match icon.as_str() {
            Some(icon) => match style.as_str() {
                Some(style) => to_value(format!("#{}", svg_icon_id(icon, style)))
                    .map_err(|_| Error::msg("svg_icon_href produced invalid value")),
                None => Err(Error::msg("`style` must be a string")),
            },
            None => Err(Error::msg("`icon` must be a string")),
        }
    }
}
