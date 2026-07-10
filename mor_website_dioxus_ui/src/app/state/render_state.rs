use dioxus::prelude::*;

use crate::app::state::theme_state::ThemeState;
use crate::app::state::website_state::WebsiteState;
use mor_website_core::config::ThemeConfig;
use mor_website_core::diagnostics::DiagnosticResult;
use mor_website_core::render::template_resolver::{
    ComponentManifest, CONTENT_REGISTRY, FOOTER_REGISTRY, HEADER_REGISTRY, LAYOUT_REGISTRY,
    SIDEBAR_LEFT_REGISTRY, SIDEBAR_RIGHT_REGISTRY,
};
use mor_website_core::website::{check_website, generate_theme_css, prepare_preview_html};

#[derive(Clone, Copy)]
pub struct RenderState {
    pub current_config: Memo<ThemeConfig>,
    /// The finished `mor-theme.css` for the live config (was: generated_xml).
    pub generated_css: Memo<String>,
    pub preview_html: Memo<String>,
    pub diag: Signal<DiagnosticResult>,
}

/// Styled placeholder shown before a website folder is opened, or while the
/// selected page has no fetched content yet. Carries the generated theme CSS
/// under the mor-true-css id so the PreviewCanvas morpher (and live token
/// edits) behave identically to a real page.
fn welcome_html(theme_css: &str, is_dark: bool, body: &str) -> String {
    let mode = if is_dark { "dark" } else { "light" };
    format!(
        r#"<html data-theme="{mode}"><head><style id="mor-true-css">{theme_css}</style><style>
html, body {{ height: 100%; margin: 0; }}
body {{ display: flex; align-items: center; justify-content: center; font-family: var(--body-font-stack, sans-serif); background: var(--bg-base, #10161f); color: var(--fg-base, #ddd); }}
.mor-welcome {{ text-align: left; max-width: 36rem; padding: 2rem 2.2rem; border: 1px solid var(--border, #333); border-radius: 12px; background: var(--bg-panel, #151d29); box-shadow: 0 12px 40px rgba(0,0,0,.25); }}
.mor-welcome h1 {{ margin: 0 0 0.35rem 0; font-size: 1.45rem; color: var(--accent, #7aa2f7); text-align: center; }}
.mor-welcome .tagline {{ text-align: center; margin: 0 0 1.25rem; opacity: 0.8; font-size: 0.95rem; }}
.mor-welcome ol {{ margin: 0 0 1rem; padding-left: 1.25rem; line-height: 1.7; }}
.mor-welcome li {{ margin: 0.35rem 0; }}
.mor-welcome p {{ margin: 0.35rem 0; opacity: 0.88; line-height: 1.55; }}
.mor-welcome code {{ background: var(--bg-elevated, #1c2635); padding: 1px 6px; border-radius: 4px; font-size: 0.9em; }}
.mor-welcome .step-num {{ display: inline-block; width: 1.4em; height: 1.4em; line-height: 1.4em; text-align: center; border-radius: 50%; background: color-mix(in srgb, var(--accent, #7aa2f7) 22%, transparent); color: var(--accent, #7aa2f7); font-size: 0.8rem; font-weight: 700; margin-right: 0.35rem; }}
.mor-welcome .hint {{ margin-top: 1rem; padding-top: 0.9rem; border-top: 1px solid var(--border, #333); font-size: 0.88rem; opacity: 0.75; }}
</style></head><body><div class="mor-welcome">
{body}
</div></body></html>"#
    )
}

const WELCOME_BODY: &str = r#"<h1>MorWebsite Editor</h1>
<p class="tagline">Open a hand-rolled site. Change how it looks. Export one CSS file.</p>
<ol>
<li><span class="step-num">1</span> <strong>Open a folder</strong> — <code>File → Open Website Folder…</code><br/>Try the repo’s <code>examples/mor_starter</code>, or run <code>mwt init --template starter</code>.</li>
<li><span class="step-num">2</span> <strong>Pick a preset</strong> — Theme Palette / Presets dock. Colors and type update live in the preview.</li>
<li><span class="step-num">3</span> <strong>Export</strong> — <code>File → Export mor-theme.css</code>. Link it from your pages.</li>
</ol>
<p><strong>Edit mode:</strong> on the preview ribbon choose <em>Edit</em>, then double-click text marked with <code>data-mor-edit</code> (site title, footer…).</p>
<p class="hint">Designer mode keeps Theme, Pages, and Presets on the bar. Use <code>View → Advanced Mode</code> for CSS/JS docks. Contract: <code>docs/SITE_CONTRACT.md</code>.</p>"#;

fn page_unavailable_body(page: &str) -> String {
    format!(
        "<p>Couldn't load <code>{page}</code> from the preview server.</p>\
<p>The page may be empty or unreadable — pick another page in <code>Site Pages</code> or hit ↻.</p>"
    )
}

impl RenderState {
    pub fn new(theme: ThemeState, website: WebsiteState) -> Self {
        let signals = theme.signals;
        let active_preset = theme.active_preset;
        let active_variant = theme.active_variant;

        let current_config = use_memo(move || {
            let mut config = signals.to_config();
            config.active_preset_id = active_preset().map(|s| s.to_string());
            config.active_variant_id = active_variant().map(|s| s.to_string());
            config
        });

        let current_config_for_css = current_config;
        let generated_css = use_memo(move || generate_theme_css(&current_config_for_css()));

        let is_dark_mode = theme.signals.is_dark_mode;
        let raw_page_html = website.raw_page_html;
        let server = website.server;
        let project_for_preview = website.project;

        let current_page_for_preview = website.current_page;
        let preview_html = use_memo(move || {
            let raw = raw_page_html();
            let theme_css = generated_css();
            let is_dark = is_dark_mode();
            match server() {
                Some(info) if project_for_preview.read().is_open() && !raw.is_empty() => {
                    prepare_preview_html(
                        &raw,
                        &format!("http://127.0.0.1:{}/", info.port),
                        &theme_css,
                        is_dark,
                    )
                }
                // Project open but nothing fetched: say so instead of
                // pretending no folder is open.
                Some(_) if project_for_preview.read().is_open() => {
                    let page = current_page_for_preview
                        .read()
                        .clone()
                        .unwrap_or_else(|| "(no page selected)".into());
                    welcome_html(&theme_css, is_dark, &page_unavailable_body(&page))
                }
                _ => welcome_html(&theme_css, is_dark, WELCOME_BODY),
            }
        });

        let current_config_for_diag_init = current_config;
        let current_config_for_diag_effect = current_config;
        let project_for_diag_init = website.project;
        let project_for_diag_effect = website.project;
        let nonce = website.preview_nonce;

        let mut diag = use_signal(move || {
            check_website(&project_for_diag_init.read(), &current_config_for_diag_init())
        });

        use_effect(move || {
            let _ = nonce(); // page saves bump this — recheck against fresh files
            diag.set(check_website(
                &project_for_diag_effect.read(),
                &current_config_for_diag_effect(),
            ));
        });

        Self {
            current_config,
            generated_css,
            preview_html,
            diag,
        }
    }

    pub fn get_manifest(&self, registry_type: &str, id: &str) -> Option<ComponentManifest> {
        let registry = match registry_type {
            "header" => HEADER_REGISTRY,
            "layout" => LAYOUT_REGISTRY,
            "content" => CONTENT_REGISTRY,
            "sidebar_left" => SIDEBAR_LEFT_REGISTRY,
            "sidebar_right" => SIDEBAR_RIGHT_REGISTRY,
            "footer" => FOOTER_REGISTRY,
            _ => return None,
        };
        registry.iter().find(|c| c.id == id).cloned()
    }
}
