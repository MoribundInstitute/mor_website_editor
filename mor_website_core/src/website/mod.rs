//! Website-project engine: the MorWebsiteEditor "plug" core.
//!
//! Where the Blogger lineage assembled `<b:skin>` XML, this module targets a
//! plain website folder (HTML/PHP/CSS/JS). Three jobs:
//!   1. Scan a project folder into a `WebsiteProject` inventory.
//!   2. Generate a standalone `mor-theme.css` from `ThemeConfig` (token layer:
//!      `:root` vars, typography, buttons, cursor/scrollbar, effects, preset CSS).
//!   3. Prepare preview HTML: inject a `<base>` and the generated CSS under
//!      `<style id="mor-true-css">` so the PreviewCanvas DOM morpher patches
//!      token edits live without iframe reloads.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use crate::config::ThemeConfig;
use crate::render::css_builder::build_master_css;
use crate::render::xml_parts::css_generator::render_css_sockets;

pub mod html_modules;
pub mod page_assets;
pub mod page_edit;
pub mod publish_protect;

pub const THEME_CSS_FILENAME: &str = "mor-theme.css";
pub const THEME_JS_FILENAME: &str = "mor-theme.js";
pub const STARTER_PAGE_FILENAME: &str = "mor-starter.html";

/// Directories never worth scanning in a website project.
const SKIP_DIRS: &[&str] = &[
    ".git", "node_modules", "target", "dist", "vendor", ".svn", ".hg",
    "graphify-out", "__pycache__", ".next", ".cache",
];

/// Extensions that count as previewable pages, in the order pages are ranked.
const PAGE_EXTS: &[&str] = &["html", "htm", "php"];

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WebsiteProject {
    /// Absolute root of the opened website folder.
    pub root: PathBuf,
    /// Relative paths (with `/` separators) of previewable pages.
    pub pages: Vec<String>,
    /// Relative paths of stylesheets.
    pub css_files: Vec<String>,
    /// Relative paths of scripts.
    pub js_files: Vec<String>,
}

impl WebsiteProject {
    pub fn is_open(&self) -> bool {
        !self.root.as_os_str().is_empty()
    }

    /// Best default page: index.* first, then shortest path.
    pub fn default_page(&self) -> Option<&str> {
        self.pages
            .iter()
            .find(|p| {
                let stem = p.rsplit('/').next().unwrap_or(p);
                stem.starts_with("index.")
            })
            .or_else(|| self.pages.first())
            .map(|s| s.as_str())
    }
}

/// Marker files that identify a subdirectory as a separate application
/// install (its own site/subdomain, e.g. a MediaWiki or WordPress living
/// under public_html) rather than pages of the website being themed.
const APP_INSTALL_MARKERS: &[&str] = &["LocalSettings.php", "wp-config.php"];

/// Read `.morignore` in the project root: one relative path prefix per line
/// (`wiki`, `blog/archive`), `#` comments and blank lines ignored.
fn load_ignore_rules(root: &Path) -> Vec<String> {
    std::fs::read_to_string(root.join(".morignore"))
        .map(|s| {
            s.lines()
                .map(|l| l.trim().trim_end_matches('/').to_string())
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .collect()
        })
        .unwrap_or_default()
}

fn is_ignored(rel: &str, rules: &[String]) -> bool {
    rules
        .iter()
        .any(|r| rel == r || rel.starts_with(&format!("{r}/")))
}

/// A directory that is its own application install (wiki, blog engine…)
/// is out of scope for theming — its pages would drown the real site's.
fn is_app_install(dir: &Path) -> bool {
    APP_INSTALL_MARKERS.iter().any(|m| dir.join(m).exists())
}

/// Walk `root` and inventory pages / stylesheets / scripts.
/// Depth-first, sorted, skipping [`SKIP_DIRS`], hidden directories,
/// `.morignore` entries, and nested app installs (MediaWiki, WordPress).
pub fn scan_project(root: &Path) -> io::Result<WebsiteProject> {
    let root = root.canonicalize()?;
    let mut project = WebsiteProject {
        root: root.clone(),
        ..Default::default()
    };
    let ignore = load_ignore_rules(&root);
    walk(&root, &root, &ignore, &mut project)?;
    // index pages first, then alphabetical — the page list doubles as the
    // preview page picker so ordering is UX, not cosmetics.
    project.pages.sort_by_key(|p| {
        let is_index = Path::new(p)
            .file_stem()
            .map(|s| s == "index")
            .unwrap_or(false);
        (!is_index, p.matches('/').count(), p.clone())
    });
    project.css_files.sort();
    project.js_files.sort();
    Ok(project)
}

fn walk(root: &Path, dir: &Path, ignore: &[String], out: &mut WebsiteProject) -> io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if path.is_dir() {
            if name.starts_with('.')
                || SKIP_DIRS.contains(&name.as_str())
                || is_ignored(&rel, ignore)
                || is_app_install(&path)
            {
                continue;
            }
            walk(root, &path, ignore, out)?;
            continue;
        }
        if is_ignored(&rel, ignore) {
            continue;
        }
        match path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()) {
            Some(ext) if PAGE_EXTS.contains(&ext.as_str()) => out.pages.push(rel),
            Some(ext) if ext == "css" => out.css_files.push(rel),
            Some(ext) if ext == "js" || ext == "mjs" => out.js_files.push(rel),
            _ => {}
        }
    }
    Ok(())
}

