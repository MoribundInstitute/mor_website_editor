//! Center Code editor: when a website folder is open, edit the selected PHP/HTML
//! page source, with a dropdown of related CSS / JS / includes that page uses.
//! Theme TOML + compiled mor-theme.css remain available as secondary buffers.

use crate::app::state::{LayoutState, WebsiteState};
use crate::app::vfs::VfsDictionary;
use crate::ui::components::code_editor::CodeEditor;
use crate::ui::layout::docks::code_nav_dock::{reveal_code_target, CODE_EDITOR_ID};
use dioxus::prelude::*;
use mor_website_core::website::page_assets::{map_page_assets, AssetKind};

/// Sentinel keys for non-project buffers in the file dropdown.
const BUF_THEME_TOML: &str = ":theme.toml";
const BUF_COMPILED_CSS: &str = ":compiled.css";

#[derive(Clone, PartialEq)]
struct FileOption {
    /// Dropdown value / buffer key (project-relative path or sentinel).
    key: String,
    /// Human label shown in the select.
    label: String,
    /// Group header for optgroup (empty = ungrouped).
    group: String,
}

fn mode_for_path(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".css") {
        "css"
    } else if lower.ends_with(".js") || lower.ends_with(".mjs") {
        "javascript"
    } else if lower.ends_with(".toml") {
        "toml"
    } else if lower.ends_with(".json") {
        "json"
    } else if lower.ends_with(".php") || lower.ends_with(".html") || lower.ends_with(".htm") {
        // CM6 has no PHP grammar in our bundle; html mode still highlights markup well.
        "html"
    } else {
        "html"
    }
}

fn read_project_file(root: &std::path::Path, rel: &str) -> Result<String, String> {
    if rel.contains("..") {
        return Err("path escapes project".into());
    }
    std::fs::read_to_string(root.join(rel)).map_err(|e| format!("Could not read {rel}: {e}"))
}

fn write_project_file(root: &std::path::Path, rel: &str, content: &str) -> Result<(), String> {
    if rel.contains("..") {
        return Err("path escapes project".into());
    }
    let dest = root.join(rel);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&dest, content).map_err(|e| format!("Could not write {rel}: {e}"))
}

/// Build the dropdown options for the open page: page → includes → css → js,
/// then theme config tools at the end.
fn related_file_options(
    root: &std::path::Path,
    page: &str,
    all_pages: &[String],
) -> Vec<FileOption> {
    let mut out = Vec::new();

    // Always list the active page first.
    out.push(FileOption {
        key: page.to_string(),
        label: format!("Page · {page}"),
        group: "This page".into(),
    });

    let map = map_page_assets(root, page);
    let mut includes = Vec::new();
    let mut css = Vec::new();
    let mut js = Vec::new();
    for n in &map.nodes {
        if !n.exists {
            continue;
        }
        match n.kind {
            AssetKind::Include => includes.push(n.path.clone()),
            AssetKind::Css => css.push(n.path.clone()),
            AssetKind::Js => js.push(n.path.clone()),
            AssetKind::Page => {
                // root page already added
            }
            AssetKind::ExternalCss | AssetKind::ExternalJs => {}
        }
    }
    includes.sort();
    includes.dedup();
    css.sort();
    css.dedup();
    js.sort();
    js.dedup();

    for p in includes {
        out.push(FileOption {
            key: p.clone(),
            label: format!("Include · {p}"),
            group: "Includes".into(),
        });
    }
    for p in css {
        out.push(FileOption {
            key: p.clone(),
            label: format!("CSS · {p}"),
            group: "Stylesheets".into(),
        });
    }
    for p in js {
        out.push(FileOption {
            key: p.clone(),
            label: format!("JS · {p}"),
            group: "Scripts".into(),
        });
    }

    // Other project pages (quick jump without leaving Code).
    for p in all_pages {
        if p == page {
            continue;
        }
        out.push(FileOption {
            key: p.clone(),
            label: p.clone(),
            group: "Other pages".into(),
        });
    }

    out.push(FileOption {
        key: BUF_THEME_TOML.into(),
        label: "theme_config.toml (live tokens)".into(),
        group: "Theme tools".into(),
    });
    out.push(FileOption {
        key: BUF_COMPILED_CSS.into(),
        label: "mor-theme.css (compiled, read-only)".into(),
        group: "Theme tools".into(),
    });

    out
}

