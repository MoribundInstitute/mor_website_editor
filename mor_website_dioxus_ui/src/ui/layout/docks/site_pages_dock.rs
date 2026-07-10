//! Page dock — tools for the **current PHP/HTML page** only.
//!
//! Site-wide look lives in Theme Palette. Clicked elements live in Inspector.
//! This dock: pick the page, open Code/Insert/Map, see assets linked from it.

use crate::app::state::{CenterView, DockPosition, LayoutState, WebsiteState};
use crate::ui::components::dock_chrome::DockChrome;
use dioxus::prelude::*;
use mor_website_core::website::page_assets::{map_page_assets, AssetKind, PageAssetMap};

/// Middle-truncate a path so the root line stays one row.
fn truncate_middle(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let keep = (max.saturating_sub(1)) / 2;
    let head: String = s.chars().take(keep).collect();
    let tail: String = s
        .chars()
        .rev()
        .take(keep)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{head}…{tail}")
}

#[component]
pub fn SitePagesDock() -> Element {
    let mut layout = use_context::<LayoutState>();
    let site = use_context::<WebsiteState>();
    let vfs = use_context::<crate::app::vfs::VfsDictionary>().0;
    let theme = use_context::<crate::app::state::ThemeState>();
    let original_toml = use_context::<Signal<String>>();
    let pos = (layout.site_pages_pos)();

    if pos == DockPosition::Hidden {
        return rsx! {};
    }

    let project = (site.project)();
    let current = (site.current_page)();
    let server = (site.server)();
    let nonce = site.preview_nonce;
    let root_label = truncate_middle(&project.root.display().to_string(), 46);
    let server_label = match server {
        Some(info) if info.php => format!("php · 127.0.0.1:{}", info.port),
        Some(info) => format!("static · 127.0.0.1:{}", info.port),
        None => "no server".to_string(),
    };

    let active_page = current
        .clone()
        .or_else(|| project.default_page().map(str::to_string));

    let page_map = use_memo(move || {
        let _ = nonce();
        let project = (site.project)();
        let page = (site.current_page)()
            .or_else(|| project.default_page().map(str::to_string))
            .unwrap_or_default();
        if !project.is_open() || page.is_empty() {
            return PageAssetMap::default();
        }
        map_page_assets(&project.root, &page)
    });

    rsx! {
        crate::ui_kit::MorPanelWrapper {
            position: pos,
            default_position: DockPosition::mor_panel_right,
            DockChrome {
                title: "Page".to_string(),
                dock_id: "site_pages".to_string(),
                position: pos,
                on_close: move |_| {
                    layout.site_pages_pos.set(DockPosition::Hidden);
                },
                section {
                    class: "site-pages-panel",
                    style: "padding: 12px; height: calc(100% - 45px); overflow-y: auto; background: var(--bg-panel); color: var(--fg-base); font-size: 0.85rem; display: flex; flex-direction: column; gap: 14px;",

                    p {
                        style: "margin: 0; font-size: 0.75rem; line-height: 1.45; color: var(--fg-muted);",
                        strong { "This page" }
                        " only — content, includes, and linked CSS/JS. Site-wide look: Theme (Alt+T). Clicked element: Inspector (Alt+X)."
                    }

                    div { style: "display: flex; flex-direction: column; gap: 8px;",
                        button {
                            class: "editor-button editor-button-good",
                            style: "justify-content: center;",
                            onclick: move |_| {
                                crate::app::shell_file_actions::open_website_folder(
                                    site,
                                    vfs,
                                    theme,
                                    original_toml,
                                );
                            },
                            "Open Folder…"
                        }
                        if project.is_open() {
                            div {
                                style: "font-family: monospace; font-size: 0.75rem; color: var(--fg-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                title: "{project.root.display()}",
                                "{root_label}"
                            }
                            div { style: "font-size: 0.72rem; color: var(--fg-muted);", "{server_label}" }
                        } else {
                            p { style: "margin: 0; color: var(--fg-muted); line-height: 1.5;",
                                "No website folder open. Pick a folder of HTML/PHP/CSS/JS files."
                            }
                        }
                    }

                    if project.is_open() {
                        // ── Current page tools ───────────────────────────
                        div {
                            style: "padding: 10px 12px; border-radius: 8px; border: 1px solid var(--editor-border-soft, #333); display: flex; flex-direction: column; gap: 8px;",
                            div {
                                style: "font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.04em; color: var(--fg-muted);",
                                "Editing"
                            }
                            div {
                                style: "font-weight: 700; font-family: var(--editor-mono, monospace); font-size: 0.9rem; word-break: break-all;",
                                if let Some(ref p) = active_page {
                                    "{p}"
                                } else {
                                    "(no page)"
                                }
                            }
                            div { style: "display: flex; flex-wrap: wrap; gap: 6px;",
                                button {
                                    class: "editor-mini-button",
                                    title: "Open Code view for this page",
                                    disabled: active_page.is_none(),
                                    onclick: move |_| {
                                        layout.center_view.set(CenterView::CodeEditor);
                                    },
                                    "Code"
                                }
                                button {
                                    class: "editor-mini-button",
                                    title: "Insert dock (blocks, images) — Alt+I",
                                    onclick: move |_| {
                                        layout.request_dock("insert", DockPosition::mor_panel_right);
                                    },
                                    "Insert"
                                }
                                button {
                                    class: "editor-mini-button",
                                    title: "Inspector — what you clicked — Alt+X",
                                    onclick: move |_| {
                                        layout.request_dock("inspector", DockPosition::mor_panel_right);
                                    },
                                    "Inspector"
                                }
                                button {
                                    class: "editor-mini-button",
                                    title: "Page asset mindmap",
                                    disabled: active_page.is_none(),
                                    onclick: move |_| {
                                        layout.center_view.set(CenterView::PageMap);
                                    },
                                    "Page map"
                                }
                            }
                            p {
                                style: "margin: 0; font-size: 0.72rem; color: var(--fg-muted); line-height: 1.4;",
                                "Edit mode: double-click text for the rich toolbar; drag Insert blocks onto the preview."
                            }
                        }

                        // ── Assets for this page ─────────────────────────
                        {
                            let map = page_map();
                            let includes: Vec<_> = map
                                .nodes
                                .iter()
                                .filter(|n| n.kind == AssetKind::Include)
                                .cloned()
                                .collect();
                            let css: Vec<_> = map
                                .nodes
                                .iter()
                                .filter(|n| n.kind.is_style())
                                .cloned()
                                .collect();
                            let js: Vec<_> = map
                                .nodes
                                .iter()
                                .filter(|n| n.kind.is_script())
                                .cloned()
                                .collect();
                            rsx! {
                                if active_page.is_some() {
                                    div {
                                        h4 { style: "margin: 0 0 6px 0; font-size: 0.72rem; letter-spacing: 0.04em; color: var(--fg-muted); font-weight: 700;",
                                            "Linked from this page"
                                        }
                                        if includes.is_empty() && css.is_empty() && js.is_empty() {
                                            p {
                                                style: "margin: 0; font-size: 0.75rem; color: var(--fg-muted);",
                                                "No includes / stylesheets / scripts detected (static scan)."
                                            }
                                        }
                                        if !includes.is_empty() {
                                            p { style: "margin: 0 0 4px 0; font-size: 0.72rem; color: var(--fg-muted);",
                                                "Includes ({includes.len()})"
                                            }
                                            div { style: "display: flex; flex-direction: column; gap: 2px; margin-bottom: 8px;",
                                                for n in includes {
                                                    span {
                                                        key: "{n.id}",
                                                        style: "font-family: monospace; font-size: 0.75rem; padding: 4px 6px; color: var(--fg-muted);",
                                                        title: "{n.path}",
                                                        "📎 {n.label}"
                                                    }
                                                }
                                            }
                                        }
                                        if !css.is_empty() {
                                            p { style: "margin: 0 0 4px 0; font-size: 0.72rem; color: var(--fg-muted);",
                                                "Styles ({css.len()})"
                                            }
                                            div { style: "display: flex; flex-direction: column; gap: 2px; margin-bottom: 8px;",
                                                for n in css {
                                                    button {
                                                        key: "{n.id}",
                                                        class: "editor-mini-button",
                                                        style: "justify-content: flex-start; text-align: left; width: 100%;",
                                                        title: "{n.path}",
                                                        disabled: n.kind == AssetKind::ExternalCss || !n.exists,
                                                        onclick: {
                                                            let path = n.path.clone();
                                                            let kind = n.kind;
                                                            move |_| {
                                                                if kind == AssetKind::Css {
                                                                    layout.css_editor_open_file.set(Some(path.clone()));
                                                                    layout.request_dock("css_editor", DockPosition::mor_panel_left);
                                                                }
                                                            }
                                                        },
                                                        "🖌️ {n.label}"
                                                    }
                                                }
                                            }
                                        }
                                        if !js.is_empty() {
                                            p { style: "margin: 0 0 4px 0; font-size: 0.72rem; color: var(--fg-muted);",
                                                "Scripts ({js.len()})"
                                            }
                                            div { style: "display: flex; flex-direction: column; gap: 2px;",
                                                for n in js {
                                                    button {
                                                        key: "{n.id}",
                                                        class: "editor-mini-button",
                                                        style: "justify-content: flex-start; text-align: left; width: 100%;",
                                                        title: "{n.path}",
                                                        disabled: n.kind == AssetKind::ExternalJs || !n.exists,
                                                        onclick: {
                                                            let path = n.path.clone();
                                                            let kind = n.kind;
                                                            move |_| {
                                                                if kind == AssetKind::Js {
                                                                    layout.js_editor_open_file.set(Some(path.clone()));
                                                                    layout.request_dock("js_editor", DockPosition::mor_panel_left);
                                                                }
                                                            }
                                                        },
                                                        "🔧 {n.label}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // ── All pages ────────────────────────────────────
                        div {
                            h4 { style: "margin: 0 0 6px 0; font-size: 0.72rem; letter-spacing: 0.04em; color: var(--fg-muted); font-weight: 700;",
                                "All pages ({project.pages.len()})"
                            }
                            div { style: "display: flex; flex-direction: column; gap: 2px;",
                                for page in project.pages.iter().cloned() {
                                    button {
                                        key: "{page}",
                                        class: if current.as_deref() == Some(page.as_str()) { "editor-mini-button editor-mini-button-active" } else { "editor-mini-button" },
                                        style: "justify-content: flex-start; text-align: left; width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                        title: "{page}",
                                        onclick: {
                                            let page = page.clone();
                                            move |_| {
                                                let mut current_page = site.current_page;
                                                current_page.set(Some(page.clone()));
                                                site.bump_preview();
                                            }
                                        },
                                        "{page}"
                                    }
                                }
                            }
                        }

                        // ── Project-wide inventory (collapsed-ish) ───────
                        details {
                            style: "margin: 0;",
                            summary {
                                style: "cursor: pointer; font-size: 0.72rem; letter-spacing: 0.04em; color: var(--fg-muted); font-weight: 700;",
                                "All project CSS / JS"
                            }
                            div { style: "margin-top: 8px; display: flex; flex-direction: column; gap: 10px;",
                                div {
                                    h4 { style: "margin: 0 0 6px 0; font-size: 0.72rem; color: var(--fg-muted);",
                                        "Stylesheets ({project.css_files.len()})"
                                    }
                                    div { style: "display: flex; flex-direction: column; gap: 2px;",
                                        for file in project.css_files.iter().cloned() {
                                            button {
                                                key: "{file}",
                                                class: "editor-mini-button",
                                                style: "justify-content: flex-start; text-align: left; width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                                title: "Open in CSS Editor",
                                                onclick: {
                                                    let file = file.clone();
                                                    move |_| {
                                                        layout.css_editor_open_file.set(Some(file.clone()));
                                                        layout.request_dock("css_editor", DockPosition::mor_panel_left);
                                                    }
                                                },
                                                "🖌️ {file}"
                                            }
                                        }
                                    }
                                }
                                div {
                                    h4 { style: "margin: 0 0 6px 0; font-size: 0.72rem; color: var(--fg-muted);",
                                        "Scripts ({project.js_files.len()})"
                                    }
                                    div { style: "display: flex; flex-direction: column; gap: 2px;",
                                        for file in project.js_files.iter().cloned() {
                                            button {
                                                key: "{file}",
                                                class: "editor-mini-button",
                                                style: "justify-content: flex-start; text-align: left; width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                                title: "Open in JS Editor",
                                                onclick: {
                                                    let file = file.clone();
                                                    move |_| {
                                                        layout.js_editor_open_file.set(Some(file.clone()));
                                                        layout.request_dock("js_editor", DockPosition::mor_panel_left);
                                                    }
                                                },
                                                "🔧 {file}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
