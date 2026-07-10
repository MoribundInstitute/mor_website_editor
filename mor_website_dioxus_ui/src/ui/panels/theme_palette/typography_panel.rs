//! Typography panel — LibreOffice-style.
//!
//! Font pickers are custom dropdowns that preview each face in its own
//! typeface (the panel loads all registry Google Fonts via @import so previews
//! are accurate). Rich controls (size, weight, line-height) sit inline, a live
//! preview pane reflects the current settings, and a per-element section lets
//! each of H1/H2/H3/Body/Quote/Code override size, weight, italic and color.
//! Per-element overrides resolve to CSS in `build_element_typography_css`.

use dioxus::document;
use dioxus::prelude::*;

use crate::app::state::ThemeState;
use crate::ui::components::inputs::EditorInput;
use mor_website_core::config::fonts::{
    all_registry_google_fonts_url, resolve_font_stack_with_fallback, webfont_stylesheet_href,
    FontPreset, FontProvider, FONT_REGISTRY, MONO_FONT_REGISTRY,
};
use mor_website_core::config::ElementStyle;

/// Fixed elements offered in the per-element section:
/// (css key, tab label, full label, default font-size used by the live preview).
const ELEMENTS: &[(&str, &str, &str, &str)] = &[
    ("h1", "H1", "Heading 1", "1.9em"),
    ("h2", "H2", "Heading 2", "1.45em"),
    ("h3", "H3", "Heading 3", "1.15em"),
    ("h4", "H4", "Heading 4", "1.05em"),
    ("h5", "H5", "Heading 5", "0.95em"),
    ("h6", "H6", "Heading 6", "0.85em"),
    ("p", "Body", "Body text", "1em"),
    ("blockquote", "Quote", "Blockquote", "1em"),
    ("code", "Code", "Inline / code", "0.9em"),
];

const SIZE_PRESETS: &[&str] = &[
    "12px", "13px", "14px", "16px", "18px", "20px", "24px", "1rem", "1.125rem", "1.25rem",
];

const WEIGHTS: &[(&str, &str)] = &[
    ("Thin", "100"),
    ("ExtraLight", "200"),
    ("Light", "300"),
    ("Regular", "400"),
    ("Medium", "500"),
    ("Semibold", "600"),
    ("Bold", "700"),
    ("ExtraBold", "800"),
    ("Black", "900"),
];

const SECTION_H: &str = "color: var(--editor-accent, #a9aae2); margin: 4px 0 2px 0; font-size: 13px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.5px;";

/// Dropdown/option/weight-button styling. Kept free of `&` so it survives as a
/// plain `<style>` text node (Dioxus escapes `&` in text — see the font link,
/// which is injected as a real <link> attribute instead).
const PANEL_CSS: &str = "\
.mor-font-option { padding: 7px 10px; cursor: pointer; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-size: 15px; }\
.mor-font-option:hover { background: color-mix(in srgb, var(--accent, #5a5a5c) 35%, transparent); }\
.mor-font-option[data-selected=\"true\"] { background: color-mix(in srgb, var(--accent, #5a5a5c) 25%, transparent); }\
.mor-weight-btn { flex: 1; padding: 6px 4px; font-size: 12px; cursor: pointer; background: var(--bg-elevated, #2c2c2e); color: var(--fg-base, #ddd); border: 1px solid var(--editor-border-soft, #3a3a3c); }\
.mor-weight-btn[data-active=\"true\"] { background: var(--accent, #5a5a5c); color: #fff; }";

