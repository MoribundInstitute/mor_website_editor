//! Website-project context: which folder is open, which page is previewed,
//! and the local preview server serving it. The website-plug counterpart of
//! `SiteData` from the Blogger lineage.

use dioxus::prelude::*;

use crate::app::services::site_server::SiteServerInfo;
use mor_website_core::website::WebsiteProject;

#[derive(Clone, Copy)]
pub struct WebsiteState {
    /// The opened website folder inventory (empty root = nothing open).
    pub project: Signal<WebsiteProject>,
    /// Relative path of the page shown in the preview.
    pub current_page: Signal<Option<String>>,
    /// The running preview server, if any.
    pub server: Signal<Option<SiteServerInfo>>,
    /// Raw HTML of `current_page` as fetched from the server.
    pub raw_page_html: Signal<String>,
    /// Bump to force a refetch of the current page.
    pub preview_nonce: Signal<u64>,
}

impl WebsiteState {
    pub fn new() -> Self {
        WebsiteState {
            project: use_signal(WebsiteProject::default),
            current_page: use_signal(|| None),
            server: use_signal(|| None),
            raw_page_html: use_signal(String::new),
            preview_nonce: use_signal(|| 0u64),
        }
    }

    /// Ask the preview to refetch the current page.
    pub fn bump_preview(&self) {
        let mut nonce = self.preview_nonce;
        let next = nonce.peek().wrapping_add(1);
        nonce.set(next);
    }
}
