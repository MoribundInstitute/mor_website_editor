//! Center workspace for the Website plug: live preview of the opened folder,
//! code editor, split view, Editor Canvas (Browse | Inspect | Edit), and the
//! mor-theme.css export surface.

use crate::app::edit_context::{
    analyze_bound, analyze_dom_facts, link_component, DomSelectFacts, EditContext, SelectionInfo,
};
use crate::app::shell::WorkbenchEditState;
use crate::app::state::{
    CenterView, ContextMenuPayload, DockPosition, LayoutState, RenderState, WebsiteState,
};
use crate::ui::workspace::layout::PreviewViewport;
use crate::ui::workspace::page_map::PageMapWorkspace;
use crate::ui::workspace::preview_canvas::PreviewCanvas;
use crate::ui::components::icons::{tool_paths, ToolIcon};
use crate::ui::workspace::preview_ribbon::PreviewRibbon;
use dioxus::prelude::*;
use mor_website_core::config::ThemeConfig;
use mor_website_core::diagnostics::DiagnosticResult;
use mor_website_core::utils::svg_icons::{is_svg, svg_to_data_uri};
use crate::ui::dialogs::modal::Modal;

use crate::ui::layout::docks::smart_code_dock::SmartCodeDock;
use crate::ui::layout::main_pane::MainPane;

/// Drag handler for the fullscreen control bar's title strip. Pointer capture
/// keeps move events flowing even when the cursor crosses the preview iframe
/// (which otherwise swallows them — the source of "sticky" drags), and the
/// body class blanks iframe pointer-events + sets the grabbing cursor.
/// Position is not persisted — re-entering fullscreen recenters the bar.
const FS_BAR_DRAG_JS: &str = r#"
(function () {
    if (window.__morFsBarDragInstalled) return;
    window.__morFsBarDragInstalled = true;

    document.addEventListener('pointerdown', function (e) {
        const handle = e.target.closest('.mor-fullscreen-bar-handle');
        if (!handle) return;
        const bar = handle.closest('.mor-fullscreen-bar');
        if (!bar) return;
        e.preventDefault();
        // Pointer capture is unreliable in this WebKitGTK webview — document
        // listeners + the iframe pointer-events block do the job instead.
        document.body.classList.add('editor-floating-dragging');

        // Un-anchor from the centered default (left:50% + translate) so
        // explicit left/top from here on tracks the pointer.
        const r = bar.getBoundingClientRect();
        const parent = bar.offsetParent ? bar.offsetParent.getBoundingClientRect() : { left: 0, top: 0 };
        const startLeft = r.left - parent.left;
        const startTop = r.top - parent.top;
        bar.style.left = startLeft + 'px';
        bar.style.top = startTop + 'px';
        bar.style.bottom = 'auto';
        bar.style.transform = 'none';

        const sx = e.clientX, sy = e.clientY;
        const onMove = function (m) {
            // Clamp so the bar can't be lost outside the workspace.
            const pw = bar.offsetParent ? bar.offsetParent.clientWidth : window.innerWidth;
            const ph = bar.offsetParent ? bar.offsetParent.clientHeight : window.innerHeight;
            bar.style.left = Math.max(0, Math.min(startLeft + m.clientX - sx, pw - r.width)) + 'px';
            bar.style.top = Math.max(0, Math.min(startTop + m.clientY - sy, ph - r.height)) + 'px';
        };
        const onUp = function () {
            document.removeEventListener('pointermove', onMove);
            document.removeEventListener('pointerup', onUp);
            document.body.classList.remove('editor-floating-dragging');
        };
        document.addEventListener('pointermove', onMove);
        document.addEventListener('pointerup', onUp);
    });
})();
"#;

const PICKER_ICONS: [(&str, &str); 15] = [
    ("Close", "M18 6 6 18M6 6l12 12"),
    ("Search", "M11 18a7 7 0 100-14 7 7 0 000 14zM20 20l-3.5-3.5"),
    ("Menu", "M4 7h16M4 12h16M4 17h16"),
    ("Left Sidebar", "M9 4v16M6 8h.01M6 12h.01 M3 4h18v16H3z"),
    ("Right Sidebar", "M15 4v16M18 8h.01M18 12h.01 M3 4h18v16H3z"),
    ("Chevron Left", "m15 18-6-6 6-6"),
    ("Chevron Right", "m9 18 6-6-6-6"),
    ("Home", "m3 11 9-8 9 8 M5 10v10h14V10 M9 20v-6h6v6"),
    ("Archive", "M5 8v12h14V8 M10 12h4 M3 4h18v4H3z"),
    (
        "Label",
        "M20 13 12 21 3 12V3h9l8 8z M7.5 7.5A1.5 1.5 0 107.5 4a1.5 1.5 0 000 3.5z",
    ),
    (
        "Share",
        "M4 12v8a2 2 0 002 2h12a2 2 0 002-2v-8 M16 6l-4-4-4 4 M12 2v13",
    ),
    (
        "User",
        "M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2 M12 3a4 4 0 1 0 0 8 4 4 0 0 0 0-8",
    ),
    (
        "Comment",
        "M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z",
    ),
    ("Arrow Up", "M12 19V5 M5 12l7-7 7 7"),
    (
        "External Link",
        "M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6 M15 3h6v6 M10 14L21 3",
    ),
];

