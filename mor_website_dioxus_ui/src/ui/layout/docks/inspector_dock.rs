//! Inspector dock — what you clicked on the canvas.
//!
//! Scope: one selection (link, image, nav item, theme token surface, component).
//! Site-wide tokens stay in Theme Palette; whole-page tools stay in Page.

use crate::app::edit_context::{instance_rewrite, EditContext, SelectionInfo};
use crate::app::shell::WorkbenchEditState;
use crate::app::state::{CenterView, DockPosition, LayoutState, WebsiteState};
use crate::ui::components::dock_chrome::DockChrome;
use dioxus::prelude::*;

#[component]
pub fn InspectorDock() -> Element {
    let mut layout = use_context::<LayoutState>();
    let pos = (layout.inspector_dock_pos)();

    if pos == DockPosition::Hidden {
        return rsx! {};
    }

    let sel = (layout.active_canvas_selection)();

    rsx! {
        crate::ui_kit::MorPanelWrapper {
            position: pos,
            default_position: DockPosition::mor_panel_right,
            DockChrome {
                title: "Inspector".to_string(),
                dock_id: "inspector".to_string(),
                position: pos,
                on_close: move |_| {
                    layout.inspector_dock_pos.set(DockPosition::Hidden);
                },
                section {
                    class: "inspector-dock-panel",
                    style: "padding: 12px; height: calc(100% - 45px); overflow-y: auto; background: var(--bg-panel); color: var(--fg-base); font-size: 0.85rem; display: flex; flex-direction: column; gap: 12px;",

                    p {
                        style: "margin: 0; font-size: 0.75rem; line-height: 1.45; color: var(--fg-muted);",
                        "Click anything in "
                        strong { "Edit" }
                        " or "
                        strong { "Inspect" }
                        " mode. Theme colors: Theme Palette (Alt+T). Page tools: Page (Alt+N)."
                    }

                    match sel {
                        None => rsx! {
                            div { class: "editor-note",
                                p { class: "editor-note-title", "Nothing selected" }
                                p { class: "editor-note-body",
                                    "Click a heading, link, image, button, sidebar item, or theme surface in the preview."
                                }
                            }
                        },
                        Some(info) => rsx! {
                            div {
                                style: "padding: 10px 12px; border-radius: 8px; border: 1px solid var(--editor-border-soft, #333); background: color-mix(in srgb, var(--editor-accent, #6d8fb8) 12%, transparent);",
                                div {
                                    style: "font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.04em; color: var(--fg-muted); margin-bottom: 4px;",
                                    "{info.context.name()}"
                                }
                                div { style: "font-weight: 600; line-height: 1.35; word-break: break-word;",
                                    "{info.label}"
                                }
                                if let Some(ref b) = info.binding {
                                    div {
                                        style: "margin-top: 6px; font-family: var(--editor-mono, monospace); font-size: 0.72rem; color: var(--fg-muted); word-break: break-all;",
                                        "{b}"
                                    }
                                }
                            }
                            InspectorBody { info: info }
                        },
                    }
                }
            }
        }
    }
}

