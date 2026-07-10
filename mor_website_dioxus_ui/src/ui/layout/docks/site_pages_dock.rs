//! Site Pages dock: the opened website folder's inventory. Open a folder,
//! pick the previewed page, and jump into the CSS/JS editors for real files.

use crate::app::state::{DockPosition, LayoutState, WebsiteState};
use crate::ui::components::dock_chrome::DockChrome;
use dioxus::prelude::*;

/// Middle-truncate a path so the root line stays one row.
fn truncate_middle(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let keep = (max.saturating_sub(1)) / 2;
    let head: String = s.chars().take(keep).collect();
    let tail: String = s.chars().rev().take(keep).collect::<Vec<_>>().into_iter().rev().collect();
    format!("{head}…{tail}")
}

#[component]
pub fn SitePagesDock() -> Element {
    let mut layout = use_context::<LayoutState>();
    let site = use_context::<WebsiteState>();
    let vfs = use_context::<crate::app::vfs::VfsDictionary>().0;
    let pos = (layout.site_pages_pos)();

    if pos == DockPosition::Hidden {
        return rsx! {};
    }

    let project = (site.project)();
    let current = (site.current_page)();
    let server = (site.server)();
    let root_label = truncate_middle(&project.root.display().to_string(), 46);
    let server_label = match server {
        Some(info) if info.php => format!("php · 127.0.0.1:{}", info.port),
        Some(info) => format!("static · 127.0.0.1:{}", info.port),
        None => "no server".to_string(),
    };

    rsx! {
        crate::ui_kit::MorPanelWrapper {
            position: pos,
            default_position: DockPosition::mor_panel_right,
            DockChrome {
                title: "Site Pages".to_string(),
                dock_id: "site_pages".to_string(),
                position: pos,
                on_close: move |_| {
                    layout.site_pages_pos.set(DockPosition::Hidden);
                },
                section {
                    class: "site-pages-panel",
                    style: "padding: 12px; height: calc(100% - 45px); overflow-y: auto; background: var(--bg-panel); color: var(--fg-base); font-size: 0.85rem; display: flex; flex-direction: column; gap: 12px;",

                    div { style: "display: flex; flex-direction: column; gap: 8px;",
                        button {
                            class: "editor-button editor-button-good",
                            style: "justify-content: center;",
                            onclick: move |_| {
                                crate::app::shell_file_actions::open_website_folder(site, vfs);
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
                                "No website folder open. Pick a folder of HTML/PHP/CSS/JS files to style it."
                            }
                        }
                    }

                    if project.is_open() {
                        div {
                            h4 { style: "margin: 0 0 6px 0; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--fg-muted);",
                                "Pages ({project.pages.len()})"
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

                        div {
                            h4 { style: "margin: 0 0 6px 0; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--fg-muted);",
                                "Stylesheets ({project.css_files.len()})"
                            }
                            div { style: "display: flex; flex-direction: column; gap: 2px;",
                                for file in project.css_files.iter().cloned() {
                                    button {
                                        key: "{file}",
                                        class: "editor-mini-button",
                                        style: "justify-content: flex-start; text-align: left; width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                                        title: "Open in CSS Editor",
                                        onclick: move |_| {
                                            layout.request_dock("css_editor", DockPosition::mor_panel_left);
                                        },
                                        "🖌️ {file}"
                                    }
                                }
                            }
                        }

                        div {
                            h4 { style: "margin: 0 0 6px 0; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: var(--fg-muted);",
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
                                                let mut open_req = layout.js_editor_open_file;
                                                open_req.set(Some(file.clone()));
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
