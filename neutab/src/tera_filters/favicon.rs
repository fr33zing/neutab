use std::collections::HashMap;

use tera::{to_value, Filter};

use crate::util;

pub struct Favicon;

impl Filter for Favicon {
    fn filter(
        &self,
        value: &tera::Value,
        _args: &HashMap<String, tera::Value>,
    ) -> tera::Result<tera::Value> {
        match value.as_str() {
            Some(url) => {
                let output = util::favicon_class(url).map_err(|_| {
                    tera::Error::msg(format!("failed to get favicon class for url: '{url}'"))
                })?;
                to_value(output).map_err(|_| {
                    tera::Error::msg("formatting favicon class produced invalid value: '{output}'")
                })
            }
            None => Err(tera::Error::msg(
                "tried to get favicon class from non-string",
            )),
        }
    }
}
