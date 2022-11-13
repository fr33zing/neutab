//! Library for newtabgen-cli.
//! Create static new tab pages from a config file.

#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

mod tera_filters;
mod tera_functions;

pub mod builder;
pub mod config;
pub mod resources;
pub mod util;
