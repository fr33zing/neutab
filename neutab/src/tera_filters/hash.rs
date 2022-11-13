use std::collections::HashMap;

use tera::{to_value, Filter};

use crate::util;

pub struct Hash;

impl Filter for Hash {
    fn filter(
        &self,
        value: &tera::Value,
        _args: &HashMap<String, tera::Value>,
    ) -> tera::Result<tera::Value> {
        match value.as_str() {
            Some(v) => to_value(util::sha1_base32(v.as_bytes())).map_err(|_| {
                tera::Error::msg("base32 encoding produced invalid value: '{output}'")
            }),
            None => Err(tera::Error::msg("tried to hash non-string")),
        }
    }
}
