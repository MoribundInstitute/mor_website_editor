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
    /// Left sidebar / site nav item from `components/mor-nav-data.js`.
    NavLink,
    /// Unbound page DOM with instance fields (link, image, button) editable
    /// via unique HTML replace in the current page file.
    Instance,
    /// No path back to `ThemeConfig` — inspect only, never editable here.
    CodeOnly,
}

impl EditContext {
    pub fn name(self) -> &'static str {
        match self {
            EditContext::SiteField => "Site field",
            EditContext::Icon => "Icon slot",
            EditContext::Widget => "Widget",
            EditContext::TokenSurface => "Theme tokens",
            EditContext::Component => "Web component",
            EditContext::NavLink => "Nav link",
            EditContext::Instance => "Page element",
            EditContext::CodeOnly => "No binding",
        }
    }
}

/// A sidebar (or similar) nav item that maps into `MorNavData`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavLinkEdit {
    pub group: usize,
    pub item: usize,
    pub href: String,
    pub label: String,
}

/// In-page element attributes for the Inspector (saved into the page PHP/HTML).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct InstanceEdit {
    pub tag: String,
    pub classes: String,
    pub href: Option<String>,
    pub src: Option<String>,
    pub alt: Option<String>,
    pub text: Option<String>,
    /// Snapshot of `outerHTML` at select time (capped); unique-match for save.
    pub outer_html: Option<String>,
}

impl InstanceEdit {
    pub fn is_link(&self) -> bool {
        self.tag == "a" || self.href.is_some()
    }
    pub fn is_image(&self) -> bool {
        self.tag == "img" || self.src.is_some()
    }
    pub fn is_button_like(&self) -> bool {
        self.tag == "button"
            || self.classes.split_whitespace().any(|c| {
                c == "btn"
                    || c == "btn-primary"
                    || c.starts_with("btn-")
                    || c.contains("button")
            })
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
    /// For [`EditContext::NavLink`]: indices + current href/label.
    pub nav: Option<NavLinkEdit>,
    /// For page-instance edits (link / image / button) and token surfaces that
    /// still expose useful attributes.
    pub instance: Option<InstanceEdit>,
}

/// Facts from an unbound DOM click (preview canvas → workspace).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DomSelectFacts {
    pub tag: String,
    pub classes: String,
    pub href: Option<String>,
    pub src: Option<String>,
    pub alt: Option<String>,
    pub text: Option<String>,
    pub outer_html: Option<String>,
    pub nav_group: Option<usize>,
    pub nav_item: Option<usize>,
}

fn instance_from_facts(facts: &DomSelectFacts) -> InstanceEdit {
    InstanceEdit {
        tag: facts.tag.clone(),
        classes: facts.classes.clone(),
        href: facts.href.clone(),
        src: facts.src.clone(),
        alt: facts.alt.clone(),
        text: facts.text.clone(),
        outer_html: facts.outer_html.clone(),
    }
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
        nav: None,
        instance: None,
    }
}

/// Classify an unbound DOM click. Sidebar nav items become [`EditContext::NavLink`]
/// so the ribbon/Inspector can edit their href in `mor-nav-data.js`.
pub fn analyze_dom_facts(facts: &DomSelectFacts) -> SelectionInfo {
    let inst = instance_from_facts(facts);
    let is_sidebar_item = facts.classes.split_whitespace().any(|c| c == "mor-sidebar__item")
        || facts.classes.contains("mor-sidebar__item-label")
        || facts.classes.contains("mor-sidebar__item-icon");
    if let (Some(group), Some(item)) = (facts.nav_group, facts.nav_item) {
        let href = facts.href.clone().unwrap_or_default();
        let label = facts
            .text
            .clone()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| format!("item {group}.{item}"));
        return SelectionInfo {
            context: EditContext::NavLink,
            binding: Some(format!("nav[{group}].items[{item}].href")),
            palette_tab: None,
            label: format!("Nav link · {label}"),
            component: None,
            nav: Some(NavLinkEdit {
                group,
                item,
                href,
                label,
            }),
            instance: Some(inst),
        };
    }
    // Fallback: `<a class="mor-sidebar__item">` without indices still shows
    // as a link surface so the user can see the href even if data attrs missing.
    if is_sidebar_item {
        let href = facts.href.clone().unwrap_or_default();
        let label = facts.text.clone().unwrap_or_else(|| "Nav item".into());
        return SelectionInfo {
            context: EditContext::NavLink,
            binding: None,
            palette_tab: None,
            label: format!("Nav link · {label}"),
            component: None,
            nav: Some(NavLinkEdit {
                group: 0,
                item: 0,
                href,
                label,
            }),
            instance: Some(inst),
        };
    }

    // In-page link / image / button-like → Instance (page file edit).
    if facts.tag == "a" || facts.tag == "img" || facts.tag == "button" || inst.is_button_like() {
        let kind = if facts.tag == "img" {
            "Image"
        } else if facts.tag == "button" || inst.is_button_like() {
            "Button"
        } else {
            "Link"
        };
        let detail = facts
            .text
            .as_deref()
            .or(facts.alt.as_deref())
            .or(facts.href.as_deref())
            .or(facts.src.as_deref())
            .unwrap_or(facts.tag.as_str());
        return SelectionInfo {
            context: EditContext::Instance,
            binding: None,
            palette_tab: if facts.tag == "button" || inst.is_button_like() {
                Some("Buttons")
            } else {
                None
            },
            label: format!("{kind} · {detail}"),
            component: None,
            nav: None,
            instance: Some(inst),
        };
    }

    let mut sel = analyze_dom(&facts.tag, &facts.classes);
    sel.instance = Some(inst);
    sel
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
            nav: None,
            instance: None,
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
            nav: None,
            instance: None,
        },
        None => SelectionInfo {
            context: EditContext::CodeOnly,
            binding: None,
            palette_tab: None,
            label: format!("{sel} · no config binding — edit in your site files"),
            component: None,
            nav: None,
            instance: None,
        },
    }
}

