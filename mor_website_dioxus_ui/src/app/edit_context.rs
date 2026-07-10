//! Selection → edit-context analysis for the preview canvas (Phase 19).
//!
//! The LibreOffice sidebar reduces any selection to one discrete context enum
//! (SelectionAnalyzer → EnumContext) and lets panels react to it. This is the
//! Mor-sized version: a clicked preview node — either a bound marker path or
//! raw tag/class facts for unbound DOM — reduces to one [`EditContext`] plus
//! the Theme Palette accordion that owns it. Pure functions, unit-testable;
//! all signal/dock wiring stays in the workspace.

/// What kind of editing surface the selection is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditContext {
    /// Text/data binding into `ThemeConfig` (site title, footer text, widget
    /// titles) — the code editor reveal and dblclick text edit own it.
    SiteField,
    /// `icons.*` slot — the icon picker owns it (shift-click / SVG drop).
    Icon,
    /// `data-block-id` widget wrapper — draggable, title editable.
    Widget,
    /// Element styled by theme tokens; a Theme Palette panel owns it.
    TokenSurface,
    /// A custom element (`<my-card>`) — its PHP/CSS/JS parts are the unit
    /// of editing, linked by filename convention (see [`link_component`]).
    Component,
    /// No path back to `ThemeConfig` — inspect only, never editable here.
    CodeOnly,
}

impl EditContext {
    fn name(self) -> &'static str {
        match self {
            EditContext::SiteField => "Site field",
            EditContext::Icon => "Icon slot",
            EditContext::Widget => "Widget",
            EditContext::TokenSurface => "Theme tokens",
            EditContext::Component => "Web component",
            EditContext::CodeOnly => "No binding",
        }
    }
}

/// The three parts of a web component, resolved to project-relative paths.
/// A part is `None` when no file in the project matches the tag by name.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComponentLink {
    pub tag: String,
    pub php: Option<String>,
    pub css: Option<String>,
    pub js: Option<String>,
}

/// Link a custom-element tag to its PHP/CSS/JS parts by filename convention:
/// a file whose stem equals the tag name (`components/my-card.php`,
/// `css/my-card.css`, `js/my-card.js`) is that component's part. Convention
/// over manifest — zero config, and wrong guesses are impossible to act on
/// destructively (the chips only open editors).
pub fn link_component(
    tag: &str,
    pages: &[String],
    css_files: &[String],
    js_files: &[String],
) -> ComponentLink {
    let find = |files: &[String]| {
        files
            .iter()
            .find(|f| {
                std::path::Path::new(f)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.eq_ignore_ascii_case(tag))
            })
            .cloned()
    };
    ComponentLink {
        tag: tag.to_string(),
        php: find(pages),
        css: find(css_files),
        js: find(js_files),
    }
}

/// One analyzed selection: the context plus what the UI needs to react —
/// which palette accordion to focus and what to print in the inspect chip.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectionInfo {
    pub context: EditContext,
    /// Config path when the node carries a binding (e.g. `site.site_title`).
    pub binding: Option<String>,
    /// `EditorAccordion` id inside the Theme Palette dock to focus, if any.
    pub palette_tab: Option<&'static str>,
    /// Human chip text, e.g. "Site field · site.site_title".
    pub label: String,
    /// For [`EditContext::Component`]: the tag's PHP/CSS/JS parts. Filled by
    /// the workspace (it owns the project file lists), not the analyzer.
    pub component: Option<ComponentLink>,
}

/// A marker-carrying node was clicked: classify its binding path.
pub fn analyze_bound(target: &str) -> SelectionInfo {
    let head = target.split('.').next().unwrap_or(target);
    let (context, palette_tab) = match head {
        "icons" => (EditContext::Icon, None),
        "colors" => (EditContext::TokenSurface, Some("Colors")),
        "buttons" => (EditContext::TokenSurface, Some("Buttons")),
        "typography" => (EditContext::TokenSurface, Some("Typography")),
        "background" => (EditContext::TokenSurface, Some("Background")),
        h if h.starts_with("scrollbar") => (EditContext::TokenSurface, Some("Scrollbars")),
        h if h.starts_with("cursor") => (EditContext::TokenSurface, Some("Cursors")),
        "site" | "footer" | "menu_links" | "seo" => (EditContext::SiteField, None),
        "widget" => (EditContext::Widget, None),
        // Unknown-but-bound: still a config path, offer the code reveal.
        _ => (EditContext::SiteField, None),
    };
    SelectionInfo {
        context,
        binding: Some(target.to_string()),
        palette_tab,
        label: format!("{} · {}", context.name(), target),
        component: None,
    }
}