#[component]
fn InspectorBody(info: SelectionInfo) -> Element {
    let mut layout = use_context::<LayoutState>();
    match info.context {
        EditContext::NavLink => rsx! { NavLinkInspector { info: info.clone() } },
        EditContext::Instance => rsx! { InstanceInspector { info: info.clone() } },
        EditContext::Component => rsx! { ComponentInspector { info: info.clone() } },
        EditContext::TokenSurface => rsx! { TokenInspector { info: info.clone() } },
        EditContext::SiteField => rsx! {
            div { class: "editor-note",
                p { class: "editor-note-body",
                    "Site-wide field bound to ThemeConfig. Double-click in the preview to edit text, or change tokens in Theme Palette."
                }
            }
            if let Some(tab) = info.palette_tab {
                button {
                    class: "editor-button",
                    onclick: move |_| layout.focus_palette_panel(tab),
                    "Open {tab} in Theme Palette…"
                }
            }
        },
        EditContext::Icon => rsx! {
            div { class: "editor-note",
                p { class: "editor-note-body",
                    "Icon slot. Shift-click the icon in the preview to replace it, or use the icon picker."
                }
            }
            if let Some(ref b) = info.binding {
                button {
                    class: "editor-button",
                    onclick: {
                        let b = b.clone();
                        move |_| layout.active_icon_picker.set(Some(b.clone()))
                    },
                    "Change icon…"
                }
            }
        },
        EditContext::Widget => rsx! {
            div { class: "editor-note",
                p { class: "editor-note-body",
                    "Widget block. Drag to reorder in Edit mode; open Code for markup."
                }
            }
        },
        EditContext::CodeOnly => rsx! {
            div { class: "editor-note",
                p { class: "editor-note-body",
                    "No editor binding for this node. Double-click text to edit page content, or open Code (Page dock) for the PHP/HTML source."
                }
            }
            button {
                class: "editor-mini-button",
                onclick: move |_| {
                    layout.center_view.set(CenterView::CodeEditor);
                },
                "Open Code view"
            }
        },
    }
}

#[component]
fn TokenInspector(info: SelectionInfo) -> Element {
    let mut layout = use_context::<LayoutState>();
    let tab = info.palette_tab.unwrap_or("Colors");
    rsx! {
        div { class: "editor-note",
            p { class: "editor-note-body",
                "This element is styled by site-wide theme tokens (not this page alone). Change "
                strong { "{tab}" }
                " in Theme Palette to restyle every matching surface."
            }
        }
        button {
            class: "editor-button editor-button-good",
            onclick: move |_| layout.focus_palette_panel(tab),
            "Open Theme · {tab}…"
        }
        if let Some(ref inst) = info.instance {
            if inst.is_link() || inst.is_image() || inst.is_button_like() {
                p {
                    style: "margin: 0; font-size: 0.75rem; color: var(--fg-muted);",
                    "Tip: click the link/image/button itself for page-local URL / alt / label fields."
                }
            }
        }
        p {
            style: "margin: 0; font-size: 0.75rem; color: var(--fg-muted);",
            "Double-click text on this element to edit page content only."
        }
    }
}

#[component]
fn ComponentInspector(info: SelectionInfo) -> Element {
    let mut layout = use_context::<LayoutState>();
    let Some(link) = info.component.clone() else {
        return rsx! {
            p { style: "margin: 0; color: var(--fg-muted);", "Custom element — no linked parts found." }
        };
    };
    let tag_label = format!("<{}>", link.tag);
    rsx! {
        p {
            style: "margin: 0; font-size: 0.78rem; color: var(--fg-muted);",
            "Web component "
            code { "{tag_label}" }
            ". Parts match files named like the tag."
        }
        div { style: "display: flex; flex-wrap: wrap; gap: 6px;",
            if let Some(css) = link.css.clone() {
                button {
                    class: "editor-mini-button",
                    onclick: move |_| {
                        layout.css_editor_open_file.set(Some(css.clone()));
                        layout.request_dock("css_editor", DockPosition::mor_panel_left);
                    },
                    "CSS"
                }
            }
            if let Some(js) = link.js.clone() {
                button {
                    class: "editor-mini-button",
                    onclick: move |_| {
                        layout.js_editor_open_file.set(Some(js.clone()));
                        layout.request_dock("js_editor", DockPosition::mor_panel_left);
                    },
                    "Script"
                }
            }
            if let Some(php) = link.php.clone() {
                span {
                    class: "editor-mini-button editor-mini-button-disabled",
                    title: "{php}",
                    "PHP · {php}"
                }
            }
            if link.css.is_none() && link.js.is_none() && link.php.is_none() {
                span { style: "font-size: 0.75rem; color: var(--fg-muted);",
                    "No matching .php/.css/.js for this tag"
                }
            }
        }
    }
}