#[component]
pub fn TypographyPanel() -> Element {
    let mut app_state = use_context::<ThemeState>();
    let signals = app_state.signals;

    // Editor-side font loading (opt-in: only fetched while this panel is mounted).
    // Injected as a real <link> so the many `&family=` separators in the URL are
    // NOT HTML-escaped (an inline `@import` text node would become `&amp;` and 404).
    let fonts_url = all_registry_google_fonts_url();
    let provider = FontProvider::from_config(&signals.font_provider.read());
    // Also load the active selection from the chosen host so custom names preview.
    let active_preview_url = {
        let body = resolve_font_stack_with_fallback(&signals.body_font_stack.read(), false);
        let heading = if signals.heading_font_stack.read().trim().is_empty() {
            body.clone()
        } else {
            resolve_font_stack_with_fallback(&signals.heading_font_stack.read(), false)
        };
        let mono = resolve_font_stack_with_fallback(&signals.mono_font_stack.read(), true);
        webfont_stylesheet_href(&[body.as_str(), heading.as_str(), mono.as_str()], provider)
    };

    rsx! {
        if !fonts_url.is_empty() {
            document::Stylesheet { href: fonts_url.clone() }
        }
        if !active_preview_url.is_empty() && active_preview_url != fonts_url {
            document::Stylesheet { href: active_preview_url }
        }
        style { "{PANEL_CSS}" }

        section { class: "editor-card",

            p {
                class: "editor-mini-label",
                title: "Any CSS font-family is allowed — registry, Google/Bunny name, full stack, or self-hosted @font-face.",
                "Website typography is free-form. Pick a preset, type any family name, or paste a full CSS stack. ⓘ"
            }

            // --- Webfont host ------------------------------------------------
            h4 { style: "{SECTION_H}", "Webfont host" }
            p {
                class: "editor-mini-label",
                style: "margin-top: 0;",
                "Remote fonts land as @import at the top of mor-theme.css. Choose None to use system fonts or your own @font-face."
            }
            div { style: "display: flex; flex-wrap: wrap; gap: 4px; margin-bottom: 10px;",
                for (label, key, tip) in [
                    ("Bunny (privacy)", "bunny", "fonts.bunny.net — same catalog as Google, no tracking"),
                    ("Google Fonts", "google", "fonts.googleapis.com"),
                    ("None / self-host", "none", "No remote import — system stacks or custom @font-face only"),
                ] {
                    button {
                        class: "mor-weight-btn",
                        style: "flex: 1 1 auto; min-width: 0;",
                        title: "{tip}",
                        "data-active": "{signals.font_provider.read().eq_ignore_ascii_case(key)}",
                        onclick: move |_| signals.font_provider.clone().set(key.to_string()),
                        "{label}"
                    }
                }
            }

            // --- Fonts -------------------------------------------------------
            h4 { style: "{SECTION_H}", "Font families" }
            FontPreviewSelect {
                label: "Body Font".to_string(),
                value: signals.body_font_stack,
                options: FONT_REGISTRY,
                include_match_body: false,
                is_mono: false,
            }
            FontPreviewSelect {
                label: "Heading Font".to_string(),
                value: signals.heading_font_stack,
                options: FONT_REGISTRY,
                include_match_body: true,
                is_mono: false,
            }
            FontPreviewSelect {
                label: "Monospace Font".to_string(),
                value: signals.mono_font_stack,
                options: MONO_FONT_REGISTRY,
                include_match_body: false,
                is_mono: true,
            }

            // --- Custom CSS (self-host / extra imports) ---------------------
            h4 { style: "{SECTION_H} margin-top: 12px;", "Custom font CSS" }
            p {
                class: "editor-mini-label",
                style: "margin-top: 0;",
                "Optional. Paste @font-face rules or an extra @import (Bunny/Google). Prepended into mor-theme.css after the host import."
            }
            textarea {
                class: "editor-field",
                style: "width: 100%; min-height: 72px; font-family: var(--font-mono, ui-monospace, monospace); font-size: 0.78rem; resize: vertical;",
                placeholder: "@font-face {{\n  font-family: 'MyBrand';\n  src: url('/fonts/MyBrand.woff2') format('woff2');\n  font-display: swap;\n}}",
                value: "{signals.custom_font_css}",
                oninput: move |e| signals.custom_font_css.clone().set(e.value()),
            }

            // --- Base controls ----------------------------------------------
            h4 { style: "{SECTION_H} margin-top: 12px;", "Base" }
            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div { class: "editor-field-group",
                    label { class: "editor-field-label", "Base Size" }
                    input {
                        r#type: "text",
                        class: "editor-field",
                        list: "mor-type-sizes",
                        value: "{signals.base_size}",
                        placeholder: "16px",
                        oninput: move |e| signals.base_size.clone().set(e.value()),
                    }
                    datalist { id: "mor-type-sizes",
                        for s in SIZE_PRESETS.iter() {
                            option { value: "{s}" }
                        }
                    }
                }
                EditorInput {
                    label: "Line Height".to_string(),
                    value: signals.line_height,
                    input_type: "text".to_string(),
                    placeholder: "1.6".to_string(),
                }
            }

            div { class: "editor-field-group",
                label { class: "editor-field-label", "Heading Weight" }
                div { style: "display: flex; flex-wrap: wrap; gap: 4px;",
                    for (name, w) in WEIGHTS.iter() {
                        button {
                            class: "mor-weight-btn",
                            style: "flex: 0 0 auto; min-width: 3.2rem;",
                            title: "{name} ({w})",
                            "data-active": "{signals.heading_weight.read().as_str() == *w}",
                            onclick: move |_| signals.heading_weight.clone().set(w.to_string()),
                            "{w}"
                        }
                    }
                }
            }

            // --- Live preview ------------------------------------------------
            h4 { style: "{SECTION_H} margin-top: 12px;", "Preview" }
            TypographyPreview { signals }

            // --- Per-element overrides --------------------------------------
            h4 { style: "{SECTION_H} margin-top: 12px;", "Per-Element Styles" }
            p { class: "editor-mini-label", style: "margin-top: 0;", "Pick an element, then override only what you need. Blank = inherit. Applies to preview and export." }
            ElementStyleEditor { elements: signals.type_elements }

            button {
                class: "mor-btn-secondary",
                style: "width: 100%; margin-top: 14px;",
                onclick: move |_| app_state.show_advanced_typography.set(true),
                "⚙ Local font files & type scale"
            }
        }
    }
}

