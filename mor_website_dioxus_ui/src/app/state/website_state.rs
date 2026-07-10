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

    /// Hard refresh: refetch from disk, drop the iframe morph cache, full
    /// document rewrite, and cache-bust local CSS/JS so stylesheet edits land.
    /// Bound to Ctrl+Shift+R (and the View menu / toolbar ↻ button).
    pub fn hard_refresh_preview(&self) {
        self.bump_preview();
        // Fire-and-forget: clear the canvas morph cache immediately so the
        // upcoming HTML write is a full reload even if the body is unchanged,
        // and rewrite the current document so JS state resets right away.
        let _ = dioxus::document::eval(
            r#"
            (function () {
              var src = document.getElementById('mor-preview-html-source');
              if (src) src._last = null;
              if (typeof window.__morHardRefreshPreview === 'function') {
                window.__morHardRefreshPreview();
              }
            })();
            "#,
        );
    }
}
