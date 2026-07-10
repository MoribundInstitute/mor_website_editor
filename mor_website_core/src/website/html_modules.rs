//! HTML template modules: reusable page building blocks for regular websites.
//!
//! The Blogger lineage compiled template variants into `<b:section>` XML;
//! here a module is a plain HTML partial + its structural CSS (+ optional
//! JS behavior). Everything is keyed to the theme engine's `.mor-*` hooks
//! and `--*` custom properties, so every preset restyles every module.
//!
//! Selection reuses the existing `TemplatePackConfig` slots
//! (`header_variant`, `left_sidebar_variant`, `right_sidebar_variant`,
//! `footer_variant`) so the Template Modules panel plumbing carries over.
//! Legacy Blogger ids stored in old configs resolve to the slot default;
//! `"none"` leaves a slot empty.

use crate::config::ThemeConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleSlot {
    Header,
    Sidebar,
    Footer,
}

pub struct HtmlModule {
    pub id: &'static str,
    pub name: &'static str,
    pub slot: ModuleSlot,
    pub description: &'static str,
    pub html: &'static str,
    /// Structural CSS folded into `mor-theme.css` when the module is selected.
    pub css: &'static str,
    /// Behavior folded into `mor-theme.js` when the module is selected.
    pub js: &'static str,
}

pub const NONE_ID: &str = "none";

// ────────────────────────────────────────────────────────────────────────────
// Headers
// ────────────────────────────────────────────────────────────────────────────

const HEADER_TOPBAR: HtmlModule = HtmlModule {
    id: "header_topbar",
    name: "Classic Topbar",
    slot: ModuleSlot::Header,
    description: "Sticky top bar: brand on the left, pill navigation on the right. Collapses to a stacked bar on small screens.",
    html: r#"<header class="main-header mor-topbar" data-edit-target="colors.bg_elevated">
  <a class="mor-brand" href="/">
    <span class="mor-brand-mark">◆</span>
    <span class="mor-brand-name" data-mor-edit="site.site_title" data-field-path="site.site_title">{{SITE_TITLE}}</span>
  </a>
  <nav class="mor-nav" aria-label="Primary">
    <a class="mor-pill" href="/">Home</a>
    <a class="mor-pill" href="/about.php">About</a>
    <a class="mor-pill" href="/projects.php">Projects</a>
    <a class="mor-pill" href="/contact.php">Contact</a>
  </nav>
</header>"#,
    css: r#"/* --- module: header_topbar --- */
.mor-topbar {
  position: sticky; top: 0; z-index: 100;
  display: flex; align-items: center; justify-content: space-between;
  gap: 1rem; flex-wrap: wrap;
  padding: 0.8rem 1.4rem;
  background: color-mix(in srgb, var(--bg-panel) 88%, transparent);
  backdrop-filter: blur(10px);
  border-bottom: var(--panel-border-width, 1px) solid var(--border-color);
}
.mor-brand {
  display: inline-flex; align-items: center; gap: 0.6rem;
  font-family: var(--font-heading); font-size: 1.15rem;
  color: var(--fg-base); text-decoration: none; letter-spacing: 0.04em;
}
.mor-brand-mark { color: var(--accent); }
.mor-nav { display: flex; flex-wrap: wrap; gap: 0.45rem; }
.mor-pill {
  padding: 0.42rem 0.9rem; border-radius: 999px;
  border: 1px solid var(--border-color);
  color: var(--fg-muted); text-decoration: none; font-size: 0.92rem;
  transition: color 160ms ease, border-color 160ms ease, background 160ms ease;
}
.mor-pill:hover, .mor-pill[aria-current="page"] {
  color: var(--accent);
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 10%, transparent);
}
"#,
    js: "",
};

