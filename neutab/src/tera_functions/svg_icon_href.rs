use std::collections::HashMap;

use tera::{to_value, Error, Result, Value};

use crate::builder::svg_icons::svg_icon_id;

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