fn encode_path_to_mask(path_d: &str) -> String {
    let raw = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2.2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"{}\"/></svg>", path_d);
    svg_to_data_uri(&raw)
}

#[component]
pub fn WebsiteWorkspace(
    preview_viewport: Signal<PreviewViewport>,
    preview_width: Signal<u32>,

    preview_html: Signal<String>,
    show_preview: Signal<bool>,
    mut center_view: Signal<CenterView>,
    diag: Signal<DiagnosticResult>,

    config_toml: ReadSignal<String>,
    active_preset: Signal<Option<&'static str>>,
    on_load_theme: EventHandler<String>,
    on_restore: EventHandler<ThemeConfig>,
    #[props(default)] on_navigate: Option<EventHandler<String>>,
    #[props(default)] on_toggle_dark_mode: Option<EventHandler<()>>,
) -> Element {
    let _ = show_preview;
    let mut layout = use_context::<LayoutState>();
    let render = use_context::<RenderState>();
    let mut edit_state = use_context::<WorkbenchEditState>();
    let is_valid = render.diag.read().is_valid;
    let error_count = render.diag.read().errors.len();
    let mut active_icon_picker = layout.active_icon_picker;
    // Default Edit: select + mutate; heavy X-Ray region paints are gone so the
    // iframe shows the real site. View mode (ribbon) turns selection off.
    let is_xray_active = use_signal(|| true);
    let is_edit_active = use_signal(|| true);
    let preset_label = active_preset()
        .and_then(|id| {
            mor_website_core::presets::all_presets()
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.name.to_string())
        })
        .unwrap_or_else(|| "custom".into());
    let mut active_xray_target = use_signal(|| None::<String>);
    let site = use_context::<WebsiteState>();
    // One-shot: open Insert on first Edit session so blocks aren't hidden.
    // Does not re-open if the user closes the dock later.
    let mut insert_surfaced = use_signal(|| false);
    use_effect(move || {
        if is_edit_active() && !insert_surfaced() {
            insert_surfaced.set(true);
            if *layout.insert_dock_pos.peek() == DockPosition::Hidden {
                layout.request_dock("insert", DockPosition::mor_panel_right);
            }
        }
    });
    // Shared with Inspector dock (layout.active_canvas_selection).
    let active_selection = layout.active_canvas_selection;

    let mut select_bound = move |target: String| {
        let sel = analyze_bound(&target);
        if let Some(tab) = sel.palette_tab {
            layout.focus_palette_panel(tab);
        }
        layout.set_canvas_selection(Some(sel));
        // Existing path: reveal the config section when a code editor is showing.
        active_xray_target.set(Some(target));
    };

    let mut select_unbound = move |facts: DomSelectFacts| {
        let mut sel = analyze_dom_facts(&facts);
        // A custom element is a three-part unit: join it with the project's
        // like-named PHP/CSS/JS files so the ribbon can link them.
        if sel.context == EditContext::Component {
            let project = site.project.read();
            sel.component = Some(link_component(
                &facts.tag,
                &project.pages,
                &project.css_files,
                &project.js_files,
            ));
        }
        // Token surfaces still jump to Theme Palette; instance/nav stay page-local.
        if sel.context == EditContext::TokenSurface {
            if let Some(tab) = sel.palette_tab {
                layout.focus_palette_panel(tab);
            }
        }
        layout.set_canvas_selection(Some(sel));
    };

    let mut is_fullscreen = use_signal(|| false);

    let apply_text_edit = {
        let restore = on_restore.clone();
        move |target: String, val: String, cfg: String| {
            if let Some(config) =
                crate::app::services::workspace_service::handle_text_edit(&target, val, &cfg)
            {
                restore.call(config);
            }
        }
    };

    // Webflow-style unbound page text: unique-match replace in the open page file.
    // Errors land on the status bar (WorkbenchEditState) — silent log-only was the bug.
    let apply_page_text = {
        let site = site;
        let mut edit_state = edit_state;
        move |(old_t, new_t): (String, String)| {
            let project = site.project.peek().clone();
            let page = site
                .current_page
                .peek()
                .clone()
                .or_else(|| project.default_page().map(str::to_string));
            let Some(page) = page else {
                log::warn!("Page text edit: no page selected");
                edit_state
                    .workbench_status
                    .set("Page edit: no page selected".into());
                return;
            };
            match crate::app::services::workspace_service::handle_page_text_edit(
                &project, &page, &old_t, &new_t,
            ) {
                Ok(true) => {
                    site.bump_preview();
                    edit_state
                        .workbench_status
                        .set(format!("Saved edit → {page}"));
                }
                Ok(false) => {}
                Err(e) => {
                    log::warn!("Page text edit: {e}");
                    edit_state.workbench_status.set(e);
                }
            }
        }
    };

    let apply_widget_move = {
        let restore = on_restore.clone();
        move |id: String, dest: String, cfg: String| {
            if let Some(config) =
                crate::app::services::workspace_service::handle_widget_move(&id, &dest, &cfg)
            {
                restore.call(config);
            }
        }
    };

    let apply_drop_svg = {
        let restore = on_restore.clone();
        move |(target, content): (String, String), cfg: String| {
            if let Some(config) =
                crate::app::services::workspace_service::handle_drop_svg(&target, &content, &cfg)
            {
                restore.call(config);
            }
        }
    };

    // Preview/Split share the same viewport + page-picker toolbar.
    let in_preview = matches!(center_view(), CenterView::Preview | CenterView::Split);

    rsx! {
        script { dangerous_inner_html: "window.addEventListener('dragover', function(e) {{ e.preventDefault(); }}, false); window.addEventListener('drop', function(e) {{ e.preventDefault(); }}, false);" }
        MainPane {
            hide_header: is_fullscreen(),

            tabs: rsx! {
                if !is_fullscreen() {
                    button {
                        class: "editor-mini-button mor-tool-btn",
                        title: "Collapse the workspace header",
                        onclick: move |_| is_fullscreen.set(true),
                        ToolIcon { d: tool_paths::COLLAPSE_UP }
                    }
                    div { style: "width: 1px; height: 16px; background: var(--editor-border-soft); margin: 0 4px;" }
                    WorkspaceTabs { center_view }
                }
            },

            toolbar: if in_preview && !is_fullscreen() {
                Some(rsx! {
                    PagePickerBar {}
                    PreviewRibbon {
                        preview_viewport,
                        preview_width,
                        is_xray_active,
                        is_edit_active,
                        active_selection,
                    }
                })
            } else {
                None
            },

            if is_fullscreen() {
                div {
                    class: "mor-fullscreen-bar",
                    onmounted: move |_| {
                        // script-tag injection doesn't execute in this webview,
                        // and a dropped Eval handle cancels the script — await it.
                        spawn(async move {
                            let _ = dioxus::document::eval(FS_BAR_DRAG_JS).await;
                        });
                    },
                    div {
                        class: "mor-fullscreen-bar-handle",
                        title: "Drag to move — resize from the bottom-right corner",
                    }
                    div {
                        class: "mor-fullscreen-bar-controls",

                        if in_preview {
                            PagePickerBar {}
                            PreviewRibbon {
                                preview_viewport,
                                preview_width,
                                is_xray_active,
                                is_edit_active,
                                active_selection,
                            }
                            div { style: "width: 1px; height: 16px; background: var(--editor-border-soft);" }
                        }

                        WorkspaceTabs { center_view }

                        div { style: "width: 1px; height: 16px; background: var(--editor-border-soft);" }

                        button {
                            class: "editor-mini-button",
                            onclick: move |_| is_fullscreen.set(false),
                            "Exit fullscreen"
                        }
                    }
                }
            }

            if let Some(icon_target) = active_icon_picker() {
                IconPickerModal {
                    target: icon_target.clone(),
                    config_toml: config_toml(),
                    on_close: move |_| active_icon_picker.set(None),
                    on_select_mask: {
                        let toml_str = config_toml.clone();
                        let restore = on_restore.clone();
                        let target = icon_target.clone();
                        move |(mask, recolor): (String, bool)| {
                            let mut config = toml::from_str::<ThemeConfig>(&toml_str()).unwrap_or_default();
                            match target.as_str() {
                                "icons.panel_close" => config.icons.panel_close = mask,
                                "icons.search" => config.icons.search = mask,
                                "icons.menu" => config.icons.menu = mask,
                                "icons.sidebar_left" => config.icons.sidebar_left = mask,
                                "icons.sidebar_right" => config.icons.sidebar_right = mask,
                                // Widget heading glyphs render from custom_icons
                                // (see css_builder --icon-label/archive/toc), so write
                                // there — that's what the preview/export actually read.
                                "icons.label" => { config.icons.custom_icons.insert("label".to_string(), mask); }
                                "icons.archive" => { config.icons.custom_icons.insert("archive".to_string(), mask); }
                                "icons.toc" => { config.icons.custom_icons.insert("toc".to_string(), mask); }
                                "icons.share" => config.icons.share = mask,
                                "icons.user" => config.icons.user = mask,
                                "icons.comment" => config.icons.comment = mask,
                                "icons.arrow_up" => config.icons.arrow_up = mask,
                                "icons.external_link" => config.icons.external_link = mask,
                                _ => {}
                            }
                            // Record per-slot render mode (slot key = field name after "icons.").
                            if let Some(slot) = target.strip_prefix("icons.") {
                                config.icons.recolor.insert(slot.to_string(), recolor);
                            }
                            restore.call(config);
                        }
                    },
                }
            }

            match center_view() {
                CenterView::CodeEditor => rsx! {
                    SmartCodeDock {
                        config_toml,
                        on_load_theme: on_load_theme.clone(),
                        active_xray_target,
                    }
                },
                CenterView::Split => rsx! {
                    div { style: "display:flex; flex-direction: row; height: 100%; width: 100%;",
                        div { style: "flex: 1; border-right: 1px solid var(--editor-border-soft);",
                            PreviewCanvas {
                                preview_viewport,
                                preview_width,
                                xray_active: Some(is_xray_active),
                                edit_active: Some(is_edit_active),
                                active_selection: Some(active_selection),
                                preview_html: preview_html(),
                                preset_label: Some(preset_label.clone()),
                                on_navigate: move |href: String| { if let Some(handler) = on_navigate.as_ref() { handler.call(href); } },
                                on_select: move |target: String| select_bound(target),
                                on_select_dom: move |facts: crate::app::edit_context::DomSelectFacts| select_unbound(facts),
                                on_icon_edit: move |target: String| { active_icon_picker.set(Some(target)); },
                                on_icon_context_menu: move |payload: ContextMenuPayload| { layout.active_context_menu.set(Some(payload)); },
                                on_toggle_dark_mode: move |_| { if let Some(handler) = on_toggle_dark_mode.as_ref() { handler.call(()); } },
                                on_update_value: {
                                    let mutator = apply_text_edit.clone();
                                    move |(target, val): (String, String)| { mutator(target, val, config_toml()); }
                                },
                                on_page_text_edit: {
                                    let mut mutator = apply_page_text.clone();
                                    move |pair: (String, String)| { mutator(pair); }
                                },
                                on_status: move |msg: String| { edit_state.workbench_status.set(msg); },
                                on_move_widget: {
                                    let mutator = apply_widget_move.clone();
                                    move |(id, dest): (String, String)| { mutator(id, dest, config_toml()); }
                                },
                                on_drop_svg: {
                                    let mutator = apply_drop_svg.clone();
                                    move |(target, content): (String, String)| { mutator((target, content), config_toml()); }
                                }
                            }
                        }
                        div { style: "flex: 1;",
                            SmartCodeDock {
                                config_toml,
                                on_load_theme: on_load_theme.clone(),
                                active_xray_target,
                            }
                        }
                    }
                },
                CenterView::Export => rsx! {
                    ExportResultView {
                        is_valid,
                        error_count,
                    }
                },
                CenterView::PageMap => rsx! {
                    PageMapWorkspace {}
                },
                // Preview, plus the retired Advanced workbench views (still enum
                // variants) all land on the live preview.
                _ => rsx! {
                    PreviewCanvas {
                        preview_viewport,
                        preview_width,
                        xray_active: Some(is_xray_active),
                        edit_active: Some(is_edit_active),
                        active_selection: Some(active_selection),
                        preview_html: preview_html(),
                        preset_label: Some(preset_label.clone()),
                        on_navigate: move |href: String| { if let Some(handler) = on_navigate.as_ref() { handler.call(href); } },
                        on_select: move |target: String| select_bound(target),
                        on_select_dom: move |facts: DomSelectFacts| select_unbound(facts),
                        on_icon_edit: move |target: String| { active_icon_picker.set(Some(target)); },
                        on_icon_context_menu: move |payload: ContextMenuPayload| { layout.active_context_menu.set(Some(payload)); },
                        on_toggle_dark_mode: move |_| { if let Some(handler) = on_toggle_dark_mode.as_ref() { handler.call(()); } },
                        on_update_value: {
                            let mutator = apply_text_edit.clone();
                            move |(target, val): (String, String)| { mutator(target, val, config_toml()); }
                        },
                        on_page_text_edit: {
                            let mut mutator = apply_page_text.clone();
                            move |pair: (String, String)| { mutator(pair); }
                        },
                        on_status: move |msg: String| { edit_state.workbench_status.set(msg); },
                        on_move_widget: {
                            let mutator = apply_widget_move.clone();
                            move |(id, dest): (String, String)| { mutator(id, dest, config_toml()); }
                        },
                        on_drop_svg: {
                            let mutator = apply_drop_svg.clone();
                            move |(target, content): (String, String)| { mutator((target, content), config_toml()); }
                        }
                    }
                },
            }
        }
    }
}