/// Rebuild outer HTML after Inspector field edits. Returns `(old, new)` for
/// [`page_edit`](mor_website_core::website::page_edit) when a unique snapshot exists.
pub fn instance_rewrite(
    inst: &InstanceEdit,
    new_href: Option<&str>,
    new_src: Option<&str>,
    new_alt: Option<&str>,
    new_text: Option<&str>,
) -> Option<(String, String)> {
    let old = inst.outer_html.as_ref()?.clone();
    if old.len() > 12_000 {
        return None;
    }
    let mut html = old.clone();
    if let Some(h) = new_href {
        html = replace_attr(&html, "href", h);
    }
    if let Some(s) = new_src {
        html = replace_attr(&html, "src", s);
    }
    if let Some(a) = new_alt {
        html = replace_attr(&html, "alt", a);
    }
    if let Some(t) = new_text {
        if let Some(rewritten) = replace_element_text(&html, t) {
            html = rewritten;
        }
    }
    if html == old {
        return None;
    }
    Some((old, html))
}

fn replace_attr(html: &str, name: &str, value: &str) -> String {
    let escaped = value.replace('&', "&amp;").replace('"', "&quot;");
    // href="…"/href='…' or missing → insert after tag name
    let lower = html.to_ascii_lowercase();
    let needle_dq = format!("{name}=\"");
    let needle_sq = format!("{name}='");
    if let Some(at) = lower.find(&needle_dq) {
        let start = at + needle_dq.len();
        if let Some(rel) = html[start..].find('"') {
            let mut out = String::with_capacity(html.len());
            out.push_str(&html[..start]);
            out.push_str(&escaped);
            out.push_str(&html[start + rel..]);
            return out;
        }
    }
    if let Some(at) = lower.find(&needle_sq) {
        let start = at + needle_sq.len();
        if let Some(rel) = html[start..].find('\'') {
            let mut out = String::with_capacity(html.len());
            out.push_str(&html[..start]);
            out.push_str(&escaped.replace('\'', "&#39;"));
            out.push_str(&html[start + rel..]);
            return out;
        }
    }
    // Insert attribute after tag name: <a …> or <img …>
    if let Some(gt) = html.find('>') {
        let insert = format!(" {name}=\"{escaped}\"");
        let mut out = String::with_capacity(html.len() + insert.len());
        // Prefer before trailing / of void tags
        let at = if gt > 0 && html.as_bytes()[gt - 1] == b'/' {
            gt - 1
        } else {
            gt
        };
        out.push_str(&html[..at]);
        out.push_str(&insert);
        out.push_str(&html[at..]);
        return out;
    }
    html.to_string()
}

fn replace_element_text(html: &str, new_text: &str) -> Option<String> {
    let open_end = html.find('>')?;
    let close_start = html.rfind("</")?;
    if close_start <= open_end {
        return None;
    }
    // Skip void-ish
    let tag = html[1..open_end]
        .split_whitespace()
        .next()?
        .trim_end_matches('/')
        .to_ascii_lowercase();
    if matches!(
        tag.as_str(),
        "img" | "br" | "hr" | "input" | "meta" | "link" | "source"
    ) {
        return None;
    }
    let escaped = new_text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let mut out = String::with_capacity(html.len());
    out.push_str(&html[..=open_end]);
    out.push_str(&escaped);
    out.push_str(&html[close_start..]);
    Some(out)
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
    fn sidebar_nav_item_is_nav_link() {
        let facts = DomSelectFacts {
            tag: "a".into(),
            classes: "mor-sidebar__item".into(),
            href: Some("/Murdoverse.php".into()),
            text: Some("Murdoch Content".into()),
            nav_group: Some(2),
            nav_item: Some(2),
            ..Default::default()
        };
        let sel = analyze_dom_facts(&facts);
        assert_eq!(sel.context, EditContext::NavLink);
        let nav = sel.nav.expect("nav");
        assert_eq!(nav.href, "/Murdoverse.php");
        assert_eq!(nav.group, 2);
        assert_eq!(nav.item, 2);
    }

    #[test]
    fn in_page_link_is_instance() {
        let facts = DomSelectFacts {
            tag: "a".into(),
            classes: "btn-primary".into(),
            href: Some("/about.php".into()),
            text: Some("About".into()),
            outer_html: Some(r#"<a class="btn-primary" href="/about.php">About</a>"#.into()),
            ..Default::default()
        };
        let sel = analyze_dom_facts(&facts);
        assert_eq!(sel.context, EditContext::Instance);
        let (old, new) = instance_rewrite(
            sel.instance.as_ref().unwrap(),
            Some("/contact.php"),
            None,
            None,
            Some("Contact"),
        )
        .unwrap();
        assert!(old.contains("/about.php"));
        assert!(new.contains("/contact.php"));
        assert!(new.contains("Contact"));
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