const HEADER_MASTHEAD: HtmlModule = HtmlModule {
    id: "header_masthead",
    name: "Centered Masthead",
    slot: ModuleSlot::Header,
    description: "Editorial masthead: small-caps eyebrow, large display title, tagline, and a hairline rule — nav links beneath.",
    html: r#"<header class="main-header mor-masthead" data-edit-target="colors.bg_panel">
  <p class="mor-masthead-eyebrow">A small-caps eyebrow line</p>
  <h1 class="mor-masthead-title" data-mor-edit="site.site_title" data-field-path="site.site_title">{{SITE_TITLE}}</h1>
  <p class="mor-masthead-tagline" data-mor-edit="site.site_subtitle" data-field-path="site.site_subtitle">{{SITE_SUBTITLE}}</p>
  <nav class="mor-masthead-nav" aria-label="Primary">
    <a href="/">Home</a><span aria-hidden="true">·</span>
    <a href="/about.php">About</a><span aria-hidden="true">·</span>
    <a href="/projects.php">Projects</a>
  </nav>
</header>"#,
    css: r#"/* --- module: header_masthead --- */
.mor-masthead {
  text-align: center; padding: 3rem 1.5rem 1.6rem;
  border-bottom: var(--panel-border-width, 1px) solid var(--border-color);
  background: var(--bg-panel);
}
.mor-masthead-eyebrow {
  margin: 0 0 0.6rem; font-size: 0.72rem;
  letter-spacing: 0.16em; text-transform: uppercase; color: var(--fg-muted);
}
.mor-masthead-title {
  margin: 0; font-family: var(--font-heading);
  font-size: clamp(2.2rem, 6vw, 3.4rem); font-weight: var(--heading-weight, 600);
  color: var(--fg-base); letter-spacing: 0.03em;
}
.mor-masthead-tagline { margin: 0.6rem auto 0; max-width: 42ch; color: var(--fg-muted); }
.mor-masthead-nav { margin-top: 1.1rem; display: flex; justify-content: center; gap: 0.7rem; }
.mor-masthead-nav a { color: var(--fg-base); text-decoration-color: color-mix(in srgb, var(--accent) 45%, transparent); }
.mor-masthead-nav a:hover { color: var(--accent); }
.mor-masthead-nav span { color: var(--fg-muted); }
"#,
    js: "",
};

// ────────────────────────────────────────────────────────────────────────────
// Sidebars
// ────────────────────────────────────────────────────────────────────────────

const SIDEBAR_NAV: HtmlModule = HtmlModule {
    id: "sidebar_nav",
    name: "Collapsible Nav Sidebar",
    slot: ModuleSlot::Sidebar,
    description: "Sticky link rail with collapsible sections — native <details>, zero JavaScript. Sections remember nothing and need nothing.",
    html: r#"<aside class="mor-panel panel-left mor-sidebar-nav" aria-label="Site navigation">
  <a class="mor-sidebar-home" href="/">🏠 Home</a>
  <details open>
    <summary>Explore</summary>
    <nav>
      <a href="/projects.html">Projects</a>
      <a href="/writing.html">Writing</a>
      <a href="/archive.html">Archive</a>
    </nav>
  </details>
  <details>
    <summary>Elsewhere</summary>
    <nav>
      <a href="https://example.com" target="_blank" rel="noopener">Somewhere else</a>
    </nav>
  </details>
</aside>"#,
    css: r#"/* --- module: sidebar_nav --- */
.mor-sidebar-nav {
  position: sticky; top: 1rem; align-self: flex-start;
  width: 240px; flex-shrink: 0;
  max-height: calc(100vh - 2rem); overflow-y: auto;
  padding: 0.9rem;
  background: var(--bg-panel);
  border: var(--panel-border-width, 1px) solid var(--border-color);
  border-radius: 6px;
}
.mor-sidebar-home {
  display: block; padding: 0.45rem 0.6rem; margin-bottom: 0.4rem;
  color: var(--fg-base); text-decoration: none; border-radius: 4px;
}
.mor-sidebar-home:hover { background: color-mix(in srgb, var(--accent) 10%, transparent); }
.mor-sidebar-nav details { border-top: 1px solid var(--border-soft, var(--border-color)); padding: 0.35rem 0; }
.mor-sidebar-nav summary {
  cursor: pointer; list-style: none;
  padding: 0.4rem 0.6rem;
  font-size: 0.72rem; letter-spacing: 0.14em; text-transform: uppercase;
  color: var(--fg-muted); user-select: none;
}
.mor-sidebar-nav summary::after { content: "▾"; float: right; transition: transform 160ms ease; }
.mor-sidebar-nav details:not([open]) summary::after { transform: rotate(-90deg); }
.mor-sidebar-nav nav a {
  display: block; padding: 0.4rem 0.6rem 0.4rem 1rem;
  color: var(--fg-base); text-decoration: none; border-radius: 4px; font-size: 0.95rem;
}
.mor-sidebar-nav nav a:hover { color: var(--accent); background: color-mix(in srgb, var(--accent) 8%, transparent); }
@media (max-width: 760px) { .mor-sidebar-nav { position: static; width: auto; max-height: none; } }
"#,
    js: "",
};