/// Page picker + refresh for the opened website: a `<select>` over the
/// project's pages driving `current_page`, and a ↻ forcing a refetch.
#[component]
fn PagePickerBar() -> Element {
    let site = use_context::<WebsiteState>();
    let project = (site.project)();
    if !project.is_open() {
        return rsx! {};
    }
    let mut current_page = site.current_page;
    let selected = current_page();

    rsx! {
        div {
            class: "preview-toolbar-group",
            style: "margin: 0;",
            select {
                class: "editor-input",
                style: "max-width: 220px;",
                title: "Page shown in the preview",
                onchange: move |evt| {
                    current_page.set(Some(evt.value()));
                    site.bump_preview();
                },
                for page in project.pages.iter() {
                    option {
                        value: "{page}",
                        selected: selected.as_deref() == Some(page.as_str()),
                        "{page}"
                    }
                }
            }
            button {
                class: "editor-mini-button mor-tool-btn",
                title: "Hard refresh preview (Ctrl+Shift+R) — refetch from disk, clear cache, reload assets",
                onclick: move |_| site.hard_refresh_preview(),
                ToolIcon { d: tool_paths::REFRESH }
            }
        }
    }
}

#[component]
fn WorkspaceTabs(center_view: Signal<CenterView>) -> Element {
    // Route every workspace switch through enter_workspace so the per-workspace
    // default dock layout is applied on click, not via a use_effect.
    let mut layout = use_context::<LayoutState>();
    let tab = |active: bool| {
        if active {
            "editor-mini-button mor-view-tab is-active"
        } else {
            "editor-mini-button mor-view-tab"
        }
    };
    rsx! {
        button {
            class: tab(center_view() == CenterView::Preview),
            title: "Preview the site",
            onclick: move |_| layout.enter_workspace(CenterView::Preview),
            ToolIcon { d: tool_paths::PREVIEW_EYE }
            span { "Preview" }
        }
        button {
            class: tab(center_view() == CenterView::CodeEditor),
            title: "Edit page PHP/HTML and related CSS/JS",
            onclick: move |_| layout.enter_workspace(CenterView::CodeEditor),
            ToolIcon { d: tool_paths::CODE }
            span { "Code" }
        }
        button {
            class: tab(center_view() == CenterView::Split),
            title: "Preview and code side by side",
            onclick: move |_| layout.enter_workspace(CenterView::Split),
            ToolIcon { d: tool_paths::SPLIT }
            span { "Split" }
        }
        button {
            class: tab(center_view() == CenterView::PageMap),
            title: "Mindmap of CSS, scripts, and PHP includes for a page",
            onclick: move |_| layout.enter_workspace(CenterView::PageMap),
            ToolIcon { d: tool_paths::PAGE_MAP }
            span { "Map" }
        }
        // Export is the terminal pipeline step, so it sits at the far right.
        button {
            class: tab(center_view() == CenterView::Export),
            title: "Export the finished site",
            onclick: move |_| layout.enter_workspace(CenterView::Export),
            ToolIcon { d: tool_paths::EXPORT }
            span { "Export" }
        }
    }
}