#[component]
fn NavLinkInspector(info: SelectionInfo) -> Element {
    let website = use_context::<WebsiteState>();
    let mut layout = use_context::<LayoutState>();
    let mut edit_state = use_context::<WorkbenchEditState>();
    let Some(nav) = info.nav.clone() else {
        return rsx! {};
    };
    let mut href = use_signal(|| nav.href.clone());
    let mut label = use_signal(|| nav.label.clone());
    let group = nav.group;
    let item = nav.item;

    use_effect(move || {
        if let Some(s) = (layout.active_canvas_selection)() {
            if let Some(n) = s.nav {
                href.set(n.href);
                label.set(n.label);
            }
        }
    });

    let mut save = move |new_href: Option<String>, new_label: Option<String>| {
        let project = website.project.peek().clone();
        match crate::app::services::workspace_service::handle_nav_link_edit(
            &project,
            group,
            item,
            new_href.as_deref(),
            new_label.as_deref(),
        ) {
            Ok(()) => {
                if let Some(mut s) = (layout.active_canvas_selection)() {
                    if let Some(ref mut n) = s.nav {
                        if let Some(ref h) = new_href {
                            n.href = h.clone();
                        }
                        if let Some(ref l) = new_label {
                            n.label = l.clone();
                            s.label = format!("Nav link · {l}");
                        }
                    }
                    layout.active_canvas_selection.set(Some(s));
                }
                website.bump_preview();
                edit_state.workbench_status.set("Nav link saved".into());
            }
            Err(e) => edit_state.workbench_status.set(e),
        }
    };

    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 8px;",
            h4 { style: "margin: 0; font-size: 0.72rem; letter-spacing: 0.04em; color: var(--fg-muted);",
                "Sidebar nav (mor-nav-data.js)"
            }
            label { style: "display: flex; flex-direction: column; gap: 4px; font-size: 0.78rem; color: var(--fg-muted);",
                "Label"
                input {
                    class: "editor-input",
                    value: "{label}",
                    oninput: move |e| label.set(e.value()),
                    onblur: move |_| save(None, Some(label())),
                }
            }
            label { style: "display: flex; flex-direction: column; gap: 4px; font-size: 0.78rem; color: var(--fg-muted);",
                "URL"
                input {
                    class: "editor-input",
                    value: "{href}",
                    placeholder: "/page.php or https://…",
                    oninput: move |e| href.set(e.value()),
                    onblur: move |_| save(Some(href()), None),
                }
            }
            button {
                class: "editor-mini-button",
                onclick: move |_| save(Some(href()), Some(label())),
                "Save nav link"
            }
        }
    }
}

