//! Preview edit-mode ribbon (Phase 19, LO-Draw-Notebookbar-inspired).
//!
//! Two rows: modes + Home/View/Selection tabs; then tool groups for the
//! active tab (palette swatches, device frame, selection props).
//! File/app commands live in the **menu bar** — not duplicated here.
//!
//! Binding law: inline controls write `ThemeSignals` / `ThemeConfig`.

use dioxus::prelude::*;

use crate::app::edit_context::{EditContext, SelectionInfo};
use crate::app::state::{DockPosition, LayoutState, ThemeState};
use crate::ui::components::icons::{tool_paths, ToolIcon};
use crate::ui::workspace::layout::{
    apply_preview_viewport, clamp_preview_width, is_landscape, rotate_preview_width,
    PreviewViewport,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum RibbonTab {
    Home,
    View,
    Selection,
}

/// Browse | Inspect | Edit. Two booleans, three states: Browse looks and
/// navigates, Inspect adds outlines + selection, Edit additionally arms the
/// mutating gestures (text dblclick, widget drag, SVG drop, icon shift-click).
#[component]
pub fn PreviewModeTabs(
    mut is_xray_active: Signal<bool>,
    mut is_edit_active: Signal<bool>,
) -> Element {
    let mode = match (is_xray_active(), is_edit_active()) {
        (false, _) => "browse",
        (true, false) => "inspect",
        (true, true) => "edit",
    };
    let seg = |active: bool| {
        if active {
            "editor-mini-button mor-mode-btn is-active"
        } else {
            "editor-mini-button mor-mode-btn"
        }
    };
    rsx! {
        div {
            class: "editor-segmented mor-mode-seg",
            role: "group",
            "aria-label": "Preview mode",
            button {
                class: seg(mode == "browse"),
                title: "Browse — links navigate, nothing selectable",
                onclick: move |_| { is_xray_active.set(false); is_edit_active.set(false); },
                ToolIcon { d: tool_paths::BROWSE }
                span { "Browse" }
            }
            button {
                class: seg(mode == "inspect"),
                title: "Inspect — hover outlines, click selects and focuses the owning panel",
                onclick: move |_| { is_xray_active.set(true); is_edit_active.set(false); },
                ToolIcon { d: tool_paths::INSPECT }
                span { "Inspect" }
            }
            button {
                class: seg(mode == "edit"),
                title: "Edit — Inspect plus dbl-click text, drag widgets, drop/shift-click icons",
                onclick: move |_| { is_xray_active.set(true); is_edit_active.set(true); },
                ToolIcon { d: tool_paths::EDIT_PEN }
                span { "Edit" }
            }
        }
    }
}

/// A compact inline color control bound to one theme token signal.
#[component]
fn RibbonSwatch(label: &'static str, value: Signal<String>) -> Element {
    let mut value = value;
    rsx! {
        label {
            class: "preview-ribbon-swatch",
            title: "{label}",
            input {
                r#type: "color",
                value: "{value}",
                oninput: move |e| value.set(e.value()),
            }
            span { "{label}" }
        }
    }
}

/// Instrument-cluster caption naming a tool group.
#[component]
fn GroupCaption(text: &'static str) -> Element {
    rsx! {
        span { class: "mor-tool-caption", "{text}" }
    }
}

#[component]
pub fn PreviewRibbon(
    preview_viewport: Signal<PreviewViewport>,
    preview_width: Signal<u32>,
    is_xray_active: Signal<bool>,
    is_edit_active: Signal<bool>,
    active_selection: Signal<Option<SelectionInfo>>,
) -> Element {
    let mut active_tab = use_signal(|| RibbonTab::Home);

    let has_selection = is_xray_active() && active_selection().is_some();
    // LO tab behavior: a fresh selection context auto-switches to its
    // contextual tab; when the context disappears, fall back to Home.
    use_effect(move || {
        let now = is_xray_active() && active_selection().is_some();
        if now {
            active_tab.set(RibbonTab::Selection);
        } else if *active_tab.peek() == RibbonTab::Selection {
            active_tab.set(RibbonTab::Home);
        }
    });
    let tab = if !has_selection && active_tab() == RibbonTab::Selection {
        RibbonTab::Home // render fallback before the effect lands
    } else {
        active_tab()
    };

    // The mode hairline: the ribbon carries its mode as a class; CSS keys
    // the bottom rule and active segment color off it.
    let mode_class = match (is_xray_active(), is_edit_active()) {
        (false, _) => "mode-browse",
        (true, false) => "mode-inspect",
        (true, true) => "mode-edit",
    };

    let selection_tab_label = active_selection()
        .map(|s| match s.context {
            EditContext::TokenSurface => s.palette_tab.unwrap_or("Selection"),
            EditContext::Icon => "Icon",
            EditContext::Widget => "Widget",
            EditContext::Component => "Component",
            EditContext::SiteField => "Site field",
            EditContext::CodeOnly => "Selection",
        })
        .unwrap_or("Selection");

    let tab_btn = |t: RibbonTab, current: RibbonTab| {
        if t == current {
            "editor-mini-button mor-ribbon-tab is-active"
        } else {
            "editor-mini-button mor-ribbon-tab"
        }
    };

    rsx! {
        div {
            class: "preview-ribbon {mode_class}",
            // Row 1 — modes + ribbon tabs only.
            div {
                class: "preview-ribbon-row",
                PreviewModeTabs { is_xray_active, is_edit_active }
                div { class: "preview-toolbar-divider" }
                button {
                    class: tab_btn(RibbonTab::Home, tab),
                    onclick: move |_| active_tab.set(RibbonTab::Home),
                    "Home"
                }
                button {
                    class: tab_btn(RibbonTab::View, tab),
                    onclick: move |_| active_tab.set(RibbonTab::View),
                    "View"
                }
                if has_selection {
                    button {
                        class: tab_btn(RibbonTab::Selection, tab),
                        onclick: move |_| active_tab.set(RibbonTab::Selection),
                        ToolIcon { d: tool_paths::SELECTION, size: 12 }
                        span { "{selection_tab_label}" }
                    }
                }
            }
            // Row 2 — the active tab's tool groups.
            div {
                class: "preview-ribbon-row",
                match tab {
                    RibbonTab::Home => rsx! { HomeTabGroups {} },
                    RibbonTab::View => rsx! { ViewTabGroups { preview_viewport, preview_width } },
                    RibbonTab::Selection => rsx! { SelectionTabGroups { active_selection } },
                }
            }
        }
    }
}

/// Home: the site-level context (an empty selection still has one) — quick
/// palette swatches and type basics, with jumps into the full panels.
#[component]
fn HomeTabGroups() -> Element {
    let theme = use_context::<ThemeState>();
    let mut layout = use_context::<LayoutState>();
    let signals = theme.signals;
    rsx! {
        div {
            class: "preview-toolbar-group",
            style: "margin: 0;",
            GroupCaption { text: "Palette" }
            RibbonSwatch { label: "Accent", value: signals.accent }
            RibbonSwatch { label: "Background", value: signals.bg_base }
            RibbonSwatch { label: "Text", value: signals.fg_base }
            RibbonSwatch { label: "Border", value: signals.border }
            button {
                class: "editor-mini-button",
                title: "Open the full Color Palette panel",
                onclick: move |_| layout.focus_palette_panel("Colors"),
                "Colors…"
            }
        }
        div {
            class: "preview-toolbar-group",
            style: "margin: 0;",
            GroupCaption { text: "Type" }
            label {
                class: "preview-width-control",
                span { class: "preview-width-label", "Base font" }
                input {
                    class: "preview-width-input",
                    r#type: "text",
                    style: "width: 52px;",
                    value: "{signals.base_size}",
                    oninput: move |e| signals.base_size.clone().set(e.value()),
                }
            }
            button {
                class: "editor-mini-button",
                title: "Open the full Typography panel",
                onclick: move |_| layout.focus_palette_panel("Typography"),
                "Typography…"
            }
        }
    }
}

/// View: the device frame controls.
#[component]
fn ViewTabGroups(
    mut preview_viewport: Signal<PreviewViewport>,
    mut preview_width: Signal<u32>,
) -> Element {
    // The "rotate" control is really a portrait <-> landscape toggle for the
    // device frame (you can't rotate a website); the tooltip carries the
    // direction it switches TO.
    let rotatable = preview_viewport().is_rotatable();
    let landscape = is_landscape(preview_viewport(), preview_width());
    let rotate_title = if !rotatable {
        "Orientation — pick Tablet, Phone, or Custom first"
    } else if landscape {
        "Switch to portrait"
    } else {
        "Switch to landscape"
    };

    let device = |vp: PreviewViewport, icon: &'static str, title: &'static str| {
        let active = preview_viewport() == vp;
        rsx! {
            button {
                class: if active { "editor-mini-button mor-tool-btn is-active" } else { "editor-mini-button mor-tool-btn" },
                title,
                onclick: move |_| { apply_preview_viewport(vp, preview_width); preview_viewport.set(vp); },
                ToolIcon { d: icon }
            }
        }
    };

    rsx! {
        div {
            class: "preview-toolbar-group",
            style: "margin: 0;",
            GroupCaption { text: "Device" }
            div {
                class: "editor-segmented",
                {device(PreviewViewport::Desktop, tool_paths::DESKTOP, "Desktop")}
                {device(PreviewViewport::Laptop, tool_paths::LAPTOP, "Laptop")}
                {device(PreviewViewport::Tablet, tool_paths::TABLET, "Tablet")}
                {device(PreviewViewport::Phone, tool_paths::PHONE, "Phone")}
                {device(PreviewViewport::Fit, tool_paths::FIT_WIDTH, "Fit to viewport")}
            }
            button {
                class: if rotatable { "editor-mini-button mor-tool-btn" } else { "editor-mini-button mor-tool-btn editor-mini-button-disabled" },
                title: rotate_title,
                onclick: move |_| { if preview_viewport().is_rotatable() { preview_width.set(rotate_preview_width(preview_viewport(), preview_width())); } },
                ToolIcon { d: tool_paths::ROTATE }
            }
            label {
                class: "preview-width-control",
                span { class: "preview-width-label", "Width" }
                input {
                    class: "preview-width-input", r#type: "number", min: "240", max: "2400", step: "10", value: "{preview_width()}",
                    oninput: move |evt| {
                        if let Ok(width_value) = evt.value().parse::<u32>() {
                            preview_width.set(clamp_preview_width(width_value));
                            preview_viewport.set(PreviewViewport::Custom);
                        }
                    },
                }
            }
        }
    }
}

/// The contextual tab's groups, keyed by the selection's [`EditContext`] —
/// inline token controls for the common surfaces, actions for the rest.
#[component]
fn SelectionTabGroups(active_selection: Signal<Option<SelectionInfo>>) -> Element {
    let mut layout = use_context::<LayoutState>();
    let theme = use_context::<ThemeState>();
    let signals = theme.signals;

    let Some(sel) = active_selection() else {
        return rsx! {};
    };

    // Bring an asset editor dock into view and select a file in it.
    let mut open_asset = move |dock_id: &'static str, file: String| {
        let (pos_sig, mut open_req) = match dock_id {
            "css_editor" => (layout.css_editor_pos, layout.css_editor_open_file),
            _ => (layout.js_editor_pos, layout.js_editor_open_file),
        };
        let pos = pos_sig();
        if pos == DockPosition::Hidden {
            layout.request_dock(dock_id, layout.preferred_dock_position(dock_id));
        } else {
            layout.request_dock(dock_id, pos); // re-focus its zone tab
        }
        open_req.set(Some(file));
    };

    // "Open the owning panel" jump, shown for every token surface.
    let panel_jump = |tab: &'static str| {
        rsx! {
            button {
                class: "editor-mini-button",
                title: "Open the full {tab} panel",
                onclick: move |_| layout.focus_palette_panel(tab),
                "{tab} panel…"
            }
        }
    };

    rsx! {
        div {
            class: "preview-toolbar-group",
            style: "margin: 0;",
            GroupCaption { text: "Selected" }
            span {
                class: "mor-selection-chip-inline",
                title: "{sel.label}",
                "{sel.label}"
            }
        }
        match (sel.context, sel.palette_tab) {
            (EditContext::TokenSurface, Some("Colors")) => rsx! {
                div {
                    class: "preview-toolbar-group",
                    style: "margin: 0;",
                    RibbonSwatch { label: "Accent", value: signals.accent }
                    RibbonSwatch { label: "Background", value: signals.bg_base }
                    RibbonSwatch { label: "Text", value: signals.fg_base }
                    RibbonSwatch { label: "Muted", value: signals.fg_muted }
                    {panel_jump("Colors")}
                }
            },
            (EditContext::TokenSurface, Some("Buttons")) => rsx! {
                div {
                    class: "preview-toolbar-group",
                    style: "margin: 0;",
                    select {
                        class: "editor-input",
                        title: "Button shape",
                        value: "{signals.buttons.read().radius}",
                        onchange: move |e| { signals.buttons.clone().write().radius = e.value(); },
                        option { value: "0px", "Square" }
                        option { value: "6px", "Rounded" }
                        option { value: "99px", "Pill" }
                    }
                    RibbonSwatch { label: "Accent", value: signals.accent }
                    {panel_jump("Buttons")}
                }
            },
            (EditContext::TokenSurface, Some("Typography")) => rsx! {
                div {
                    class: "preview-toolbar-group",
                    style: "margin: 0;",
                    label {
                        class: "preview-width-control",
                        span { class: "preview-width-label", "Base font" }
                        input {
                            class: "preview-width-input",
                            r#type: "text",
                            style: "width: 52px;",
                            value: "{signals.base_size}",
                            oninput: move |e| signals.base_size.clone().set(e.value()),
                        }
                    }
                    RibbonSwatch { label: "Text", value: signals.fg_base }
                    {panel_jump("Typography")}
                }
            },
            (EditContext::TokenSurface, Some(tab)) => rsx! {
                div { class: "preview-toolbar-group", style: "margin: 0;", {panel_jump(tab)} }
            },
            (EditContext::Icon, _) => rsx! {
                div {
                    class: "preview-toolbar-group",
                    style: "margin: 0;",
                    if let Some(binding) = sel.binding.clone() {
                        button {
                            class: "editor-mini-button",
                            title: "Pick a replacement icon for this slot",
                            onclick: move |_| layout.active_icon_picker.set(Some(binding.clone())),
                            "Change icon…"
                        }
                    }
                }
            },
            (EditContext::Component, _) => rsx! {
                div {
                    class: "preview-toolbar-group",
                    style: "margin: 0;",
                    if let Some(link) = sel.component.clone() {
                        if let Some(css) = link.css.clone() {
                            button {
                                class: "editor-mini-button",
                                title: "Open {css} in the CSS editor",
                                onclick: move |_| open_asset("css_editor", css.clone()),
                                "CSS"
                            }
                        }
                        if let Some(js) = link.js.clone() {
                            button {
                                class: "editor-mini-button",
                                title: "Open {js} in the JS editor",
                                onclick: move |_| open_asset("js_editor", js.clone()),
                                "Script"
                            }
                        }
                        if let Some(php) = link.php.clone() {
                            // No PHP editor dock exists — name the part so
                            // the three-file unit is still visible.
                            span {
                                class: "editor-mini-button editor-mini-button-disabled",
                                title: "Server part: {php} — edit in your own editor",
                                "PHP"
                            }
                        }
                        if link.css.is_none() && link.js.is_none() && link.php.is_none() {
                            span {
                                style: "font-size: 0.7rem; color: var(--fg-muted);",
                                "no part files named {link.tag}.*"
                            }
                        }
                    }
                }
            },
            // SiteField/Widget: the canvas chip identifies them and the
            // code-reveal path already fires on select. CodeOnly: no action.
            _ => rsx! {},
        }
    }
}