const SIDEBAR_TOC: HtmlModule = HtmlModule {
    id: "sidebar_toc",
    name: "Table of Contents Rail",
    slot: ModuleSlot::Sidebar,
    description: "Sticky rail that builds itself from the page's h2/h3 headings and highlights the section you're reading (IntersectionObserver).",
    html: r#"<aside class="mor-panel panel-right mor-toc-rail" aria-label="Table of contents">
  <p class="mor-toc-title">On this page</p>
  <nav class="mor-toc-list"><!-- filled by mor-theme.js --></nav>
</aside>"#,
    css: r#"/* --- module: sidebar_toc --- */
.mor-toc-rail {
  position: sticky; top: 1rem; align-self: flex-start;
  width: 220px; flex-shrink: 0;
  max-height: calc(100vh - 2rem); overflow-y: auto;
  padding: 0.9rem 1rem;
  background: var(--bg-panel);
  border: var(--panel-border-width, 1px) solid var(--border-color);
  border-radius: 6px;
  font-size: 0.9rem;
}
.mor-toc-title {
  margin: 0 0 0.5rem; font-size: 0.72rem;
  letter-spacing: 0.14em; text-transform: uppercase; color: var(--fg-muted);
}
.mor-toc-list a {
  display: block; padding: 0.28rem 0.5rem;
  color: var(--fg-muted); text-decoration: none;
  border-left: 2px solid transparent;
}
.mor-toc-list a.mor-toc-h3 { padding-left: 1.2rem; font-size: 0.85rem; }
.mor-toc-list a:hover { color: var(--fg-base); }
.mor-toc-list a.active { color: var(--accent); border-left-color: var(--accent); }
@media (max-width: 900px) { .mor-toc-rail { display: none; } }
"#,
    js: r##"/* --- module: sidebar_toc --- */
(function () {
  var list = document.querySelector('.mor-toc-list');
  if (!list) return;
  var headings = Array.prototype.slice.call(document.querySelectorAll('main h2, main h3, .canvas-core h2, .canvas-core h3'));
  if (!headings.length) return;
  headings.forEach(function (h, i) {
    if (!h.id) h.id = 'mor-sec-' + i;
    var a = document.createElement('a');
    a.href = '#' + h.id;
    a.textContent = h.textContent;
    if (h.tagName === 'H3') a.className = 'mor-toc-h3';
    list.appendChild(a);
  });
  var links = list.querySelectorAll('a');
  var seen = new IntersectionObserver(function (entries) {
    entries.forEach(function (e) {
      if (!e.isIntersecting) return;
      links.forEach(function (l) { l.classList.remove('active'); });
      var hit = list.querySelector('a[href="#' + e.target.id + '"]');
      if (hit) hit.classList.add('active');
    });
  }, { rootMargin: '0px 0px -70% 0px' });
  headings.forEach(function (h) { seen.observe(h); });
})();
"##,
};

// ────────────────────────────────────────────────────────────────────────────
// Footers
// ────────────────────────────────────────────────────────────────────────────