#[component]
fn IconPickerModal(
    target: String,
    config_toml: String,
    on_close: EventHandler<()>,
    on_select_mask: EventHandler<(String, bool)>,
) -> Element {
    let mut status_msg = use_signal(String::new);
    let mut raw_svg_input = use_signal(String::new);
    let mut emoji_input = use_signal(String::new);
    // Default ON: most icons are monochrome and want to follow the theme. Uncheck
    // to keep an icon's own colours (colour emoji, multicolour SVG, a logo).
    let recolor = use_signal(|| true);
    // Standard floating dialog (opaque .mor-modal, draggable, resizable). The
    // parent unmounts us on close, so this local `open` just satisfies Modal.
    let open_signal = use_signal(|| true);

    rsx! {
        Modal {
            open: open_signal,
            title: "Select Visual Icon",
            style: "width: 460px;".to_string(),
            on_close: move |_| on_close.call(()),

                div { style: "font-size: 0.85em; color: var(--fg-muted); margin-bottom: 12px;", "Target slot: ", code { style: "color: var(--accent);", "{target}" } }
                label { style: "display: flex; align-items: center; gap: 8px; font-size: 0.85em; color: var(--fg-base); margin-bottom: 16px; cursor: pointer;",
                    input {
                        r#type: "checkbox",
                        checked: recolor(),
                        onchange: {
                            let mut recolor = recolor;
                            move |evt: Event<FormData>| recolor.set(evt.checked())
                        }
                    }
                    "Recolor to match theme"
                    span { style: "color: var(--fg-muted); font-weight: normal;", " — off keeps the icon's own colours (colour emoji, multicolour SVG, logos)" }
                }
                if !status_msg().is_empty() { div { class: "export-status", "{status_msg}" } }

                h4 { style: "margin: 0 0 10px 0; font-size: 0.85em; color: var(--fg-base); text-transform: uppercase; letter-spacing: 0.05em;", "Paste Raw SVG" }
                div { style: "display: flex; flex-direction: column; gap: 8px; margin-bottom: 24px;",
                    textarea {
                        class: "editor-textarea",
                        style: "width: 100%; box-sizing: border-box; min-height: 80px; resize: vertical; font-family: monospace; font-size: 11px;",
                        placeholder: "Paste raw <svg> code here...",
                        value: "{raw_svg_input}",
                        oninput: move |evt| raw_svg_input.set(evt.value()),
                    }
                    button {
                        class: "editor-button",
                        style: "justify-content: center;",
                        onclick: move |_| {
                            let raw_svg = raw_svg_input().trim().to_string();
                            if raw_svg.is_empty() || !is_svg(&raw_svg) {
                                status_msg.set("Error: Invalid or empty SVG.".to_string());
                                return;
                            }
                            let mask = svg_to_data_uri(&raw_svg);
                            on_select_mask.call((mask, recolor()));
                            status_msg.set("SVG applied!".to_string());
                            raw_svg_input.set(String::new());
                        },
                        "Apply Pasted SVG"
                    }
                    button {
                        class: "editor-button",
                        style: "justify-content: center;",
                        onclick: move |_| {
                            let apply = on_select_mask.clone();
                            let recolor_on = recolor();
                            spawn(async move {
                                if let Some(file) = rfd::AsyncFileDialog::new().add_filter("SVG", &["svg"]).pick_file().await {
                                    let bytes = file.read().await;
                                    let raw_svg = String::from_utf8_lossy(&bytes).into_owned();
                                    if !is_svg(&raw_svg) {
                                        status_msg.set("Error: File is not a valid SVG.".to_string());
                                        return;
                                    }
                                    let mask = svg_to_data_uri(&raw_svg);
                                    apply.call((mask, recolor_on));
                                    status_msg.set(format!("SVG applied from {}", file.file_name()));
                                }
                            });
                        },
                        "Browse OS for .svg..."
                    }
                }

                h4 { style: "margin: 0 0 10px 0; font-size: 0.85em; color: var(--fg-base); text-transform: uppercase; letter-spacing: 0.05em;", "Emoji / Symbol" }
                div { style: "display: flex; gap: 8px; margin-bottom: 6px;",
                    input {
                        class: "editor-input",
                        style: "flex: 1; text-align: center; font-size: 18px;",
                        placeholder: "🔔  ★  ⚙  →",
                        value: "{emoji_input}",
                        oninput: move |evt| emoji_input.set(evt.value()),
                    }
                    button {
                        class: "editor-button",
                        style: "justify-content: center; white-space: nowrap;",
                        onclick: move |_| {
                            let raw = emoji_input().trim().to_string();
                            if raw.is_empty() {
                                status_msg.set("Error: enter an emoji or symbol.".to_string());
                                return;
                            }
                            // Wrap as an SVG <text> glyph so it rides the existing icon
                            // pipeline unchanged. Recolor ON → masked silhouette in the theme
                            // colour; recolor OFF → the SVG renders as a background image, i.e.
                            // the full-colour emoji.
                            let safe = raw.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
                            let svg = format!(
                                "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'><text x='12' y='12' font-size='20' text-anchor='middle' dominant-baseline='central'>{safe}</text></svg>"
                            );
                            on_select_mask.call((svg_to_data_uri(&svg), recolor()));
                            status_msg.set(if recolor() { "Emoji applied (theme colour).".to_string() } else { "Emoji applied (full colour).".to_string() });
                            emoji_input.set(String::new());
                        },
                        "Apply Emoji"
                    }
                }
                p { style: "margin: 0 0 24px 0; font-size: 0.75em; color: var(--fg-muted);", "Any emoji or Unicode symbol. With “Recolor” on it's a theme-tinted silhouette; off keeps the full-colour emoji." }

                h4 { style: "margin: 0 0 10px 0; font-size: 0.85em; color: var(--fg-base); text-transform: uppercase; letter-spacing: 0.05em;", "Loaded in Current Theme" }
                div {
                    style: "display: grid; grid-template-columns: repeat(5, 1fr); gap: 12px; margin-bottom: 24px;",
                    {
                        let parsed = toml::from_str::<ThemeConfig>(&config_toml).unwrap_or_default();
                        let current_icons = [
                            ("Panel Close", parsed.icons.panel_close),
                            ("Search", parsed.icons.search),
                            ("Menu", parsed.icons.menu),
                            ("Left Sidebar", parsed.icons.sidebar_left),
                            ("Right Sidebar", parsed.icons.sidebar_right),
                        ];

                        rsx! {
                            for (label, mask_uri) in current_icons.into_iter() {
                                button {
                                    class: "editor-button", style: "aspect-ratio: 1; padding: 0; display: flex; align-items: center; justify-content: center; background: var(--bg-elevated); border-color: var(--border-color);", title: "{label}",
                                    onclick: {
                                        let mask_uri = mask_uri.clone();
                                        let apply = on_select_mask.clone();
                                        move |_| {
                                            apply.call((mask_uri.clone(), recolor()));
                                            status_msg.set(format!("Applied {} icon.", label));
                                        }
                                    },
                                    span { style: "display: block; width: 24px; height: 24px; background-color: var(--editor-text); -webkit-mask-image: {mask_uri}; -webkit-mask-size: contain; -webkit-mask-repeat: no-repeat; -webkit-mask-position: center;" }
                                }
                            }
                        }
                    }
                }

                h4 { style: "margin: 0 0 10px 0; font-size: 0.85em; color: var(--fg-base); text-transform: uppercase; letter-spacing: 0.05em;", "Default Library" }
                div {
                    style: "display: grid; grid-template-columns: repeat(5, 1fr); gap: 12px;",
                    for (label, path_d) in PICKER_ICONS.iter() {
                        button {
                            class: "editor-button", style: "aspect-ratio: 1; padding: 0; display: flex; align-items: center; justify-content: center; background: var(--bg-elevated); border-color: var(--border-color);", title: "{label}",
                            onclick: {
                                let mask = encode_path_to_mask(path_d);
                                let apply = on_select_mask.clone();
                                move |_| {
                                    apply.call((mask.clone(), recolor()));
                                    status_msg.set(format!("Applied {} icon.", label));
                                }
                            },
                            div { style: "width: 24px; height: 24px; color: var(--fg-base);", dangerous_inner_html: format!("<svg viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='{}'/></svg>", path_d) }
                        }
                    }
                }
        }
    }
}