/// Live sample, reflecting both the global signals AND per-element overrides
/// (so the preview is true WYSIWYG for the per-element editor below).
#[component]
fn TypographyPreview(signals: crate::app::theme_signals::ThemeSignals) -> Element {
    let body_ff = resolve_font_stack_with_fallback(&signals.body_font_stack.read(), false);
    let heading_raw = signals.heading_font_stack.read().clone();
    let heading_ff = if heading_raw.trim().is_empty() {
        body_ff.clone()
    } else {
        resolve_font_stack_with_fallback(&heading_raw, false)
    };
    let mono_ff = resolve_font_stack_with_fallback(&signals.mono_font_stack.read(), true);
    let base = signals.base_size.read().clone();
    let lh = signals.line_height.read().clone();
    let hweight = signals.heading_weight.read().clone();

    let els = signals.type_elements.read().clone();
    let ov = |sel: &str| els.iter().find(|e| e.selector == sel).cloned();

    let s_h1 = sample_style(&heading_ff, "1.9em", &hweight, ov("h1").as_ref(), "");
    let s_h2 = sample_style(&heading_ff, "1.45em", &hweight, ov("h2").as_ref(), "");
    let s_h3 = sample_style(&heading_ff, "1.15em", &hweight, ov("h3").as_ref(), "");
    let s_p = sample_style(&body_ff, "1em", "", ov("p").as_ref(), "");
    let s_quote = sample_style(
        &body_ff,
        "1em",
        "",
        ov("blockquote").as_ref(),
        "padding-left: 12px; border-left: 3px solid #ccc; color: #555; font-style: italic;",
    );
    let s_code = sample_style(
        &mono_ff,
        "0.9em",
        "",
        ov("code").as_ref(),
        "background: #f0f0f0; padding: 1px 5px; border-radius: 3px;",
    );

    rsx! {
        div {
            style: "background: #fff; color: #111; border: 1px solid var(--editor-border-soft, #3a3a3c); border-radius: 6px; padding: 14px; font-family: {body_ff}; font-size: {base}; line-height: {lh};",
            h1 { style: "{s_h1} margin: 0 0 6px 0;", "Heading One" }
            h2 { style: "{s_h2} margin: 0 0 6px 0;", "Heading Two" }
            h3 { style: "{s_h3} margin: 0 0 8px 0;", "Heading Three" }
            p { style: "{s_p} margin: 0 0 8px 0;",
                "The quick brown fox jumps over the lazy dog — typography sets the tone of the whole page."
            }
            blockquote { style: "{s_quote} margin: 0 0 8px 0;",
                "A blockquote shows how quoted passages will look."
            }
            p { style: "{s_p} margin: 0;",
                "Inline "
                code { style: "{s_code}", "monospace code" }
                " sample."
            }
        }
    }
}