const FOOTER_GRID: HtmlModule = HtmlModule {
    id: "footer_grid",
    name: "Mega Grid Footer",
    slot: ModuleSlot::Footer,
    description: "Multi-column link grid with a tagline column and a bottom strip: copyright on the left, smooth back-to-top on the right.",
    html: r#"<footer class="mor-footer mor-footer-grid">
  <div class="mor-footer-columns">
    <section>
      <h3>Explore</h3>
      <nav>
        <a href="/projects.html">Projects</a>
        <a href="/writing.html">Writing</a>
        <a href="/archive.html">Archive</a>
      </nav>
    </section>
    <section>
      <h3>Elsewhere</h3>
      <nav>
        <a href="https://example.com" target="_blank" rel="noopener">Somewhere else</a>
      </nav>
    </section>
    <section class="mor-footer-about">
      <h3>About</h3>
      <p>A line or two about the site — what it is, who keeps it, why it exists.</p>
    </section>
  </div>
  <hr class="mor-footer-rule" />
  <div class="mor-footer-strip">
    <p data-mor-edit="footer.footer_text" data-field-path="footer.footer_text">{{FOOTER_TEXT}}</p>
    <button type="button" onclick="window.scrollTo({top:0,behavior:'smooth'})" data-edit-target="buttons.radius">↑ Top</button>
  </div>
</footer>"#,
    css: r#"/* --- module: footer_grid --- */
.mor-footer-grid {
  margin-top: 3rem; padding: 2.2rem 1.5rem 1.2rem;
  background: var(--bg-panel);
  border-top: var(--panel-border-width, 1px) solid var(--border-color);
}
.mor-footer-columns {
  display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 1.8rem; max-width: 1100px; margin: 0 auto;
}
.mor-footer-grid h3 {
  margin: 0 0 0.7rem; font-family: var(--font-heading);
  font-size: 1rem; letter-spacing: 0.05em; color: var(--fg-base);
}
.mor-footer-grid nav a {
  display: block; padding: 0.3rem 0;
  color: var(--fg-muted); text-decoration: none;
}
.mor-footer-grid nav a:hover { color: var(--accent); }
.mor-footer-about p { margin: 0; color: var(--fg-muted); line-height: 1.6; }
.mor-footer-rule {
  margin: 1.6rem auto 0.9rem; max-width: 1100px; border: 0;
  border-top: 1px solid var(--border-soft, var(--border-color));
}
.mor-footer-strip {
  display: flex; justify-content: space-between; align-items: center;
  max-width: 1100px; margin: 0 auto; color: var(--fg-muted);
}
.mor-footer-strip p { margin: 0; }
.mor-footer-strip button {
  padding: 0.45rem 0.9rem; border-radius: var(--btn-radius, 6px);
  border: 1px solid var(--border-color); background: transparent;
  color: var(--fg-base); cursor: pointer; font-family: inherit;
}
.mor-footer-strip button:hover { border-color: var(--accent); color: var(--accent); }
"#,
    js: "",
};

const FOOTER_HAIRLINE: HtmlModule = HtmlModule {
    id: "footer_hairline",
    name: "Hairline Footer",
    slot: ModuleSlot::Footer,
    description: "One quiet centered line: copyright, a divider dot, and a back-to-top anchor. Nothing else.",
    html: r##"<footer class="mor-footer mor-footer-hairline">
  <p><span data-mor-edit="footer.footer_text" data-field-path="footer.footer_text">{{FOOTER_TEXT}}</span> <span aria-hidden="true">·</span> <a href="#top">back to top ↑</a></p>
</footer>"##,
    css: r#"/* --- module: footer_hairline --- */
.mor-footer-hairline {
  margin-top: 3rem; padding: 1.4rem 1rem; text-align: center;
  border-top: 1px solid var(--border-soft, var(--border-color));
  color: var(--fg-muted); font-size: 0.9rem;
}
.mor-footer-hairline p { margin: 0; }
.mor-footer-hairline a { color: var(--fg-muted); }
.mor-footer-hairline a:hover { color: var(--accent); }
"#,
    js: "",
};

