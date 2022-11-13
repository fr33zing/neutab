//! Provides the `count_links_in_page` Tera function.

use std::collections::HashMap;

use tera::{to_value, Error, Result, Value};

use crate::config::Config;

/// Link counting function for use in Tera templates. Returns the number of links in all sections of
/// the provided page.
///
/// # Example
///
/// ```html
/// <span>
///     {% set n = count_links_in_page(page_name = page.name) %}
///     This page has {{ n }} links.
/// <span>
/// ```
pub struct CountLinksInPage(pub Config);

impl tera::Function for CountLinksInPage {
    fn call(&self, args: &HashMap<String, Value>) -> Result<Value> {
        let page_name = args
            .get("page_name")
            .ok_or_else(|| Error::msg("count_links requires argument `page_name`"))?;

        match page_name.as_str() {
            Some(page_name) => {
                let n = self
                    .0
                    .pages
                    .iter()
                    .find(|p| p.name == page_name)
                    .ok_or_else(|| Error::msg("page not found"))?
                    .sections
                    .iter()
                    .fold(0, |sum, val| sum + val.links.len());
                to_value(n).map_err(|_| Error::msg("count_links produced invalid value"))
            }
            None => Err(Error::msg("`page_name` must be a str")),
        }
    }
}