/// An unbound DOM node was clicked (no marker anywhere up its chain):
/// classify by tag/class only. Everything that theme tokens style maps to
/// its palette panel; media and unknowns are inspect-only — Mor never edits
/// DOM it has no config path for.
pub fn analyze_dom(tag: &str, classes: &str) -> SelectionInfo {
    // Custom elements are their own context: the component's PHP/CSS/JS
    // parts are the editing surface, not theme tokens.
    if tag.contains('-') {
        return SelectionInfo {
            context: EditContext::Component,
            binding: None,
            palette_tab: None,
            label: format!("<{tag}> · web component"),
            component: None, // workspace joins with project files
        };
    }
    let palette_tab = match tag {
        "button" | "input" | "select" | "textarea" => Some("Buttons"),
        _ if classes.contains("mor-pill") || classes.contains("btn") || classes.contains("button") => {
            Some("Buttons")
        }
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "p" | "li" | "span" | "em" | "strong"
        | "blockquote" | "pre" | "code" | "small" | "figcaption" | "td" | "th" | "label" => {
            Some("Typography")
        }
        "a" => Some("Colors"),
        "body" | "html" | "header" | "footer" | "nav" | "main" | "aside" | "section"
        | "article" | "div" | "ul" | "ol" | "form" | "table" => Some("Colors"),
        _ => None, // img, svg, video, custom elements, …
    };
    let sel = if classes.is_empty() {
        format!("<{tag}>")
    } else {
        format!("<{tag}> .{}", classes.split_whitespace().next().unwrap_or(""))
    };
    match palette_tab {
        Some(tab) => SelectionInfo {
            context: EditContext::TokenSurface,
            binding: None,
            palette_tab: Some(tab),
            label: format!("{sel} · {tab} tokens"),
            component: None,
        },
        None => SelectionInfo {
            context: EditContext::CodeOnly,
            binding: None,
            palette_tab: None,
            label: format!("{sel} · no config binding — edit in your site files"),
            component: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bound_paths_map_to_owning_panel() {
        assert_eq!(analyze_bound("colors.accent").palette_tab, Some("Colors"));
        assert_eq!(analyze_bound("buttons.radius").palette_tab, Some("Buttons"));
        assert_eq!(analyze_bound("scrollbar_thumb_color").palette_tab, Some("Scrollbars"));
        let icon = analyze_bound("icons.search");
        assert_eq!(icon.context, EditContext::Icon);
        assert_eq!(icon.binding.as_deref(), Some("icons.search"));
        assert_eq!(analyze_bound("site.site_title").context, EditContext::SiteField);
        assert_eq!(analyze_bound("widget.Label1.title").context, EditContext::Widget);
    }

    #[test]
    fn unbound_dom_classifies_by_tag_and_never_binds() {
        assert_eq!(analyze_dom("h2", "").palette_tab, Some("Typography"));
        assert_eq!(analyze_dom("button", "").palette_tab, Some("Buttons"));
        assert_eq!(analyze_dom("nav", "site-nav").palette_tab, Some("Colors"));
        assert_eq!(analyze_dom("div", "mor-pill x").palette_tab, Some("Buttons"));
        let img = analyze_dom("img", "hero");
        assert_eq!(img.context, EditContext::CodeOnly);
        assert_eq!(img.binding, None);
        assert!(img.label.contains("no config binding"));
    }

    #[test]
    fn custom_elements_are_components_linked_by_filename() {
        let sel = analyze_dom("my-card", "fancy");
        assert_eq!(sel.context, EditContext::Component);
        assert_eq!(sel.binding, None);

        let pages = vec!["index.php".into(), "components/my-card.php".into()];
        let css = vec!["css/site.css".into(), "css/My-Card.css".into()];
        let js: Vec<String> = vec![];
        let link = link_component("my-card", &pages, &css, &js);
        assert_eq!(link.php.as_deref(), Some("components/my-card.php"));
        assert_eq!(link.css.as_deref(), Some("css/My-Card.css")); // case-insensitive
        assert_eq!(link.js, None);

        // no matching files → all parts None, never a guess
        let none = link_component("their-widget", &pages, &css, &js);
        assert_eq!((none.php, none.css, none.js), (None, None, None));
    }

    #[test]
    fn empty_selection_body_click_still_has_a_context() {
        // LO: an empty slide still has a page context. Clicking blank page
        // background lands on <body> → site-surface colors, not a dead end.
        assert_eq!(analyze_dom("body", "").palette_tab, Some("Colors"));
    }
}