/// Load every stylesheet and script of the project into a VFS map keyed by
/// relative path. This is what the CSS/JS editor docks operate on; saving
/// writes back to disk via [`save_vfs_file`].
pub fn load_project_vfs(project: &WebsiteProject) -> HashMap<String, String> {
    let mut vfs = HashMap::new();
    for rel in project.css_files.iter().chain(project.js_files.iter()) {
        if let Ok(content) = std::fs::read_to_string(project.root.join(rel)) {
            vfs.insert(rel.clone(), content);
        }
    }
    vfs
}

/// Persist one VFS entry back into the project folder.
pub fn save_vfs_file(project: &WebsiteProject, rel: &str, content: &str) -> io::Result<PathBuf> {
    let dest = project.root.join(rel);
    // Refuse traversal outside the project (a VFS key is user-influenced data).
    if rel.contains("..") {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "path escapes project"));
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&dest, content)?;
    Ok(dest)
}

/// Generate the finished, standalone theme stylesheet from the config:
/// `:root` custom properties, the selected template modules' structural CSS,
/// per-element typography, buttons, cursor, scrollbar, glow/effects, image
/// frames, and the active preset's CSS — with every `{{TOKEN}}` socket
/// resolved for light+dark palettes. Module CSS rides as base chunks so
/// preset CSS still cascades over it.
pub fn generate_theme_css(config: &ThemeConfig) -> String {
    use crate::config::fonts::{
        build_webfont_css_import, typography_font_stacks, FontProvider,
    };

    let module_css = html_modules::selected_module_css(config);
    let css = build_master_css(&module_css, config);
    let body = render_css_sockets(css, config);

    // Webfonts must lead the stylesheet (@import rules first). Provider can
    // be google / bunny / none; custom_font_css holds self-hosted @font-face.
    let stacks = typography_font_stacks(
        &config.typography.body_font_stack,
        &config.typography.heading_font_stack,
        &config.typography.mono_font_stack,
    );
    let stack_refs: Vec<&str> = stacks.iter().map(|s| s.as_str()).collect();
    let provider = FontProvider::from_config(&config.typography.font_provider);
    let mut out = build_webfont_css_import(&stack_refs, provider);
    let custom = config.typography.custom_font_css.trim();
    if !custom.is_empty() {
        out.push_str(custom);
        if !custom.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
    }
    out.push_str(&body);
    out
}

/// Write the generated stylesheet as `mor-theme.css` in the project root.
/// Returns the path written.
pub fn export_theme_css(project: &WebsiteProject, config: &ThemeConfig) -> io::Result<PathBuf> {
    let dest = project.root.join(THEME_CSS_FILENAME);
    std::fs::write(&dest, generate_theme_css(config))?;
    Ok(dest)
}

/// Lightweight token bridge for hand-rolled website previews.
///
/// Full [`generate_theme_css`] includes module chrome + `preset_css`, which
/// fights a site that already ships its own CSS (looks.css, etc.) and makes
/// the preview look like two themes layered. This emits **only** CSS variables
/// so the ribbon palette can still retint tokens without re-skinning layout.
pub fn generate_preview_bridge_css(config: &ThemeConfig) -> String {
    use crate::presets::resolve_palette_pair;

    let (light, dark) = resolve_palette_pair(
        config.active_preset_id.as_deref(),
        config.active_variant_id.as_deref(),
        config,
    );

    let panel = |c: &crate::config::ColorConfig| c.bg_panel.to_css();
    let elev = |c: &crate::config::ColorConfig| c.bg_elevated.to_css();

    format!(
        r#"/* mor-theme bridge — tokens only (site CSS owns chrome) */
:root {{
  --bg-base: {dbg};
  --bg-panel: {dpanel};
  --bg-elevated: {delev};
  --bg-workspace: {dbg};
  --fg-base: {dfg};
  --fg-muted: {dmuted};
  --fg-dim: {dfg};
  --accent: {dacc};
  --border-color: {dborder};
  --theme-border-color: {dborder};
  --link-color: {dfg};
  --link-visited: {dmuted};
  --font-body: {body};
  --font-heading: {heading};
  --font-display: {heading};
  --font-mono: {mono};
  --font-size-base: {size};
  --line-height-body: {lh};
  color-scheme: dark;
}}
html[data-theme="light"] {{
  --bg-base: {lbg};
  --bg-panel: {lpanel};
  --bg-elevated: {lelev};
  --bg-workspace: {lbg};
  --fg-base: {lfg};
  --fg-muted: {lmuted};
  --fg-dim: {lfg};
  --accent: {lacc};
  --border-color: {lborder};
  --theme-border-color: {lborder};
  --link-color: {lacc};
  --link-visited: #0b0080;
  --font-body: sans-serif;
  --font-heading: sans-serif;
  --font-display: sans-serif;
  color-scheme: light;
}}
"#,
        dbg = dark.colors.bg_base,
        dpanel = panel(&dark.colors),
        delev = elev(&dark.colors),
        dfg = dark.colors.fg_base,
        dmuted = dark.colors.fg_muted,
        dacc = dark.colors.accent,
        dborder = dark.colors.border,
        body = config.typography.body_font_stack,
        heading = if config.typography.heading_font_stack.is_empty() {
            &config.typography.body_font_stack
        } else {
            &config.typography.heading_font_stack
        },
        mono = config.typography.mono_font_stack,
        size = config.typography.base_size,
        lh = config.typography.line_height,
        lbg = light.colors.bg_base,
        lpanel = panel(&light.colors),
        lelev = elev(&light.colors),
        lfg = light.colors.fg_base,
        lmuted = light.colors.fg_muted,
        lacc = light.colors.accent,
        lborder = light.colors.border,
    )
}

