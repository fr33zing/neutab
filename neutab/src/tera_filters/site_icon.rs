use std::collections::HashMap;

use tera::{to_value, Filter};

use crate::builder;

pub struct SiteIcon;

impl Filter for SiteIcon {
    fn filter(
        &self,
        value: &tera::Value,
        _args: &HashMap<String, tera::Value>,
    ) -> tera::Result<tera::Value> {
        match value.as_str() {
            Some(url) => {
                let output = builder::site_icons::site_icon_class(url).map_err(|_| {
                    tera::Error::msg(format!("failed to get site icon class for url: '{url}'"))
                })?;
                to_value(output).map_err(|_| {
                    tera::Error::msg(
                        "formatting site icon class produced invalid value: '{output}'",
                    )
                })
            }
            None => Err(tera::Error::msg(
                "tried to get site icon class from non-string",
            )),
        }
    }
}