// ────────────────────────────────────────────────────────────────────────────
// Registry + selection
// ────────────────────────────────────────────────────────────────────────────

pub const ALL_MODULES: &[&HtmlModule] = &[
    &HEADER_TOPBAR,
    &HEADER_MASTHEAD,
    &SIDEBAR_NAV,
    &SIDEBAR_TOC,
    &FOOTER_GRID,
    &FOOTER_HAIRLINE,
];

pub fn modules_for_slot(slot: ModuleSlot) -> Vec<&'static HtmlModule> {
    ALL_MODULES.iter().copied().filter(|m| m.slot == slot).collect()
}

pub fn module_by_id(id: &str) -> Option<&'static HtmlModule> {
    ALL_MODULES.iter().copied().find(|m| m.id == id)
}

/// Resolve a stored variant id to a module. `"none"` → None; a legacy
/// Blogger id (or anything unknown) falls back to the given default so old
/// configs keep working without migration.
fn resolve(id: &str, default: &'static HtmlModule) -> Option<&'static HtmlModule> {
    if id == NONE_ID {
        return None;
    }
    module_by_id(id).or(Some(default))
}

/// The modules a config selects, in page order:
/// header, left sidebar, right sidebar, footer.
pub fn selected_modules(config: &ThemeConfig) -> Vec<&'static HtmlModule> {
    let pack = &config.template_pack;
    [
        resolve(&pack.header_variant, &HEADER_TOPBAR),
        resolve(&pack.left_sidebar_variant, &SIDEBAR_NAV),
        resolve(&pack.right_sidebar_variant, &SIDEBAR_TOC),
        resolve(&pack.footer_variant, &FOOTER_GRID),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Structural CSS of the selected modules (deduplicated — the same module in
/// two sidebar slots contributes once).
pub fn selected_module_css(config: &ThemeConfig) -> Vec<&'static str> {
    let mut seen = Vec::new();
    for m in selected_modules(config) {
        if !m.css.is_empty() && !seen.contains(&m.css) {
            seen.push(m.css);
        }
    }
    seen
}

/// Combined behavior JS for the selected modules; empty string when no
/// selected module ships JS (callers then skip writing mor-theme.js).
pub fn generate_theme_js(config: &ThemeConfig) -> String {
    let mut seen: Vec<&str> = Vec::new();
    for m in selected_modules(config) {
        if !m.js.is_empty() && !seen.contains(&m.js) {
            seen.push(m.js);
        }
    }
    seen.join("\n")
}

/// Stamp Site Contract placeholders (`{{SITE_TITLE}}` etc.) and ensure edit
/// markers carry the live config values for Editor Canvas.
pub fn stamp_site_placeholders(html: &str, config: &ThemeConfig) -> String {
    html.replace("{{SITE_TITLE}}", &escape_html_min(&config.site.site_title))
        .replace(
            "{{SITE_SUBTITLE}}",
            &escape_html_min(&config.site.site_subtitle),
        )
        .replace(
            "{{FOOTER_TEXT}}",
            &escape_html_min(&config.footer.footer_text),
        )
}

fn escape_html_min(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A complete starter page composed from the selected modules: header, then
/// a two/three-column `.page-body` row (sidebars + main content), then the
/// footer — already linked to /mor-theme.css and /mor-theme.js.
/// Site fields are stamped with `data-mor-edit` markers for Editor Canvas.
pub fn generate_starter_page(config: &ThemeConfig, title: &str) -> String {
    let pack = &config.template_pack;
    let header = resolve(&pack.header_variant, &HEADER_TOPBAR);
    let left = resolve(&pack.left_sidebar_variant, &SIDEBAR_NAV);
    let right = resolve(&pack.right_sidebar_variant, &SIDEBAR_TOC);
    let footer = resolve(&pack.footer_variant, &FOOTER_GRID);

    let js_tag = if generate_theme_js(config).is_empty() {
        ""
    } else {
        "\n  <script src=\"/mor-theme.js\" defer></script>"
    };

    let page_title = if title.is_empty() {
        config.site.site_title.as_str()
    } else {
        title
    };

    let raw = format!(
        r#"<!doctype html>
<html lang="en" data-theme="dark" id="top">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title}</title>
  <link rel="stylesheet" href="/mor-theme.css" />{js_tag}
  <style>
    body {{ margin: 0; background: var(--bg-base); color: var(--fg-base); font-family: var(--font-body); }}
    .page-body {{ display: flex; gap: 1.2rem; align-items: flex-start; max-width: 1200px; margin: 1.2rem auto; padding: 0 1rem; }}
    .canvas-core {{ flex: 1; min-width: 0; }}
  </style>
</head>
<body>
{header}
<div class="page-body">
{left}
<main class="canvas-core">
  <article class="mor-post" data-edit-target="colors.bg_panel">
    <h1 data-mor-edit="site.site_title" data-field-path="site.site_title" data-edit-target="typography.heading_font_stack">{{SITE_TITLE}}</h1>
    <p data-mor-edit="site.site_subtitle" data-field-path="site.site_subtitle" data-edit-target="typography.body_font_stack">{{SITE_SUBTITLE}}</p>
    <p>This page was generated from your selected template modules. Replace this content and duplicate the file for new pages.</p>
    <h2>A first section</h2>
    <p>Headings here feed the Table of Contents rail automatically (if selected).</p>
    <h2>A second section</h2>
    <p>Everything on this page is styled by <code data-edit-target="typography.mono_font_stack">mor-theme.css</code> — switch presets in the editor and re-export to restyle it all.</p>
  </article>
</main>
{right}
</div>
{footer}
</body>
</html>"#,
        title = escape_html_min(page_title),
        js_tag = js_tag,
        header = header.map(|m| m.html).unwrap_or(""),
        left = left.map(|m| m.html).unwrap_or(""),
        right = right.map(|m| m.html).unwrap_or(""),
        footer = footer.map(|m| m.html).unwrap_or(""),
    );
    stamp_site_placeholders(&raw, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::defaults::default_theme_config;

    #[test]
    fn legacy_blogger_ids_resolve_to_defaults() {
        let config = default_theme_config(); // stores mor_header_baseline etc.
        let selected = selected_modules(&config);
        assert_eq!(selected.len(), 4);
        assert_eq!(selected[0].id, "header_topbar");
        assert_eq!(selected[3].id, "footer_grid");
    }

    #[test]
    fn none_empties_a_slot_and_dedup_holds() {
        let mut config = default_theme_config();
        config.template_pack.header_variant = NONE_ID.into();
        config.template_pack.left_sidebar_variant = "sidebar_toc".into();
        config.template_pack.right_sidebar_variant = "sidebar_toc".into();
        let selected = selected_modules(&config);
        assert_eq!(selected.len(), 3); // no header; toc twice + footer
        assert_eq!(selected_module_css(&config).len(), 2); // toc css once + footer css
        assert!(generate_theme_js(&config).contains("mor-toc-list"));
    }

    #[test]
    fn starter_page_composes_selected_modules() {
        let config = default_theme_config();
        let page = generate_starter_page(&config, "My Site");
        assert!(page.contains("mor-topbar"));
        assert!(page.contains("mor-sidebar-nav"));
        assert!(page.contains("mor-toc-rail"));
        assert!(page.contains("mor-footer-grid"));
        assert!(page.contains("/mor-theme.css"));
        assert!(page.contains("/mor-theme.js")); // toc module ships js
        // Editor Canvas markers + stamped site title (no leftover placeholders)
        assert!(page.contains("data-mor-edit=\"site.site_title\""));
        assert!(page.contains(&config.site.site_title));
        assert!(!page.contains("{{SITE_TITLE}}"));
        let mut no_js = default_theme_config();
        no_js.template_pack.right_sidebar_variant = NONE_ID.into();
        assert!(!generate_starter_page(&no_js, "x").contains("mor-theme.js"));
    }
}
