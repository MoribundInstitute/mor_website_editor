//! Page Map workspace — mindmap of CSS / JS / PHP includes for one page.
//! Inspired by mor-rust-code-gui-manager's dependency ring layout.

use dioxus::prelude::*;
use mor_website_core::website::page_assets::{
    layout_mindmap, map_page_assets, AssetKind, AssetNode, PageAssetMap,
};

use crate::app::state::{DockPosition, LayoutState, WebsiteState};

const VIEW_W: f64 = 960.0;
const VIEW_H: f64 = 640.0;
const CX: f64 = VIEW_W / 2.0;
const CY: f64 = VIEW_H / 2.0;

#[derive(Clone, PartialEq)]
struct DrawnEdge {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    stroke: &'static str,
}

#[derive(Clone, PartialEq)]
struct DrawnNode {
    node: AssetNode,
    x: f64,
    y: f64,
    fill: &'static str,
    stroke: &'static str,
    r: f64,
    missing: bool,
}

fn open_asset(layout: &mut LayoutState, kind: AssetKind, path: &str) {
    match kind {
        AssetKind::Css => {
            layout.css_editor_open_file.set(Some(path.to_string()));
            layout.css_editor_pos.set(DockPosition::mor_panel_left);
        }
        AssetKind::Js => {
            layout.js_editor_open_file.set(Some(path.to_string()));
            layout.js_editor_pos.set(DockPosition::mor_panel_left);
        }
        _ => {}
    }
}