/// Build an inline style for one preview sample: base defaults merged with the
/// element's override (override wins per-property), plus any static `extra`.
fn sample_style(
    family: &str,
    default_size: &str,
    default_weight: &str,
    ov: Option<&ElementStyle>,
    extra: &str,
) -> String {
    let nz = |s: &str| {
        let t = s.trim();
        (!t.is_empty()).then(|| t.to_string())
    };
    let size = ov
        .and_then(|o| nz(&o.font_size))
        .unwrap_or_else(|| default_size.to_string());
    let mut s = format!("font-family: {}; font-size: {};", family, size);

    let weight = ov
        .and_then(|o| nz(&o.font_weight))
        .unwrap_or_else(|| default_weight.to_string());
    if !weight.is_empty() {
        s.push_str(&format!(" font-weight: {};", weight));
    }
    if let Some(o) = ov {
        if let Some(v) = nz(&o.line_height) {
            s.push_str(&format!(" line-height: {};", v));
        }
        if let Some(v) = nz(&o.letter_spacing) {
            s.push_str(&format!(" letter-spacing: {};", v));
        }
        if let Some(v) = nz(&o.color) {
            s.push_str(&format!(" color: {};", v));
        }
        if let Some(v) = nz(&o.background) {
            s.push_str(&format!(" background: {};", v));
        }
        if let Some(v) = nz(&o.padding) {
            s.push_str(&format!(" padding: {};", v));
        }
        if let Some(v) = nz(&o.border_radius) {
            s.push_str(&format!(" border-radius: {};", v));
        }
        if let Some(v) = nz(&o.text_align) {
            s.push_str(&format!(" text-align: {};", v));
        }
        if o.italic {
            s.push_str(" font-style: italic;");
        }
    }
    if !extra.is_empty() {
        s.push(' ');
        s.push_str(extra);
    }
    s
}