#[component]
pub fn SmartCodeDock(
    config_toml: ReadSignal<String>,
    on_load_theme: EventHandler<String>,
    #[props(default)] active_xray_target: Option<Signal<Option<String>>>,
) -> Element {
    let layout = use_context::<LayoutState>();
    let website = use_context::<WebsiteState>();
    let mut vfs = use_context::<VfsDictionary>().0;
    let mut is_takeover = use_signal(|| false);
    // Shared with Code Nav: true = compiled theme CSS (legacy name).
    let mut show_xml = layout.code_show_xml;

    let mut active_key = use_signal(|| String::new());
    let mut buffer = use_signal(String::new);
    let mut status = use_signal(String::new);
    let mut dirty = use_signal(|| false);
    // Track which page the related-file list was built for.
    let mut options_for_page = use_signal(String::new);

    let project = website.project.read().clone();
    let project_open = project.is_open();
    let current_page = website
        .current_page
        .read()
        .clone()
        .or_else(|| project.default_page().map(str::to_string));

    // Compiled mor-theme.css from live config (theme tools).
    let export_css = use_memo(move || {
        match toml::from_str::<mor_website_core::config::ThemeConfig>(&config_toml()) {
            Ok(config) => {
                crate::app::services::workspace_service::build_fresh_export_css(&config)
            }
            Err(err) => format!("Render failed: could not parse TOML: {}", err),
        }
    });

    // X-Ray still jumps into live theme TOML.
    if let Some(mut target_sig) = active_xray_target {
        use_effect(move || {
            if let Some(target_str) = target_sig() {
                active_key.set(BUF_THEME_TOML.into());
                show_xml.set(false);
                buffer.set(config_toml());
                dirty.set(false);
                reveal_code_target(target_str);
                target_sig.set(None);
            }
        });
    }

    // When project/page changes: default to the PHP page.
    {
        let page_for_fx = current_page.clone();
        let pages_for_fx = project.pages.clone();
        use_effect(move || {
            if !project_open {
                if active_key().is_empty() || !active_key().starts_with(':') {
                    let key = if show_xml() {
                        BUF_COMPILED_CSS
                    } else {
                        BUF_THEME_TOML
                    };
                    active_key.set(key.into());
                }
                return;
            }
            let Some(page) = page_for_fx.clone() else {
                return;
            };
            let key = active_key();
            let is_page_file = pages_for_fx.iter().any(|p| p == &key)
                || key.ends_with(".php")
                || key.ends_with(".html")
                || key.ends_with(".htm");
            // When the selected page changes, snap editor to that page's source.
            if options_for_page() != page {
                options_for_page.set(page.clone());
                if key.is_empty()
                    || key == BUF_THEME_TOML
                    || key == BUF_COMPILED_CSS
                    || is_page_file
                {
                    active_key.set(page);
                }
            } else if key.is_empty() {
                active_key.set(page);
            }
        });
    }

    // Load buffer whenever active_key changes.
    use_effect(move || {
        let key = active_key();
        if key.is_empty() {
            return;
        }
        if key == BUF_THEME_TOML {
            buffer.set(config_toml());
            dirty.set(false);
            show_xml.set(false);
            status.set(String::new());
            return;
        }
        if key == BUF_COMPILED_CSS {
            buffer.set(export_css());
            dirty.set(false);
            show_xml.set(true);
            status.set(String::new());
            return;
        }
        show_xml.set(false);
        let project = website.project.peek().clone();
        if !project.is_open() {
            buffer.set(format!("// Open a website folder to edit {key}\n"));
            dirty.set(false);
            return;
        }
        // Prefer VFS for CSS/JS (may have unsaved dock edits), else disk.
        if let Some(v) = vfs.peek().get(&key).cloned() {
            buffer.set(v);
            dirty.set(false);
            status.set(String::new());
            return;
        }
        match read_project_file(&project.root, &key) {
            Ok(text) => {
                buffer.set(text);
                dirty.set(false);
                status.set(String::new());
            }
            Err(e) => {
                buffer.set(format!("/* {e} */\n"));
                dirty.set(false);
                status.set(e);
            }
        }
    });

    let options: Vec<FileOption> = if project_open {
        if let Some(ref page) = current_page {
            related_file_options(&project.root, page, &project.pages)
        } else {
            vec![
                FileOption {
                    key: BUF_THEME_TOML.into(),
                    label: "theme_config.toml".into(),
                    group: "Theme tools".into(),
                },
                FileOption {
                    key: BUF_COMPILED_CSS.into(),
                    label: "mor-theme.css (compiled)".into(),
                    group: "Theme tools".into(),
                },
            ]
        }
    } else {
        vec![
            FileOption {
                key: BUF_THEME_TOML.into(),
                label: "theme_config.toml".into(),
                group: String::new(),
            },
            FileOption {
                key: BUF_COMPILED_CSS.into(),
                label: "mor-theme.css (compiled)".into(),
                group: String::new(),
            },
        ]
    };

    let key_now = active_key();
    let read_only = key_now == BUF_COMPILED_CSS;
    let mode = if key_now == BUF_THEME_TOML {
        "toml".to_string()
    } else if key_now == BUF_COMPILED_CSS {
        "css".to_string()
    } else {
        mode_for_path(&key_now).to_string()
    };

    let filename = if key_now.is_empty() {
        "—".to_string()
    } else if key_now == BUF_THEME_TOML {
        "theme_config.toml".into()
    } else if key_now == BUF_COMPILED_CSS {
        "mor-theme.css".into()
    } else {
        key_now.clone()
    };

    let badge = if read_only {
        "Compiled · Read-only"
    } else if dirty() {
        "Unsaved"
    } else if key_now == BUF_THEME_TOML {
        "Live Reload Active"
    } else {
        "Project file"
    };

    let on_buffer_change = move |new_val: String| {
        buffer.set(new_val.clone());
        dirty.set(true);
        let key = active_key();
        if key == BUF_THEME_TOML {
            on_load_theme.call(new_val);
            dirty.set(false); // theme applies live; optional save is separate
        } else if key != BUF_COMPILED_CSS && !key.is_empty() {
            // Keep VFS in sync for CSS/JS so other docks see edits.
            if key.ends_with(".css") || key.ends_with(".js") || key.ends_with(".mjs") {
                vfs.write().insert(key, new_val);
            }
        }
    };

    let do_save = move |_| {
        let key = active_key();
        if key.is_empty() || key == BUF_COMPILED_CSS {
            return;
        }
        if key == BUF_THEME_TOML {
            crate::utils::io::save_toml(&config_toml());
            status.set("Saved theme config.".into());
            dirty.set(false);
            return;
        }
        let project = website.project.peek().clone();
        if !project.is_open() {
            status.set("No website folder open.".into());
            return;
        }
        let content = buffer();
        match write_project_file(&project.root, &key, &content) {
            Ok(()) => {
                if key.ends_with(".css") || key.ends_with(".js") || key.ends_with(".mjs") {
                    vfs.write().insert(key.clone(), content);
                }
                website.bump_preview();
                dirty.set(false);
                status.set(format!("Saved {key}"));
            }
            Err(e) => status.set(e),
        }
    };

    // Stable-enough host id per language so CodeMirror remounts when mode changes.
    let editor_id = format!("{CODE_EDITOR_ID}-{mode}");

    let editor = rsx! {
        CodeEditor {
            id: Some(editor_id.clone()),
            value: buffer(),
            mode: mode.clone(),
            read_only: read_only,
            minimap_key: Some(format!("code_editor_{mode}")),
            on_change: on_buffer_change,
        }
    };

    // Group options for <optgroup>
    let mut groups: Vec<(String, Vec<FileOption>)> = Vec::new();
    for opt in options {
        if let Some(slot) = groups.iter_mut().find(|(g, _)| g == &opt.group) {
            slot.1.push(opt);
        } else {
            groups.push((opt.group.clone(), vec![opt]));
        }
    }

    let page_list = project.pages.clone();

    let header_controls = rsx! {
        div {
            style: "display: flex; align-items: center; gap: 6px; flex-wrap: wrap;",

            if project_open && !page_list.is_empty() {
                select {
                    class: "editor-select",
                    style: "max-width: 160px; font-size: 0.78rem;",
                    title: "Page whose source and linked assets are shown",
                    value: current_page.clone().unwrap_or_default(),
                    onchange: move |evt| {
                        let page = evt.value();
                        let mut current_page = website.current_page;
                        current_page.set(Some(page.clone()));
                        website.bump_preview();
                        active_key.set(page);
                        dirty.set(false);
                    },
                    for p in page_list.iter() {
                        option { value: "{p}", "{p}" }
                    }
                }
            }

            select {
                class: "editor-select",
                style: "max-width: 280px; font-size: 0.78rem;",
                title: "Edit this page, or a stylesheet / script it uses",
                value: "{key_now}",
                onchange: move |evt| {
                    let next = evt.value();
                    if dirty() && active_key() != BUF_THEME_TOML {
                        // Soft warn only — auto-switch (user can save first).
                        status.set("Switched file (previous buffer had unsaved edits).".into());
                    }
                    active_key.set(next);
                },
                for (group, opts) in groups.iter() {
                    if group.is_empty() {
                        for o in opts {
                            option { value: "{o.key}", selected: o.key == key_now, "{o.label}" }
                        }
                    } else {
                        optgroup {
                            label: "{group}",
                            for o in opts {
                                option { value: "{o.key}", selected: o.key == key_now, "{o.label}" }
                            }
                        }
                    }
                }
            }

            div { style: "width: 1px; height: 16px; background: var(--editor-border-soft); margin: 0 2px;" }
            button {
                class: "editor-mini-button",
                title: "Expand the editor to a full-viewport focused stage",
                onclick: move |_| is_takeover.set(true),
                "Takeover"
            }
            if !read_only {
                button {
                    class: if dirty() { "editor-mini-button editor-mini-button-active" } else { "editor-mini-button" },
                    title: "Save current file to disk",
                    onclick: do_save,
                    "Save"
                }
            }
        }
    };

    rsx! {
        if is_takeover() {
            div {
                style: "flex: 1; min-height: 0; display: flex; flex-direction: column; overflow: hidden;",
                div {
                    style: "flex-shrink: 0; display: flex; align-items: center; gap: 8px; padding: 6px 12px; background: var(--bg-elevated); border-bottom: 1px solid var(--editor-border);",
                    span {
                        style: "font-family: monospace; font-size: 0.85rem; font-weight: bold; color: var(--fg-base);",
                        "{filename}"
                    }
                    if dirty() {
                        span { style: "color: var(--editor-warning, #d29922); font-size: 0.75rem;", "●" }
                    }
                    div { style: "flex: 1;" }
                    {header_controls}
                    div { style: "width: 1px; height: 16px; background: var(--editor-border-soft); margin: 0 2px;" }
                    button {
                        class: "editor-mini-button",
                        onclick: move |_| is_takeover.set(false),
                        "Editor ×"
                    }
                }
                if !status().is_empty() {
                    div {
                        style: "flex-shrink: 0; padding: 4px 12px; font-size: 0.75rem; color: var(--fg-muted); border-bottom: 1px solid var(--editor-border-soft);",
                        "{status}"
                    }
                }
                div {
                    style: "flex: 1; min-height: 0; display: flex; flex-direction: column;",
                    {editor.clone()}
                }
            }
        } else {
            div {
                class: "export-viewport",
                style: "display: flex; flex-direction: column; min-width: 0; height: 100%; border: 1px solid var(--editor-border); border-radius: var(--radius-md); overflow: hidden; background: var(--bg-base);",
                div {
                    class: "editor-pane-header",
                    style: "display: flex; justify-content: space-between; align-items: center; gap: 8px; padding: 8px 12px; background: rgba(0,0,0,0.2); border-bottom: 1px solid var(--border-color); flex-shrink: 0; flex-wrap: wrap;",
                    div {
                        style: "display: flex; align-items: center; gap: 8px; min-width: 0;",
                        span {
                            style: "font-family: monospace; font-size: 0.85rem; font-weight: bold; color: var(--fg-base); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 220px;",
                            title: "{filename}",
                            "{filename}"
                        }
                        span {
                            style: "font-size: 0.7rem; font-weight: 600; color: var(--editor-accent); background: rgba(0,0,0,0.25); padding: 2px 6px; border-radius: 4px; border: 1px solid var(--editor-border-soft); white-space: nowrap;",
                            "{badge}"
                        }
                    }
                    {header_controls}
                }
                if !status().is_empty() {
                    div {
                        style: "flex-shrink: 0; padding: 4px 12px; font-size: 0.75rem; color: var(--fg-muted); border-bottom: 1px solid var(--editor-border-soft);",
                        "{status}"
                    }
                }
                div {
                    style: "display: flex; flex-direction: column; flex: 1; min-height: 0;",
                    {editor}
                }
            }
        }
    }
}