/// The `<link>` snippet a site adds to consume the exported stylesheet.
pub fn theme_link_snippet() -> String {
    format!("<link rel=\"stylesheet\" href=\"/{THEME_CSS_FILENAME}\" />")
}

/// The `<script>` snippet for module behaviors (TOC scrollspy etc.).
pub fn theme_js_snippet() -> String {
    format!("<script src=\"/{THEME_JS_FILENAME}\" defer></script>")
}

/// Everything "write the theme into the project" means, in one call:
/// `mor-theme.css` always; `mor-theme.js` only when the selected modules
/// ship behavior (a stale `mor-theme.js` is removed when they stop).
/// Returns the relative filenames written.
pub fn export_theme_bundle(project: &WebsiteProject, config: &ThemeConfig) -> io::Result<Vec<String>> {
    let mut written = vec![THEME_CSS_FILENAME.to_string()];
    std::fs::write(project.root.join(THEME_CSS_FILENAME), generate_theme_css(config))?;
    let js = html_modules::generate_theme_js(config);
    if js.is_empty() {
        let _ = std::fs::remove_file(project.root.join(THEME_JS_FILENAME));
    } else {
        std::fs::write(project.root.join(THEME_JS_FILENAME), js)?;
        written.push(THEME_JS_FILENAME.to_string());
    }
    Ok(written)
}

/// Write the starter page composed from the selected modules. Always
/// overwrites — it is a generated artifact, not user content.
pub fn export_starter_page(project: &WebsiteProject, config: &ThemeConfig, title: &str) -> io::Result<PathBuf> {
    let dest = project.root.join(STARTER_PAGE_FILENAME);
    std::fs::write(&dest, html_modules::generate_starter_page(config, title))?;
    Ok(dest)
}

/// Page stack for **File → New Website…** / `mwt init`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NewSitePages {
    /// PHP pages with shared `includes/header.php` + `footer.php` (recommended).
    #[default]
    PhpModular,
    /// Standalone `.html` pages (no PHP required).
    StaticHtml,
}

/// Options for scaffolding a fresh website folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSiteOptions {
    pub pages: NewSitePages,
    /// Write `css/site.css` and link it from every page.
    pub include_site_css: bool,
    /// Write `components/mor-card.js` (+ optional `js/site.js`) and link them.
    pub include_js: bool,
    /// Also write an About page.
    pub about_page: bool,
}

impl Default for NewSiteOptions {
    fn default() -> Self {
        Self {
            pages: NewSitePages::PhpModular,
            include_site_css: true,
            include_js: true,
            about_page: true,
        }
    }
}

/// Full starter (PHP + CSS + JS + About). Used by `mwt init --template starter`.
pub fn scaffold_starter_site(root: &Path, config: &ThemeConfig) -> io::Result<Vec<String>> {
    scaffold_new_site(root, config, &NewSiteOptions::default())
}