/// LibreOffice-style per-element editor: pick an element via the tab row, then
/// edit its full style with room to breathe (replaces the cramped grid rows).
#[component]
fn ElementStyleEditor(elements: Signal<Vec<ElementStyle>>) -> Element {
    let mut active = use_signal(|| 0usize);
    let idx = active().min(ELEMENTS.len() - 1);
    let (selector, _tab, full, _def) = ELEMENTS[idx];

    let cur = elements
        .read()
        .iter()
        .find(|e| e.selector == selector)
        .cloned()
        .unwrap_or(ElementStyle {
            selector: selector.to_string(),
            ..Default::default()
        });
    let italic_now = cur.italic;
    let has_any = elements.read().iter().any(|e| e.selector == selector);

    rsx! {
        // Element tabs (• marks elements that carry an override).
        div { style: "display: flex; flex-wrap: wrap; gap: 4px; margin-bottom: 10px;",
            for (i, (sel, tab, _f, _d)) in ELEMENTS.iter().enumerate() {
                button {
                    class: "mor-weight-btn",
                    style: "flex: 0 0 auto; min-width: 42px;",
                    "data-active": "{i == idx}",
                    onclick: move |_| active.set(i),
                    "{tab}"
                    if elements.read().iter().any(|e| e.selector == *sel) {
                        span { style: "color: var(--accent, #8ab4f8); margin-left: 3px;", "•" }
                    }
                }
            }
        }

        div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 10px;",
            div { class: "editor-field-group", style: "margin: 0;",
                label { class: "editor-field-label", "Size" }
                input {
                    r#type: "text", class: "editor-field", list: "mor-type-sizes",
                    value: "{cur.font_size}", placeholder: "inherit",
                    oninput: move |e| update_element(elements, selector, |s| s.font_size = e.value()),
                }
            }
            div { class: "editor-field-group", style: "margin: 0;",
                label { class: "editor-field-label", "Weight" }
                select {
                    class: "editor-select",
                    value: "{cur.font_weight}",
                    onchange: move |e| update_element(elements, selector, |s| s.font_weight = e.value()),
                    option { value: "", selected: cur.font_weight.is_empty(), "Inherit" }
                    for (name, w) in WEIGHTS.iter() {
                        option { value: "{w}", selected: cur.font_weight == *w, "{name} ({w})" }
                    }
                }
            }
            div { class: "editor-field-group", style: "margin: 0;",
                label { class: "editor-field-label", "Line Height" }
                input {
                    r#type: "text", class: "editor-field",
                    value: "{cur.line_height}", placeholder: "inherit",
                    oninput: move |e| update_element(elements, selector, |s| s.line_height = e.value()),
                }
            }
            div { class: "editor-field-group", style: "margin: 0;",
                label { class: "editor-field-label", "Letter Spacing" }
                input {
                    r#type: "text", class: "editor-field",
                    value: "{cur.letter_spacing}", placeholder: "e.g. 0.5px",
                    oninput: move |e| update_element(elements, selector, |s| s.letter_spacing = e.value()),
                }
            }
            // Box treatment: turns a heading into a "plaque" (chip background,
            // padding, rounded corners, centering) — no preset CSS needed.
            div { class: "editor-field-group", style: "margin: 0;",
                label { class: "editor-field-label", "Background" }
                input {
                    r#type: "text", class: "editor-field",
                    value: "{cur.background}", placeholder: "color or gradient",
                    oninput: move |e| update_element(elements, selector, |s| s.background = e.value()),
                }
            }
            div { class: "editor-field-group", style: "margin: 0;",
                label { class: "editor-field-label", "Padding" }
                input {
                    r#type: "text", class: "editor-field",
                    value: "{cur.padding}", placeholder: "e.g. 20px",
                    oninput: move |e| update_element(elements, selector, |s| s.padding = e.value()),
                }
            }
            div { class: "editor-field-group", style: "margin: 0;",
                label { class: "editor-field-label", "Corner Radius" }
                input {
                    r#type: "text", class: "editor-field",
                    value: "{cur.border_radius}", placeholder: "e.g. 8px",
                    oninput: move |e| update_element(elements, selector, |s| s.border_radius = e.value()),
                }
            }
            div { class: "editor-field-group", style: "margin: 0;",
                label { class: "editor-field-label", "Text Align" }
                select {
                    class: "editor-select",
                    value: "{cur.text_align}",
                    onchange: move |e| update_element(elements, selector, |s| s.text_align = e.value()),
                    option { value: "", selected: cur.text_align.is_empty(), "Inherit" }
                    for align in ["left", "center", "right"] {
                        option { value: "{align}", selected: cur.text_align == align, "{align}" }
                    }
                }
            }
        }

        div { style: "display: flex; gap: 8px; align-items: center; margin-top: 8px;",
            button {
                class: "mor-weight-btn",
                style: "flex: 0 0 auto; min-width: 64px; font-style: italic;",
                "data-active": "{italic_now}",
                onclick: move |_| update_element(elements, selector, move |s| s.italic = !italic_now),
                "Italic"
            }
            input {
                r#type: "text", class: "editor-field", style: "flex: 1; margin: 0;",
                value: "{cur.color}", placeholder: "color (hex, blank = inherit)",
                oninput: move |e| update_element(elements, selector, |s| s.color = e.value()),
            }
            if has_any {
                button {
                    class: "mor-btn-secondary",
                    style: "flex: 0 0 auto;",
                    title: "Clear {full} overrides",
                    onclick: move |_| { elements.write().retain(|e| e.selector != selector); },
                    "Clear"
                }
            }
        }
    }
}