/// Export surface: site-first walkthrough — write CSS, link pages, bundle.
/// Optional starter-kit HTML is Advanced-only (scaffold a *new* page, not layout).
#[component]
fn ExportResultView(is_valid: bool, error_count: usize) -> Element {
    let render = use_context::<RenderState>();
    let site = use_context::<WebsiteState>();
    let layout = use_context::<LayoutState>();
    let mut step1_status = use_signal(String::new);
    let mut step2_status = use_signal(String::new);
    let mut step3_status = use_signal(String::new);
    let mut step4_status = use_signal(String::new);

    let css = (render.generated_css)();
    let css_kb = css.len() as f64 / 1024.0;
    let config = (render.current_config)();
    let project = (site.project)();
    let project_open = project.is_open();
    let show_starter_kits = layout.advanced_tools_visible();

    let has_js = !mor_website_core::website::html_modules::generate_theme_js(&config).is_empty();
    let selected = mor_website_core::website::html_modules::selected_modules(&config);
    let link_snippet = mor_website_core::website::theme_link_snippet();
    let js_snippet = mor_website_core::website::theme_js_snippet();

    // ponytail: recomputed every render — cheap local reads over the page
    // list, and it keeps the count live after injection / project open.
    let (linked, total) = if project_open {
        mor_website_core::website::count_linked_pages(&project)
    } else {
        (0, 0)
    };

    let step_card = "border: 1px solid var(--editor-border); border-radius: var(--radius-md); padding: 16px; background: var(--bg-panel);";
    let step_title = "margin: 0 0 4px 0; font-size: 1rem; color: var(--fg-base);";
    let step_hint = "margin: 0 0 12px 0; font-size: 0.8rem; color: var(--fg-muted); line-height: 1.5;";
    let step_status = "margin: 10px 0 0 0; font-size: 0.8rem; color: var(--fg-base); white-space: pre-wrap;";
    let no_project_hint = "Open a website folder first (File → Open Website Folder…). Try examples/mor_starter.";

    rsx! {
        div {
            style: "display: flex; flex-direction: column; flex: 1; min-height: 0; gap: 16px; padding: 4px; overflow-y: auto;",

            p {
                style: "margin: 0; font-size: 0.85rem; color: var(--fg-muted); line-height: 1.5;",
                "Your site’s structure stays in PHP/HTML on disk. "
                strong { "File → Save Theme to Site" }
                " writes workspace.toml + mor-theme.css (and optional JS) from the Theme Palette."
            }

            if !is_valid {
                div { class: "export-error-banner",
                    span { style: "flex-shrink: 0;", "⚠" }
                    span { "Site config has {error_count} integrity error(s) — see Diagnostics. The CSS below may be incomplete." }
                }
            }

            // ── Step 1 · Write theme files ──────────────────────────────
            div { class: "editor-panel", style: step_card,
                h3 { style: step_title, "1 · Write mor-theme.css into your site" }
                p { style: step_hint,
                    "Compiles the live token config into a standalone stylesheet in the project root."
                    if has_js { " Optional kit JS ships as mor-theme.js when a starter kit needs it." }
                }
                button {
                    class: if project_open { "editor-button editor-button-good" } else { "editor-button editor-button-disabled" },
                    title: if project_open { "" } else { no_project_hint },
                    onclick: move |_| {
                        let project = (site.project)();
                        if !project.is_open() {
                            step1_status.set(format!("✗ {no_project_hint}."));
                            return;
                        }
                        let config = (render.current_config)();
                        match mor_website_core::website::export_theme_bundle(&project, &config) {
                            Ok(written) => {
                                let css_kb = (render.generated_css)().len() as f64 / 1024.0;
                                let js_note = if written.len() > 1 { " + mor-theme.js" } else { "" };
                                step1_status.set(format!("✓ wrote mor-theme.css ({css_kb:.1} KB){js_note}"));
                                site.bump_preview();
                            }
                            Err(e) => step1_status.set(format!("✗ {e}")),
                        }
                    },
                    if has_js { "Write mor-theme.css + mor-theme.js" } else { "Write mor-theme.css" }
                }
                if !project_open {
                    p { style: "margin: 10px 0 0 0; font-size: 0.8rem; color: var(--d29922, #d29922);", "{no_project_hint}." }
                }
                if !step1_status().is_empty() {
                    p { style: step_status, "{step1_status}" }
                }
            }

            // ── Step 2 · Link the theme from every page ─────────────────
            div { class: "editor-panel", style: step_card,
                h3 { style: step_title, "2 · Link them from your pages" }
                if project_open {
                    p { style: step_hint, "{linked} of {total} pages link the theme" }
                    if linked < total {
                        button {
                            class: "editor-button editor-button-good",
                            style: "margin-bottom: 12px;",
                            onclick: move |_| {
                                let project = (site.project)();
                                let config = (render.current_config)();
                                match mor_website_core::website::inject_theme_links(&project, &config) {
                                    Ok((modified, skipped)) => {
                                        let mut msg = format!("✓ updated {} page(s)", modified.len());
                                        if !skipped.is_empty() {
                                            msg.push_str(&format!("\nno </head> found: {}", skipped.join(", ")));
                                        }
                                        step2_status.set(msg);
                                        site.bump_preview();
                                    }
                                    Err(e) => step2_status.set(format!("✗ {e}")),
                                }
                            },
                            "Add link tags to all pages"
                        }
                    }
                } else {
                    p { style: step_hint, "{no_project_hint} to see link status. The snippets below work for manual use either way." }
                }
                div { style: "display: flex; flex-direction: column; gap: 8px;",
                    SnippetRow { snippet: link_snippet, status: step2_status }
                    if has_js {
                        SnippetRow { snippet: js_snippet, status: step2_status }
                    }
                }
                if !step2_status().is_empty() {
                    p { style: step_status, "{step2_status}" }
                }
            }

            // ── Optional starter kits (Advanced only) ───────────────────
            if show_starter_kits {
                div { class: "editor-panel", style: step_card,
                    h3 { style: step_title, "Optional · Starter kits (new pages only)" }
                    p { style: step_hint,
                        "Scaffold HTML for a greenfield page — does not replace your existing includes. "
                        "Pick kits under Theme Palette → Starter kits. Structure for live sites stays in your PHP/HTML files."
                    }
                    if selected.is_empty() {
                        p { style: step_hint, "No kits selected — open Theme Palette → Starter kits (Advanced mode)." }
                    }
                    div { style: "display: flex; flex-direction: column; gap: 6px; margin-bottom: 12px;",
                        for m in selected {
                            div { style: "display: flex; align-items: center; gap: 10px;",
                                span { style: "flex: 1; font-size: 0.85rem; color: var(--fg-base);", "{m.name}" }
                                button {
                                    class: "editor-mini-button",
                                    onclick: move |_| {
                                        crate::utils::clipboard::copy_to_clipboard(m.html.to_string());
                                        step3_status.set(format!("✓ {} HTML copied.", m.name));
                                    },
                                    "Copy HTML"
                                }
                            }
                        }
                    }
                    button {
                        class: if project_open { "editor-button" } else { "editor-button editor-button-disabled" },
                        title: if project_open { "Compose selected kits into mor-starter.html (always overwritten)" } else { no_project_hint },
                        onclick: move |_| {
                            let project = (site.project)();
                            if !project.is_open() {
                                step3_status.set(format!("✗ {no_project_hint}."));
                                return;
                            }
                            let config = (render.current_config)();
                            let title = if config.site.site_title.trim().is_empty() {
                                "My Site".to_string()
                            } else {
                                config.site.site_title.clone()
                            };
                            match mor_website_core::website::export_starter_page(&project, &config, &title) {
                                Ok(path) => {
                                    step3_status.set(format!("✓ wrote {}", path.display()));
                                    if let Ok(rescanned) = mor_website_core::website::scan_project(&project.root) {
                                        let mut project_signal = site.project;
                                        project_signal.set(rescanned);
                                    }
                                    let mut current_page = site.current_page;
                                    current_page.set(Some(mor_website_core::website::STARTER_PAGE_FILENAME.to_string()));
                                    site.bump_preview();
                                }
                                Err(e) => step3_status.set(format!("✗ {e}")),
                            }
                        },
                        "Write starter page (mor-starter.html)"
                    }
                    if !step3_status().is_empty() {
                        p { style: step_status, "{step3_status}" }
                    }
                }
            }

            // ── Step 3 · Bundle ─────────────────────────────────────────
            div { class: "editor-panel", style: step_card,
                h3 { style: step_title, "3 · Bundle" }
                p { style: step_hint, "Ship the whole site as a zip (with a fresh mor-theme.css), or grab the full CSS for elsewhere." }
                div { class: "export-action-group", style: "display: flex; flex-wrap: wrap; gap: 10px;",
                    button {
                        class: if project_open { "editor-button editor-button-good" } else { "editor-button editor-button-disabled" },
                        title: if project_open { "" } else { no_project_hint },
                        onclick: move |_| {
                            let project = (site.project)();
                            if !project.is_open() {
                                step4_status.set(format!("✗ {no_project_hint}."));
                                return;
                            }
                            let config = (render.current_config)();
                            let mut status = step4_status;
                            spawn(async move {
                                let Some(handle) = rfd::AsyncFileDialog::new()
                                    .add_filter("Zip", &["zip"])
                                    .set_file_name("site.zip")
                                    .save_file()
                                    .await
                                else {
                                    return;
                                };
                                let dest = handle.path().to_path_buf();
                                let result = tokio::task::spawn_blocking(move || {
                                    mor_website_core::website::zip_site(&project, &config, &dest)
                                        .map(|_| dest)
                                })
                                .await;
                                match result {
                                    Ok(Ok(dest)) => status.set(format!("✓ exported site → {}", dest.display())),
                                    Ok(Err(e)) => status.set(format!("✗ zip failed: {e}")),
                                    Err(e) => status.set(format!("✗ zip task failed: {e}")),
                                }
                            });
                        },
                        "Export Site Zip…"
                    }
                    button {
                        class: "editor-button",
                        onclick: move |_| {
                            crate::utils::clipboard::copy_to_clipboard((render.generated_css)());
                            step4_status.set("✓ theme CSS copied to clipboard.".to_string());
                        },
                        "Copy full CSS"
                    }
                }
                if !step4_status().is_empty() {
                    p { style: step_status, "{step4_status}" }
                }
            }

            // ── Generated stylesheet (read-only) ────────────────────────
            details {
                class: "editor-panel",
                style: step_card,
                summary { style: "cursor: pointer; font-size: 1rem; color: var(--fg-base); font-weight: 600;",
                    "Generated Stylesheet — mor-theme.css ({css_kb:.1} KB)"
                }
                textarea {
                    class: "editor-textarea",
                    style: "width: 100%; height: 320px; box-sizing: border-box; margin-top: 12px; resize: vertical; font-family: monospace; font-size: 11px; white-space: pre;",
                    readonly: true,
                    value: "{css}",
                }
            }
        }
    }
}

/// One copyable snippet line for the export stepper.
#[component]
fn SnippetRow(snippet: String, status: Signal<String>) -> Element {
    let text = snippet.clone();
    rsx! {
        div { style: "display: flex; align-items: center; gap: 10px;",
            code {
                style: "flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.8rem; background: var(--bg-elevated); padding: 6px 10px; border-radius: 4px;",
                title: "Add this to each page's <head>",
                "{snippet}"
            }
            button {
                class: "editor-button",
                onclick: move |_| {
                    crate::utils::clipboard::copy_to_clipboard(text.clone());
                    let mut status = status;
                    status.set("✓ snippet copied.".to_string());
                },
                "Copy"
            }
        }
    }
}
