//! Provides the `len` Tera function.

use std::collections::HashMap;

use tera::{to_value, Error, Result, Value};

/// Length filter for use in Tera templates. Returns the length of the provided array.
///
/// # Example
///
/// ```html
/// <span>
///     Config has {{ len(arr = config.pages) }} pages.
/// </span>
/// ```
pub struct Len;

impl tera::Function for Len {
    fn call(&self, args: &HashMap<String, Value>) -> Result<Value> {
        let arr = args
            .get("arr")
            .ok_or_else(|| Error::msg("len requires argument `arr`"))?;

        match arr.as_array() {
            Some(arr) => to_value(arr.len()).map_err(|_| Error::msg("len produced invalid value")),
            None => Err(Error::msg("`arr` must be an array")),
        }
    }
}