/// Find-or-create the override for `selector`, apply `f`, then drop it again if
/// it ended up all-default (keeps saved configs clean).
fn update_element(
    mut elements: Signal<Vec<ElementStyle>>,
    selector: &str,
    f: impl FnOnce(&mut ElementStyle),
) {
    let mut v = elements.write();
    let idx = v.iter().position(|e| e.selector == selector);
    let mut entry = match idx {
        Some(i) => v[i].clone(),
        None => ElementStyle {
            selector: selector.to_string(),
            ..Default::default()
        },
    };
    f(&mut entry);

    let is_default = entry.font_size.trim().is_empty()
        && entry.font_weight.trim().is_empty()
        && entry.line_height.trim().is_empty()
        && entry.letter_spacing.trim().is_empty()
        && entry.color.trim().is_empty()
        && entry.background.trim().is_empty()
        && entry.padding.trim().is_empty()
        && entry.border_radius.trim().is_empty()
        && entry.text_align.trim().is_empty()
        && !entry.italic;

    match idx {
        Some(i) if is_default => {
            v.remove(i);
        }
        Some(i) => v[i] = entry,
        None if is_default => {}
        None => v.push(entry),
    }
}

/// Font picker: searchable registry + always-visible free CSS stack field.
/// Any family name or full `font-family` stack is valid for website export.
#[component]
fn FontPreviewSelect(
    label: String,
    value: Signal<String>,
    options: &'static [FontPreset],
    include_match_body: bool,
    is_mono: bool,
) -> Element {
    let mut value = value;
    let mut open = use_signal(|| false);
    let mut filter = use_signal(String::new);

    let current = value.read().clone();
    let current_trimmed = current.trim().to_string();
    let q = filter.read().trim().to_ascii_lowercase();

    let matched = options.iter().find(|f| {
        f.name.eq_ignore_ascii_case(&current_trimmed)
            || f.css_stack.eq_ignore_ascii_case(&current_trimmed)
    });
    let is_match_body = include_match_body && current_trimmed.is_empty();

    let preview_ff = if is_match_body {
        "inherit".to_string()
    } else if current_trimmed.is_empty() {
        "inherit".to_string()
    } else {
        resolve_font_stack_with_fallback(&current_trimmed, is_mono)
    };

    let display = if is_match_body {
        "Match body".to_string()
    } else if let Some(f) = matched {
        f.name.to_string()
    } else if current_trimmed.is_empty() {
        "Type or pick a font…".to_string()
    } else {
        primary_display(&current_trimmed)
    };

    let filtered: Vec<&FontPreset> = options
        .iter()
        .filter(|f| {
            q.is_empty()
                || f.name.to_ascii_lowercase().contains(&q)
                || f.category.to_ascii_lowercase().contains(&q)
        })
        .collect();

    rsx! {
        div { class: "editor-field-group", style: "position: relative;",
            label { class: "editor-field-label", "{label}" }

            // Always-editable CSS stack / family name (website freedom).
            input {
                r#type: "text",
                class: "editor-field",
                style: "width: 100%; font-family: {preview_ff}; margin-bottom: 6px;",
                value: "{current}",
                placeholder: if is_mono {
                    "e.g. JetBrains Mono · or ui-monospace, Menlo, monospace"
                } else {
                    "e.g. Inter · Raleway · 'My Brand', system-ui, sans-serif"
                },
                title: "Any CSS font-family value. Registry names, Google/Bunny faces, or a full stack.",
                oninput: move |e| value.set(e.value()),
            }

            button {
                class: "editor-select",
                style: "width: 100%; text-align: left; display: flex; justify-content: space-between; align-items: center; cursor: pointer; font-family: {preview_ff};",
                title: "Browse the preset catalog",
                onclick: move |_| { let o = *open.read(); open.set(!o); },
                span { style: "overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", "Catalog: {display}" }
                span { style: "opacity: 0.6; font-family: initial; margin-left: 6px;", "▾" }
            }

            if open() {
                div {
                    style: "position: fixed; inset: 0; z-index: 40;",
                    onclick: move |_| open.set(false),
                }
                div {
                    style: "position: absolute; left: 0; right: 0; top: 100%; z-index: 50; max-height: 360px; overflow-y: auto; margin-top: 4px; background: var(--bg-elevated, #2c2c2e); border: 1px solid var(--editor-border-soft, #3a3a3c); border-radius: 6px; box-shadow: 0 8px 24px rgba(0,0,0,0.45);",

                    div { style: "padding: 8px 8px 4px; position: sticky; top: 0; background: var(--bg-elevated, #2c2c2e); z-index: 1;",
                        input {
                            r#type: "search",
                            class: "editor-field",
                            style: "width: 100%; font-size: 0.85rem;",
                            placeholder: "Search fonts…",
                            value: "{filter}",
                            oninput: move |e| filter.set(e.value()),
                            onclick: move |e| e.stop_propagation(),
                        }
                    }

                    if include_match_body {
                        div {
                            class: "mor-font-option",
                            "data-selected": "{is_match_body}",
                            onclick: move |_| { value.set(String::new()); open.set(false); filter.set(String::new()); },
                            "Match body"
                        }
                    }

                    for (i, font) in filtered.iter().enumerate() {
                        {
                            let show_cat = i == 0 || filtered[i - 1].category != font.category;
                            let name = font.name;
                            let stack = font.css_stack;
                            let cat = font.category;
                            let is_web = font.google_font_name.is_some();
                            let selected = matched.map(|m| m.name == font.name).unwrap_or(false);
                            rsx! {
                                if show_cat {
                                    div {
                                        key: "cat-{cat}-{i}",
                                        style: "padding: 6px 10px 2px; font-size: 10px; text-transform: uppercase; letter-spacing: 0.5px; opacity: 0.5; font-family: initial;",
                                        "{cat}"
                                    }
                                }
                                div {
                                    key: "{name}-{i}",
                                    class: "mor-font-option",
                                    style: "font-family: {stack};",
                                    "data-selected": "{selected}",
                                    title: "{stack}",
                                    onclick: move |_| {
                                        // Store the full stack so export is self-describing.
                                        value.set(stack.to_string());
                                        open.set(false);
                                        filter.set(String::new());
                                    },
                                    "{name}"
                                    if is_web {
                                        span {
                                            style: "font-family: initial; font-size: 10px; opacity: 0.45; margin-left: 6px;",
                                            "web"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if filtered.is_empty() {
                        div {
                            style: "padding: 10px; font-size: 0.8rem; opacity: 0.7; font-family: initial;",
                            "No catalog match — type any name in the field above."
                        }
                    }
                }
            }
        }
    }
}

fn primary_display(stack: &str) -> String {
    let p = stack
        .split(',')
        .next()
        .unwrap_or(stack)
        .trim()
        .trim_matches(|c| c == '\'' || c == '"');
    if p.len() > 36 {
        format!("{}…", &p[..36])
    } else {
        p.to_string()
    }
}