/// Scaffold a website folder from dialog / CLI options.
/// Always writes `workspace.toml` + generated `mor-theme.css`.
pub fn scaffold_new_site(
    root: &Path,
    config: &ThemeConfig,
    opts: &NewSiteOptions,
) -> io::Result<Vec<String>> {
    std::fs::create_dir_all(root)?;
    if opts.include_site_css {
        std::fs::create_dir_all(root.join("css"))?;
    }
    if opts.include_js {
        std::fs::create_dir_all(root.join("components"))?;
        std::fs::create_dir_all(root.join("js"))?;
    }
    if matches!(opts.pages, NewSitePages::PhpModular) {
        std::fs::create_dir_all(root.join("includes"))?;
    }

    let title = escape_attr_min(&config.site.site_title);
    let subtitle = escape_attr_min(&config.site.site_subtitle);
    let footer = escape_attr_min(&config.footer.footer_text);

    let site_css_link = if opts.include_site_css {
        "\n  <link rel=\"stylesheet\" href=\"/css/site.css\" />"
    } else {
        ""
    };
    let js_links = if opts.include_js {
        "\n  <script src=\"/components/mor-card.js\" defer></script>\n  <script src=\"/js/site.js\" defer></script>\n  <script src=\"/mor-theme.js\" defer></script>"
    } else {
        "\n  <script src=\"/mor-theme.js\" defer></script>"
    };
    let about_href = match opts.pages {
        NewSitePages::PhpModular => "/about.php",
        NewSitePages::StaticHtml => "/about.html",
    };
    let about_nav = if opts.about_page {
        format!(r#"    <a class="mor-pill" href="{about_href}">About</a>"#)
    } else {
        String::new()
    };
    let card_block = if opts.include_js {
        r#"    <mor-card class="mor-card">
      <span slot="title">Web component</span>
      <p>This card themes via CSS variables from <code data-edit-target="typography.mono_font_stack">mor-theme.css</code> — no sealed hex colors.</p>
    </mor-card>
"#
    } else {
        ""
    };

    let mut files: Vec<(String, String)> = Vec::new();
    let mut pages: Vec<String> = Vec::new();
    let mut css_files: Vec<String> = Vec::new();
    let mut js_files: Vec<String> = Vec::new();

    match opts.pages {
        NewSitePages::PhpModular => {
            let header_php = format!(
                r##"<?php
// Modular chrome — swap markup freely; keep .mor-* hooks + data-mor-edit markers.
?><!doctype html>
<html lang="en" data-theme="dark" id="top">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title><?php echo htmlspecialchars($page_title ?? '{title}', ENT_QUOTES, 'UTF-8'); ?></title>
  <link rel="stylesheet" href="/mor-theme.css" />{site_css_link}{js_links}
</head>
<body>
<header class="main-header mor-topbar" data-edit-target="colors.bg_elevated">
  <a class="mor-brand" href="/">
    <span class="mor-brand-mark">◆</span>
    <span class="mor-brand-name" data-mor-edit="site.site_title" data-field-path="site.site_title">{title}</span>
  </a>
  <nav class="mor-nav" aria-label="Primary">
    <a class="mor-pill" href="/">Home</a>
{about_nav}
  </nav>
</header>
"##
            );
            let footer_php = format!(
                r##"<footer class="mor-footer mor-footer-hairline">
  <p>
    <span data-mor-edit="footer.footer_text" data-field-path="footer.footer_text">{footer}</span>
    <span aria-hidden="true">·</span>
    <a href="#top">back to top ↑</a>
  </p>
</footer>
</body>
</html>
"##
            );
            let index_php = format!(
                r##"<?php
$page_title = '{title}';
require __DIR__ . '/includes/header.php';
?>
<main class="canvas-core" style="max-width:720px;margin:2rem auto;padding:0 1.2rem;">
  <article class="mor-post" data-edit-target="colors.bg_panel">
    <h1 data-mor-edit="site.site_title" data-field-path="site.site_title" data-edit-target="typography.heading_font_stack">{title}</h1>
    <p data-mor-edit="site.site_subtitle" data-field-path="site.site_subtitle" data-edit-target="typography.body_font_stack">{subtitle}</p>
    <p>Open this folder in <strong>MorWebsite Editor</strong>. Pick a preset, tweak colors, switch the preview to <strong>Edit</strong> mode, and double-click the title.</p>
{card_block}    <h2>Site Contract</h2>
    <p>DRY tokens, WET structure. See <code>docs/SITE_CONTRACT.md</code> in the editor repo.</p>
  </article>
</main>
<?php require __DIR__ . '/includes/footer.php'; ?>
"##
            );
            files.push(("includes/header.php".into(), header_php));
            files.push(("includes/footer.php".into(), footer_php));
            files.push(("index.php".into(), index_php));
            pages.push("index.php".into());
            if opts.about_page {
                let about_php = format!(
                    r##"<?php
$page_title = 'About · {title}';
require __DIR__ . '/includes/header.php';
?>
<main class="canvas-core" style="max-width:720px;margin:2rem auto;padding:0 1.2rem;">
  <article class="mor-post">
    <h1 data-edit-target="typography.heading_font_stack">About</h1>
    <p data-edit-target="typography.body_font_stack">A second page on purpose — some HTML duplication is fine when both pages share the same <code>.mor-*</code> hooks and link the same theme file.</p>
    <p data-mor-edit="site.site_subtitle" data-field-path="site.site_subtitle">{subtitle}</p>
  </article>
</main>
<?php require __DIR__ . '/includes/footer.php'; ?>
"##
                );
                files.push(("about.php".into(), about_php));
                pages.push("about.php".into());
            }
        }
        NewSitePages::StaticHtml => {
            let head = format!(
                r##"<!doctype html>
<html lang="en" data-theme="dark" id="top">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{title}</title>
  <link rel="stylesheet" href="/mor-theme.css" />{site_css_link}{js_links}
</head>
<body>
<header class="main-header mor-topbar" data-edit-target="colors.bg_elevated">
  <a class="mor-brand" href="/">
    <span class="mor-brand-mark">◆</span>
    <span class="mor-brand-name" data-mor-edit="site.site_title" data-field-path="site.site_title">{title}</span>
  </a>
  <nav class="mor-nav" aria-label="Primary">
    <a class="mor-pill" href="/">Home</a>
{about_nav}
  </nav>
</header>
"##
            );
            let foot = format!(
                r##"<footer class="mor-footer mor-footer-hairline">
  <p>
    <span data-mor-edit="footer.footer_text" data-field-path="footer.footer_text">{footer}</span>
    <span aria-hidden="true">·</span>
    <a href="#top">back to top ↑</a>
  </p>
</footer>
</body>
</html>
"##
            );
            let index_html = format!(
                r##"{head}
<main class="canvas-core" style="max-width:720px;margin:2rem auto;padding:0 1.2rem;">
  <article class="mor-post" data-edit-target="colors.bg_panel">
    <h1 data-mor-edit="site.site_title" data-field-path="site.site_title" data-edit-target="typography.heading_font_stack">{title}</h1>
    <p data-mor-edit="site.site_subtitle" data-field-path="site.site_subtitle" data-edit-target="typography.body_font_stack">{subtitle}</p>
    <p>Open this folder in <strong>MorWebsite Editor</strong>. Pick a preset, tweak colors, and edit pages on disk.</p>
{card_block}  </article>
</main>
{foot}
"##
            );
            files.push(("index.html".into(), index_html));
            pages.push("index.html".into());
            if opts.about_page {
                let about_html = format!(
                    r##"{head}
<main class="canvas-core" style="max-width:720px;margin:2rem auto;padding:0 1.2rem;">
  <article class="mor-post">
    <h1 data-edit-target="typography.heading_font_stack">About</h1>
    <p data-edit-target="typography.body_font_stack">A second static page sharing the same theme tokens.</p>
    <p data-mor-edit="site.site_subtitle" data-field-path="site.site_subtitle">{subtitle}</p>
  </article>
</main>
{foot}
"##
                );
                files.push(("about.html".into(), about_html));
                pages.push("about.html".into());
            }
        }
    }

    if opts.include_site_css {
        let site_css = r##"/* Local site CSS — prefer variables so Mor presets restyle everything. */
body {
  margin: 0;
  background: var(--bg-base, #10161f);
  color: var(--fg-base, #ddd);
  font-family: var(--font-body, system-ui, sans-serif);
}
.mor-card {
  --card-bg: var(--bg-panel);
  --card-fg: var(--fg-base);
  --card-accent: var(--accent);
}
"##;
        files.push(("css/site.css".into(), site_css.into()));
        css_files.push("css/site.css".into());
    }

    if opts.include_js {
        let mor_card_js = r##"/**
 * <mor-card> — minimal web component that themes from CSS variables only.
 * Site Contract: no hard-coded hex; editor restyles via --card-* / --bg-panel.
 */
class MorCard extends HTMLElement {
  constructor() {
    super();
    const root = this.attachShadow({ mode: "open" });
    root.innerHTML = `
      <style>
        :host {
          display: block;
          margin: 1.2rem 0;
          padding: 1rem 1.15rem;
          border-radius: 10px;
          border: 1px solid var(--border-color, #333);
          background: var(--card-bg, var(--bg-panel, #151d29));
          color: var(--card-fg, var(--fg-base, #ddd));
          font-family: var(--font-body, system-ui, sans-serif);
        }
        .title {
          display: block;
          margin: 0 0 0.45rem;
          font-family: var(--font-heading, inherit);
          font-weight: 600;
          color: var(--card-accent, var(--accent, #7aa2f7));
        }
        ::slotted(p) { margin: 0; line-height: 1.55; opacity: 0.92; }
      </style>
      <span class="title"><slot name="title">Card</slot></span>
      <slot></slot>
    `;
  }
}
if (!customElements.get("mor-card")) {
  customElements.define("mor-card", MorCard);
}
"##;
        let site_js = r##"// Site scripts — keep behavior here; theme tokens stay in mor-theme.css.
document.documentElement.dataset.morReady = "1";
"##;
        files.push(("components/mor-card.js".into(), mor_card_js.into()));
        files.push(("js/site.js".into(), site_js.into()));
        js_files.push("components/mor-card.js".into());
        js_files.push("js/site.js".into());
    }

    let stack_label = match opts.pages {
        NewSitePages::PhpModular => "PHP (modular includes)",
        NewSitePages::StaticHtml => "Static HTML",
    };
    let readme = format!(
        r##"# {title}

Scaffolded with MorWebsite Editor ({stack_label}).

## Open in the editor

1. **File → Open Website Folder…** and pick this directory (or it may already be open).
2. Left dock → **Presets** / Theme Palette → load a look.
3. **File → Save Theme to Site** writes `workspace.toml` + `mor-theme.css`.

## Layout

- Pages: {pages_list}
- `workspace.toml` — editor theme tokens
- `mor-theme.css` — generated stylesheet
{extra_layout}
See `docs/SITE_CONTRACT.md` in the MorWebsite Editor repo.
"##,
        pages_list = pages.join(", "),
        extra_layout = {
            let mut lines = String::new();
            if matches!(opts.pages, NewSitePages::PhpModular) {
                lines.push_str("- `includes/header.php` / `footer.php` — shared chrome\n");
            }
            if opts.include_site_css {
                lines.push_str("- `css/site.css` — local rules using CSS variables\n");
            }
            if opts.include_js {
                lines.push_str("- `components/mor-card.js`, `js/site.js` — site JavaScript\n");
            }
            lines
        }
    );
    files.push(("README.md".into(), readme));
    files.push((
        "workspace.toml".into(),
        toml::to_string_pretty(config).unwrap_or_default(),
    ));

    let mut written = Vec::new();
    for (rel, content) in files {
        let path = root.join(&rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, content)?;
        written.push(rel);
    }

    let project = WebsiteProject {
        root: root.to_path_buf(),
        pages,
        css_files,
        js_files,
    };
    export_theme_bundle(&project, config)?;
    written.push(THEME_CSS_FILENAME.into());
    if root.join(THEME_JS_FILENAME).exists() {
        written.push(THEME_JS_FILENAME.into());
    }

    Ok(written)
}

/// How many of the project's pages already link the theme stylesheet.
/// `(linked, total)` — drives the "Link them" step's ✓ state in the UI.
pub fn count_linked_pages(project: &WebsiteProject) -> (usize, usize) {
    let mut linked = 0;
    for page in &project.pages {
        if let Ok(content) = std::fs::read_to_string(project.root.join(page)) {
            if content.contains(THEME_CSS_FILENAME) {
                linked += 1;
            }
        }
    }
    (linked, project.pages.len())
}

/// Idempotently add the theme `<link>` (and `<script>` when behaviors exist)
/// to every page that lacks them, inserted just before `</head>`. Pages
/// without a `</head>` are left untouched and reported in the second list.
/// Returns `(modified pages, skipped pages)`.
pub fn inject_theme_links(
    project: &WebsiteProject,
    config: &ThemeConfig,
) -> io::Result<(Vec<String>, Vec<String>)> {
    let has_js = !html_modules::generate_theme_js(config).is_empty();
    let mut modified = Vec::new();
    let mut skipped = Vec::new();
    for page in &project.pages {
        let path = project.root.join(page);
        let Ok(content) = std::fs::read_to_string(&path) else {
            skipped.push(page.clone());
            continue;
        };
        let mut insert = String::new();
        if !content.contains(THEME_CSS_FILENAME) {
            insert.push_str(&format!("  {}\n", theme_link_snippet()));
        }
        if has_js && !content.contains(THEME_JS_FILENAME) {
            insert.push_str(&format!("  {}\n", theme_js_snippet()));
        }
        if insert.is_empty() {
            continue;
        }
        match find_ci(&content, "</head>") {
            Some(pos) => {
                let mut updated = content;
                updated.insert_str(pos, &insert);
                std::fs::write(&path, updated)?;
                modified.push(page.clone());
            }
            None => skipped.push(page.clone()),
        }
    }
    Ok((modified, skipped))
}

/// Prepare fetched/loaded page HTML for the preview iframe:
///   * inject `<base href>` so relative assets resolve against the site server,
///   * stamp `data-theme` on `<html>` for dark-mode-aware sites,
///   * append the generated theme CSS as `<style id="mor-true-css">` last in
///     `<head>` — the id the PreviewCanvas morpher patches in place.
pub fn prepare_preview_html(
    raw_html: &str,
    base_href: &str,
    theme_css: &str,
    is_dark: bool,
) -> String {
    let mut html = raw_html.to_string();

    // data-theme stamp on the <html> tag (idempotent: skip if already present).
    if let Some(pos) = find_ci(&html, "<html") {
        let end = html[pos..].find('>').map(|e| pos + e);
        if let Some(end) = end {
            if !html[pos..end].contains("data-theme") {
                let stamp = format!(" data-theme=\"{}\"", if is_dark { "dark" } else { "light" });
                html.insert_str(end, &stamp);
            }
        }
    }

    let injection_head = format!("<base href=\"{}\" />", escape_attr_min(base_href));
    let injection_tail = format!("<style id=\"mor-true-css\">\n{theme_css}\n</style>");

    match find_ci(&html, "<head") {
        Some(pos) => {
            // <base> goes right after the opening <head...> tag,
            // the theme style right before </head> so it cascades last.
            if let Some(open_end) = html[pos..].find('>').map(|e| pos + e + 1) {
                html.insert_str(open_end, &injection_head);
            }
            if let Some(close) = find_ci(&html, "</head>") {
                html.insert_str(close, &injection_tail);
            } else {
                html.push_str(&injection_tail);
            }
        }
        None => {
            // Fragment / headless page: prepend a minimal head.
            html = format!("<head>{injection_head}{injection_tail}</head>{html}");
        }
    }
    html
}

/// Case-insensitive substring find (HTML tags may be any case).
fn find_ci(haystack: &str, needle: &str) -> Option<usize> {
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.is_empty() || h.len() < n.len() {
        return None;
    }
    (0..=h.len() - n.len()).find(|&i| h[i..i + n.len()].eq_ignore_ascii_case(n))
}

fn escape_attr_min(s: &str) -> String {
    s.replace('"', "&quot;")
}

/// Zip the whole project folder (binary-safe), skipping [`SKIP_DIRS`].
/// The freshly generated theme CSS is included even if never exported to disk.
pub fn zip_site(project: &WebsiteProject, config: &ThemeConfig, dest: &Path) -> io::Result<()> {
    use zip::write::SimpleFileOptions;
    let file = File::create(dest)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let ignore = load_ignore_rules(&project.root);
    let mut stack = vec![project.root.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let rel = path
                .strip_prefix(&project.root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if path.is_dir() {
                if name.starts_with('.')
                    || SKIP_DIRS.contains(&name.as_str())
                    || is_ignored(&rel, &ignore)
                    || is_app_install(&path)
                {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if is_ignored(&rel, &ignore) {
                continue;
            }
            if rel == THEME_CSS_FILENAME {
                continue; // fresh copy appended below
            }
            let mut buf = Vec::new();
            File::open(&path)?.read_to_end(&mut buf)?;
            zip.start_file(rel.as_str(), options)?;
            zip.write_all(&buf)?;
        }
    }
    zip.start_file(THEME_CSS_FILENAME, options)?;
    zip.write_all(generate_theme_css(config).as_bytes())?;
    zip.finish()?;
    Ok(())
}

/// Website-flavoured integrity check, replacing the Blogger XML analyzer:
///   * unresolved `{{TOKEN}}` sockets left in the generated CSS,
///   * pages that never link any stylesheet,
///   * `id`/`class` hooks styled by the theme CSS but absent from every page
///     (selector drift) — reported as warnings, capped to stay readable.
pub fn check_website(project: &WebsiteProject, config: &ThemeConfig) -> crate::diagnostics::DiagnosticResult {
    use crate::diagnostics::{DiagnosticResult, Warning};

    let css = generate_theme_css(config);
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Unresolved sockets are always a generator bug worth surfacing loudly.
    let mut leftover: Vec<&str> = Vec::new();
    let mut rest = css.as_str();
    while let Some(start) = rest.find("{{") {
        let tail = &rest[start..];
        if let Some(end) = tail.find("}}") {
            leftover.push(&tail[..end + 2]);
            rest = &tail[end + 2..];
        } else {
            break;
        }
    }
    leftover.dedup();
    for token in leftover.iter().take(10) {
        errors.push(format!("Unresolved CSS socket left in theme output: {token}"));
    }

    let mut pages_html = String::new();
    for page in project.pages.iter().take(50) {
        if let Ok(content) = std::fs::read_to_string(project.root.join(page)) {
            let links_css = content.contains("rel=\"stylesheet\"")
                || content.contains("rel='stylesheet'")
                || content.contains("<style");
            if !links_css {
                warnings.push(Warning::warn("website", format!(
                    "{page}: no stylesheet linked — theme CSS won't apply here"
                )));
            }
            pages_html.push_str(&content);
        }
    }

    // Selector drift: .mor-* hooks the theme styles that no page carries.
    let mut drift: Vec<String> = Vec::new();
    for chunk in css.split('.').skip(1) {
        let class: String = chunk
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if class.starts_with("mor-") && !pages_html.contains(&class) && !drift.contains(&class) {
            drift.push(class);
        }
    }
    if !drift.is_empty() && !pages_html.is_empty() {
        let shown = drift.iter().take(8).cloned().collect::<Vec<_>>().join(", ");
        warnings.push(Warning::warn("website", format!(
            "{} theme class hook(s) not found in any page (preset CSS targeting .mor-* applies only where those classes exist): {}",
            drift.len(),
            shown
        )));
    }

    DiagnosticResult {
        is_valid: errors.is_empty(),
        errors,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::defaults::default_theme_config;

    #[test]
    fn theme_css_includes_webfont_import_for_remote_faces() {
        let mut config = default_theme_config();
        config.typography.body_font_stack = "Inter".into();
        config.typography.font_provider = "google".into();
        let css = generate_theme_css(&config);
        assert!(
            css.starts_with("@import url("),
            "webfont @import must lead mor-theme.css"
        );
        assert!(css.contains("fonts.googleapis.com") || css.contains("Inter"));
        config.typography.font_provider = "none".into();
        let bare = generate_theme_css(&config);
        assert!(!bare.starts_with("@import url(\"https://fonts."));
    }

    #[test]
    fn theme_css_has_no_unresolved_sockets() {
        let css = generate_theme_css(&default_theme_config());
        assert!(!css.is_empty());
        assert!(!css.contains("{{"), "unresolved socket in: {css}");
        assert!(css.contains(":root"));
    }

    #[test]
    fn preview_injection_places_base_and_style() {
        let html = "<!doctype html><html><head><title>x</title></head><body>hi</body></html>";
        let out = prepare_preview_html(html, "http://127.0.0.1:8080/", "body{color:red}", true);
        assert!(out.contains("<base href=\"http://127.0.0.1:8080/\""));
        assert!(out.contains("<style id=\"mor-true-css\">"));
        assert!(out.contains("data-theme=\"dark\""));
        let base_pos = out.find("<base").unwrap();
        let style_pos = out.find("<style id=\"mor-true-css\"").unwrap();
        let head_close = out.find("</head>").unwrap();
        assert!(base_pos < style_pos && style_pos < head_close);
    }

    #[test]
    fn headless_fragment_gets_a_head() {
        let out = prepare_preview_html("<p>frag</p>", "http://x/", "b{}", false);
        assert!(out.starts_with("<head>"));
        assert!(out.contains("<p>frag</p>"));
    }

    #[test]
    fn theme_bundle_and_link_injection_roundtrip() {
        let dir = std::env::temp_dir().join(format!("mor_ws_export_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.html"), "<html><head><title>x</title></head><body></body></html>").unwrap();
        std::fs::write(dir.join("frag.html"), "<p>no head here</p>").unwrap();
        let project = scan_project(&dir).unwrap();
        let config = default_theme_config(); // default modules include TOC js

        let written = export_theme_bundle(&project, &config).unwrap();
        assert_eq!(written, vec![THEME_CSS_FILENAME.to_string(), THEME_JS_FILENAME.to_string()]);
        let css = std::fs::read_to_string(dir.join(THEME_CSS_FILENAME)).unwrap();
        assert!(css.contains("mor-topbar"), "module css folded into theme.css");

        let (modified, skipped) = inject_theme_links(&project, &config).unwrap();
        assert_eq!(modified, vec!["index.html".to_string()]);
        assert_eq!(skipped, vec!["frag.html".to_string()]);
        let page = std::fs::read_to_string(dir.join("index.html")).unwrap();
        assert!(page.contains(THEME_CSS_FILENAME) && page.contains(THEME_JS_FILENAME));
        // Idempotent: second run changes nothing.
        let (again, _) = inject_theme_links(&project, &config).unwrap();
        assert!(again.is_empty());
        assert_eq!(count_linked_pages(&project), (1, 2));

        // Dropping the TOC module removes the stale mor-theme.js.
        let mut no_js = config.clone();
        no_js.template_pack.right_sidebar_variant = html_modules::NONE_ID.into();
        export_theme_bundle(&project, &no_js).unwrap();
        assert!(!dir.join(THEME_JS_FILENAME).exists());

        export_starter_page(&project, &config, "Test").unwrap();
        assert!(dir.join(STARTER_PAGE_FILENAME).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_skips_app_installs_and_morignore_entries() {
        let dir = std::env::temp_dir().join(format!("mor_ws_ignore_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("wiki")).unwrap();
        std::fs::create_dir_all(dir.join("subdomain")).unwrap();
        std::fs::write(dir.join("index.html"), "<html></html>").unwrap();
        // MediaWiki install: recognized by LocalSettings.php, skipped wholesale.
        std::fs::write(dir.join("wiki/LocalSettings.php"), "<?php").unwrap();
        std::fs::write(dir.join("wiki/page.html"), "<html></html>").unwrap();
        // Subdomain folder: excluded via .morignore.
        std::fs::write(dir.join("subdomain/other.html"), "<html></html>").unwrap();
        std::fs::write(dir.join(".morignore"), "# comment\nsubdomain\n").unwrap();
        let p = scan_project(&dir).unwrap();
        assert_eq!(p.pages, vec!["index.html"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_inventories_and_ranks_index_first() {
        let dir = std::env::temp_dir().join(format!("mor_ws_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("about.html"), "<html></html>").unwrap();
        std::fs::write(dir.join("index.php"), "<?php ?>").unwrap();
        std::fs::write(dir.join("sub/page.html"), "<html></html>").unwrap();
        std::fs::write(dir.join("looks.css"), ":root{}").unwrap();
        std::fs::write(dir.join("app.js"), "//").unwrap();
        let p = scan_project(&dir).unwrap();
        assert_eq!(p.pages[0], "index.php");
        assert_eq!(p.pages.len(), 3);
        assert_eq!(p.css_files, vec!["looks.css"]);
        assert_eq!(p.js_files, vec!["app.js"]);
        assert_eq!(p.default_page(), Some("index.php"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scaffold_starter_writes_contract_site() {
        let dir = std::env::temp_dir().join(format!("mor_scaffold_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let config = default_theme_config();
        let written = scaffold_starter_site(&dir, &config).unwrap();
        assert!(written.iter().any(|f| f == "index.php"));
        assert!(written.iter().any(|f| f == THEME_CSS_FILENAME));
        let index = std::fs::read_to_string(dir.join("index.php")).unwrap();
        assert!(index.contains("data-mor-edit=\"site.site_title\""));
        assert!(index.contains("mor-card"));
        assert!(dir.join("includes/header.php").exists());
        assert!(dir.join("components/mor-card.js").exists());
        assert!(dir.join("workspace.toml").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scaffold_static_html_minimal() {
        let dir = std::env::temp_dir().join(format!("mor_scaffold_html_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let config = default_theme_config();
        let opts = NewSiteOptions {
            pages: NewSitePages::StaticHtml,
            include_site_css: true,
            include_js: false,
            about_page: false,
        };
        let written = scaffold_new_site(&dir, &config, &opts).unwrap();
        assert!(written.iter().any(|f| f == "index.html"));
        assert!(!written.iter().any(|f| f == "about.html"));
        assert!(!dir.join("includes").exists());
        assert!(!dir.join("components/mor-card.js").exists());
        assert!(dir.join("css/site.css").exists());
        assert!(dir.join("workspace.toml").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