#[component]
pub fn PageMapWorkspace() -> Element {
    let site = use_context::<WebsiteState>();
    let mut layout = use_context::<LayoutState>();
    let project = (site.project)();
    let mut current_page = site.current_page;
    let nonce = site.preview_nonce;

    let map = use_memo(move || {
        let _ = nonce();
        let project = (site.project)();
        if !project.is_open() {
            return PageAssetMap::default();
        }
        let page = current_page()
            .or_else(|| project.default_page().map(str::to_string))
            .unwrap_or_default();
        if page.is_empty() {
            return PageAssetMap::default();
        }
        map_page_assets(&project.root, &page)
    });

    let drawn = use_memo(move || {
        let m = map();
        let pts = layout_mindmap(&m, CX, CY);
        let edges: Vec<DrawnEdge> = m
            .edges
            .iter()
            .filter_map(|e| {
                let a = pts.get(&e.from)?;
                let b = pts.get(&e.to)?;
                Some(DrawnEdge {
                    x1: a.x,
                    y1: a.y,
                    x2: b.x,
                    y2: b.y,
                    stroke: edge_stroke(&e.via),
                })
            })
            .collect();
        let nodes: Vec<DrawnNode> = m
            .nodes
            .iter()
            .filter_map(|n| {
                let p = pts.get(&n.id)?;
                let (fill, stroke, r) = node_style(n.kind);
                let missing = !n.exists
                    && !matches!(n.kind, AssetKind::ExternalCss | AssetKind::ExternalJs);
                Some(DrawnNode {
                    node: n.clone(),
                    x: p.x,
                    y: p.y,
                    fill,
                    stroke,
                    r,
                    missing,
                })
            })
            .collect();
        (edges, nodes, m)
    });

    if !project.is_open() {
        return rsx! {
            div { class: "page-map empty",
                h2 { "Page Map" }
                p { "Open a website folder to see which CSS and scripts each page loads." }
            }
        };
    }

    let (edges, nodes, m) = drawn();
    let selected = current_page()
        .or_else(|| project.default_page().map(str::to_string))
        .unwrap_or_default();
    let list_nodes: Vec<AssetNode> = m
        .nodes
        .iter()
        .filter(|n| n.kind != AssetKind::Page)
        .cloned()
        .collect();

    rsx! {
        div { class: "page-map",
            div { class: "page-map-toolbar",
                div { class: "page-map-title",
                    span { class: "page-map-kicker", "Page Map" }
                    span { class: "page-map-sub", "Mindmap of includes · stylesheets · scripts" }
                }
                div { class: "page-map-controls",
                    label { class: "page-map-label",
                        "Page"
                        select {
                            class: "editor-input",
                            title: "Page to map",
                            onchange: move |evt| {
                                current_page.set(Some(evt.value()));
                            },
                            for page in project.pages.iter() {
                                option {
                                    value: "{page}",
                                    selected: selected.as_str() == page.as_str(),
                                    "{page}"
                                }
                            }
                        }
                    }
                    button {
                        class: "editor-mini-button",
                        title: "Rescan from disk",
                        onclick: move |_| site.bump_preview(),
                        "Rescan"
                    }
                }
                div { class: "page-map-stats",
                    span { class: "stat include", "{m.include_count()} includes" }
                    span { class: "stat css", "{m.css_count()} CSS" }
                    span { class: "stat js", "{m.js_count()} JS" }
                    span { class: "stat edges", "{m.edges.len()} links" }
                }
            }

            div { class: "page-map-body",
                div { class: "page-map-canvas-wrap",
                    svg {
                        class: "page-map-svg",
                        view_box: "0 0 {VIEW_W} {VIEW_H}",
                        preserve_aspect_ratio: "xMidYMid meet",

                        circle {
                            class: "page-map-ring",
                            cx: "{CX}", cy: "{CY}", r: "160",
                            fill: "none",
                        }
                        circle {
                            class: "page-map-ring outer",
                            cx: "{CX}", cy: "{CY}", r: "300",
                            fill: "none",
                        }

                        for e in edges.iter() {
                            line {
                                class: "page-map-edge",
                                x1: "{e.x1}", y1: "{e.y1}",
                                x2: "{e.x2}", y2: "{e.y2}",
                                stroke: "{e.stroke}",
                                stroke_width: "1.5",
                            }
                        }

                        for dn in nodes.iter() {
                            {
                                let path = dn.node.path.clone();
                                let kind = dn.node.kind;
                                let label = dn.node.label.clone();
                                let tip = format!("{} · {}", dn.node.path, dn.node.kind.label());
                                let ly = dn.r + 14.0;
                                let cls = if dn.missing {
                                    "page-map-node missing"
                                } else {
                                    "page-map-node"
                                };
                                let stroke = if dn.missing { "#e05" } else { dn.stroke };
                                rsx! {
                                    g {
                                        class: "{cls}",
                                        transform: "translate({dn.x}, {dn.y})",
                                        style: "cursor: pointer;",
                                        "data-tip": "{tip}",
                                        onclick: move |_| {
                                            if kind == AssetKind::Page {
                                                current_page.set(Some(path.clone()));
                                            } else {
                                                open_asset(&mut layout, kind, &path);
                                            }
                                        },
                                        // Native tooltip via SVG title child
                                        title { "{tip}" }
                                        circle {
                                            r: "{dn.r}",
                                            fill: "{dn.fill}",
                                            stroke: "{stroke}",
                                            stroke_width: "2",
                                        }
                                        text {
                                            y: "{ly}",
                                            text_anchor: "middle",
                                            class: "page-map-node-label",
                                            "{label}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                aside { class: "page-map-side",
                    h3 { "Legend" }
                    ul { class: "page-map-legend",
                        li { span { class: "swatch page" } "Page (center)" }
                        li { span { class: "swatch include" } "PHP include" }
                        li { span { class: "swatch css" } "Stylesheet" }
                        li { span { class: "swatch js" } "Script" }
                        li { span { class: "swatch ext" } "External URL" }
                        li { span { class: "swatch missing" } "Missing on disk" }
                    }

                    h3 { "Assets" }
                    div { class: "page-map-list",
                        if list_nodes.is_empty() {
                            p { class: "muted", "No assets found. Pick a page or rescan." }
                        }
                        for node in list_nodes.iter() {
                            {
                                let path = node.path.clone();
                                let kind = node.kind;
                                let label = node.label.clone();
                                let kind_l = node.kind.label();
                                let path_disp = node.path.clone();
                                let missing = !node.exists
                                    && !matches!(
                                        node.kind,
                                        AssetKind::ExternalCss | AssetKind::ExternalJs
                                    );
                                let cls = if missing {
                                    "page-map-list-item missing"
                                } else {
                                    "page-map-list-item"
                                };
                                rsx! {
                                    button {
                                        class: "{cls}",
                                        "data-tip": "{path_disp}",
                                        onclick: move |_| open_asset(&mut layout, kind, &path),
                                        span { class: "kind", "{kind_l}" }
                                        span { class: "name", "{label}" }
                                        span { class: "path", "{path_disp}" }
                                    }
                                }
                            }
                        }
                    }

                    p { class: "page-map-hint",
                        "Static scan — follows require/include, "
                        "link/script tags, and $extraCss / $extraScripts. "
                        "Click a CSS or JS node to open it in the editor."
                    }
                }
            }
        }
    }
}

fn node_style(kind: AssetKind) -> (&'static str, &'static str, f64) {
    match kind {
        AssetKind::Page => ("#1a1a1a", "#ffffff", 22.0),
        AssetKind::Include => ("#1a1520", "#c4b5fd", 12.0),
        AssetKind::Css => ("#0f1a14", "#6ee7b7", 10.0),
        AssetKind::Js => ("#1a1408", "#fbbf24", 10.0),
        AssetKind::ExternalCss => ("#0c1218", "#38bdf8", 9.0),
        AssetKind::ExternalJs => ("#18140c", "#fb923c", 9.0),
    }
}

fn edge_stroke(via: &str) -> &'static str {
    match via {
        "stylesheet" | "extraCss" => "#2d5a45",
        "script" | "extraScripts" => "#5a4a20",
        _ => "#3a3a3a",
    }
}
