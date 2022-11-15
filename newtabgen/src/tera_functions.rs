// SPDX-License-Identifier: GPL-3.0-or-later

//! Helpful functions for use with tera.

mod len;
pub use len::Len;

mod count_links_in_page;
pub use count_links_in_page::CountLinksInPage;

mod svg_icon_href;
pub use svg_icon_href::SvgIconHref;