#[component]
fn InstanceInspector(info: SelectionInfo) -> Element {
    let website = use_context::<WebsiteState>();
    let mut layout = use_context::<LayoutState>();
    let mut edit_state = use_context::<WorkbenchEditState>();
    let Some(inst) = info.instance.clone() else {
        return rsx! {};
    };

    let mut href = use_signal(|| inst.href.clone().unwrap_or_default());
    let mut src = use_signal(|| inst.src.clone().unwrap_or_default());
    let mut alt = use_signal(|| inst.alt.clone().unwrap_or_default());
    let mut text = use_signal(|| inst.text.clone().unwrap_or_default());
    let show_href = inst.is_link() || inst.is_button_like();
    let show_src = inst.is_image();
    let show_alt = inst.is_image();
    let show_text = !inst.is_image();
    let has_snapshot = inst.outer_html.as_ref().is_some_and(|s| !s.is_empty());
    let palette_tab = info.palette_tab;

    use_effect(move || {
        if let Some(s) = (layout.active_canvas_selection)() {
            if let Some(i) = s.instance {
                href.set(i.href.unwrap_or_default());
                src.set(i.src.unwrap_or_default());
                alt.set(i.alt.unwrap_or_default());
                text.set(i.text.unwrap_or_default());
            }
        }
    });

    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 8px;",
            h4 { style: "margin: 0; font-size: 0.72rem; letter-spacing: 0.04em; color: var(--fg-muted);",
                "This page only (PHP/HTML)"
            }
            if !has_snapshot {
                p {
                    style: "margin: 0; font-size: 0.75rem; color: var(--editor-warning, #d29922);",
                    "No HTML snapshot — re-click the element, or double-click text to edit."
                }
            }
            if show_text {
                label { style: "display: flex; flex-direction: column; gap: 4px; font-size: 0.78rem; color: var(--fg-muted);",
                    "Label / text"
                    input {
                        class: "editor-input",
                        value: "{text}",
                        oninput: move |e| text.set(e.value()),
                    }
                }
            }
            if show_href {
                label { style: "display: flex; flex-direction: column; gap: 4px; font-size: 0.78rem; color: var(--fg-muted);",
                    "URL (href)"
                    input {
                        class: "editor-input",
                        value: "{href}",
                        placeholder: "/page.php or https://…",
                        oninput: move |e| href.set(e.value()),
                    }
                }
            }
            if show_src {
                label { style: "display: flex; flex-direction: column; gap: 4px; font-size: 0.78rem; color: var(--fg-muted);",
                    "Image src"
                    input {
                        class: "editor-input",
                        value: "{src}",
                        oninput: move |e| src.set(e.value()),
                    }
                }
            }
            if show_alt {
                label { style: "display: flex; flex-direction: column; gap: 4px; font-size: 0.78rem; color: var(--fg-muted);",
                    "Alt text"
                    input {
                        class: "editor-input",
                        value: "{alt}",
                        placeholder: "Describe the image…",
                        oninput: move |e| alt.set(e.value()),
                    }
                }
            }
            button {
                class: "editor-button editor-button-good",
                disabled: !has_snapshot,
                onclick: move |_| {
                    let Some(sel) = (layout.active_canvas_selection)() else {
                        return;
                    };
                    let Some(inst) = sel.instance.as_ref() else {
                        return;
                    };
                    let href_s = href();
                    let src_s = src();
                    let alt_s = alt();
                    let text_s = text();
                    let Some((old, new)) = instance_rewrite(
                        inst,
                        if show_href { Some(href_s.as_str()) } else { None },
                        if show_src { Some(src_s.as_str()) } else { None },
                        if show_alt { Some(alt_s.as_str()) } else { None },
                        if show_text { Some(text_s.as_str()) } else { None },
                    ) else {
                        edit_state.workbench_status.set(
                            "No change, or missing HTML snapshot — double-click text or use Code"
                                .into(),
                        );
                        return;
                    };
                    let project = website.project.peek().clone();
                    let page = website
                        .current_page
                        .peek()
                        .clone()
                        .or_else(|| project.default_page().map(str::to_string));
                    let Some(page) = page else {
                        edit_state.workbench_status.set("No page selected".into());
                        return;
                    };
                    match crate::app::services::workspace_service::handle_page_text_edit(
                        &project, &page, &old, &new,
                    ) {
                        Ok(true) => {
                            website.bump_preview();
                            if let Some(mut s) = (layout.active_canvas_selection)() {
                                if let Some(ref mut i) = s.instance {
                                    i.outer_html = Some(new.clone());
                                    if show_href {
                                        i.href = Some(href_s.clone());
                                    }
                                    if show_src {
                                        i.src = Some(src_s.clone());
                                    }
                                    if show_alt {
                                        i.alt = Some(alt_s.clone());
                                    }
                                    if show_text {
                                        i.text = Some(text_s.clone());
                                    }
                                }
                                let detail = if !text_s.trim().is_empty() {
                                    text_s
                                } else {
                                    href_s
                                };
                                s.label = format!("{} · {detail}", s.context.name());
                                layout.active_canvas_selection.set(Some(s));
                            }
                            edit_state
                                .workbench_status
                                .set(format!("Saved element → {page}"));
                        }
                        Ok(false) => {}
                        Err(e) => edit_state.workbench_status.set(e),
                    }
                },
                "Save to page"
            }
            if let Some(tab) = palette_tab {
                button {
                    class: "editor-mini-button",
                    onclick: move |_| layout.focus_palette_panel(tab),
                    "Theme style · {tab}…"
                }
            }
            p {
                style: "margin: 0; font-size: 0.72rem; color: var(--fg-muted); line-height: 1.4;",
                "Saves with PHP-aware unique HTML match. If save fails, use double-click edit or Code view."
            }
        }
    }
}
