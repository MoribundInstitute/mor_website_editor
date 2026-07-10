//! Plugin Manager — OnlyOffice-inspired layout:
//! Available plugins | Marketplace, toolbar (category + search), card grid.

use crate::app::config_bridge::{CompendiumManifest, PluginState};
use crate::app::plugin_registry::{
    fetch_marketplace, fallback_compendium, DEFAULT_MARKETPLACE_URL,
};
use crate::app::state::{DockPosition, LayoutState, PluginManagerContext};
use crate::ui::components::dock_chrome::DockChrome;
use crate::utils::mcp_installer::{
    install_mcp_binary_from_disk, install_official_mcp_engine, install_plugin_from_github,
    is_mcp_bridge_plugin, list_installed_mcp_binaries, OFFICIAL_MCP_PLUGIN_ID, OFFICIAL_MCP_REPO,
};
use dioxus::prelude::*;
use rfd::FileDialog;

#[derive(Clone, Copy, PartialEq)]
enum ManagerTab {
    /// Installed / enabled plugins (OnlyOffice "Available plugins").
    Available,
    /// Remote catalog + install sources (OnlyOffice "Marketplace").
    Marketplace,
}

/// Soft tile colors for marketplace cards (OnlyOffice-style variety).
const TILE_COLORS: &[&str] = &[
    "#c4b5fd", // violet
    "#fde68a", // yellow
    "#a7f3d0", // mint
    "#fbcfe8", // pink
    "#bfdbfe", // blue
    "#fed7aa", // peach
    "#ddd6fe", // lavender
    "#bbf7d0", // green
];

fn tile_color_for(id: &str) -> &'static str {
    let h = id.bytes().fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    TILE_COLORS[(h as usize) % TILE_COLORS.len()]
}

fn initial_glyph(name: &str) -> String {
    name.chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".into())
}

/// Infer a coarse category from id/name for the filter dropdown.
fn infer_category(id: &str, name: &str) -> &'static str {
    let s = format!("{id} {name}").to_ascii_lowercase();
    if s.contains("mcp") || s.contains("ai") || s.contains("llm") {
        "AI / MCP"
    } else if s.contains("theme") || s.contains("color") || s.contains("chameleon") {
        "Theme"
    } else if s.contains("publish") || s.contains("ssh") || s.contains("deploy") {
        "Publish"
    } else if s.contains("bell") || s.contains("notif") || s.contains("widget") {
        "Widgets"
    } else {
        "General"
    }
}

const PLUGIN_MGR_CSS: &str = r#"
.mor-pm {
  display: flex; flex-direction: column; height: calc(100% - 45px);
  overflow: hidden; background: var(--bg-panel); color: var(--fg-base); font-size: 0.85rem;
}
.mor-pm-banner {
  padding: 8px 12px; font-size: 0.78rem; border-bottom: 1px solid var(--editor-border-soft);
  flex-shrink: 0;
}
.mor-pm-banner.warn {
  background: color-mix(in srgb, var(--editor-warning, #8fa8c4) 14%, transparent);
  color: var(--editor-warning, #8fa8c4);
}
.mor-pm-banner.ok {
  background: color-mix(in srgb, var(--editor-good, #46c08a) 14%, transparent);
  color: var(--editor-good, #46c08a);
}
.mor-pm-banner.err {
  background: color-mix(in srgb, var(--editor-danger, #f0506b) 14%, transparent);
  color: var(--editor-danger, #f0506b);
}
.mor-pm-tabs {
  display: flex; align-items: center; gap: 6px;
  padding: 10px 12px 0; flex-shrink: 0; flex-wrap: wrap;
}
.mor-pm-tab {
  padding: 6px 12px; border-radius: 6px; border: 1px solid var(--border, #333);
  background: transparent; color: var(--fg-muted); cursor: pointer; font: inherit; font-size: 0.8rem;
}
.mor-pm-tab:hover { color: var(--fg-base); border-color: var(--border-light, #444); }
.mor-pm-tab.is-active {
  background: color-mix(in srgb, var(--accent, #6d8fb8) 18%, transparent);
  border-color: color-mix(in srgb, var(--accent, #6d8fb8) 45%, var(--border, #333));
  color: var(--fg-base); font-weight: 600;
}
.mor-pm-dev-link {
  margin-left: auto; background: none; border: none; color: var(--accent, #6d8fb8);
  font: inherit; font-size: 0.78rem; cursor: pointer; text-decoration: underline;
  text-underline-offset: 2px; padding: 4px 0;
}
.mor-pm-dev-link:hover { color: var(--accent-hover, #82a3c9); }
.mor-pm-toolbar {
  display: flex; align-items: center; gap: 8px; flex-wrap: wrap;
  padding: 10px 12px; flex-shrink: 0; border-bottom: 1px solid var(--border, #333);
}
.mor-pm-toolbar label { font-size: 0.75rem; color: var(--fg-muted); }
.mor-pm-select, .mor-pm-search {
  font: inherit; font-size: 0.8rem; color: var(--fg-base);
  background: var(--bg-elevated, #1c1f28); border: 1px solid var(--border, #333);
  border-radius: 6px; padding: 5px 8px;
}
.mor-pm-search { flex: 1; min-width: 120px; }
.mor-pm-body {
  flex: 1; overflow-y: auto; padding: 12px;
}
.mor-pm-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(168px, 1fr));
  gap: 12px;
}
.mor-pm-card {
  display: flex; flex-direction: column;
  background: var(--bg-elevated, #1c1f28);
  border: 1px solid var(--border, #333);
  border-radius: 10px; overflow: hidden;
  min-height: 200px;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}
.mor-pm-card:hover {
  border-color: color-mix(in srgb, var(--accent, #6d8fb8) 40%, var(--border, #333));
  box-shadow: 0 6px 18px rgba(0,0,0,0.22);
}
.mor-pm-card-art {
  height: 88px; display: flex; align-items: center; justify-content: center;
  font-size: 2rem; font-weight: 700; color: rgba(0,0,0,0.45);
  letter-spacing: -0.02em; user-select: none;
}
.mor-pm-card-body {
  display: flex; flex-direction: column; gap: 6px;
  padding: 10px 12px 12px; flex: 1;
}
.mor-pm-card-title {
  font-weight: 650; font-size: 0.88rem; color: var(--fg-base);
  line-height: 1.25; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.mor-pm-card-desc {
  margin: 0; flex: 1;
  font-size: 0.72rem; line-height: 1.4; color: var(--fg-muted);
  display: -webkit-box; -webkit-line-clamp: 3; -webkit-box-orient: vertical; overflow: hidden;
}
.mor-pm-card-meta {
  display: flex; align-items: center; gap: 6px; flex-wrap: wrap;
  font-size: 0.68rem; color: var(--fg-muted);
}
.mor-pm-badge {
  font-size: 0.62rem; font-weight: 700; letter-spacing: 0.04em;
  padding: 2px 6px; border-radius: 4px; text-transform: uppercase;
}
.mor-pm-badge.mcp {
  background: color-mix(in srgb, var(--editor-good, #46c08a) 18%, transparent);
  color: var(--editor-good, #46c08a);
}
.mor-pm-badge.cat {
  background: color-mix(in srgb, var(--accent, #6d8fb8) 14%, transparent);
  color: var(--fg-muted);
}
.mor-pm-stars {
  letter-spacing: 0.5px; color: #e8b84a; font-size: 0.7rem;
}
.mor-pm-card-actions {
  display: flex; gap: 6px; margin-top: 4px;
}
.mor-pm-card-actions button {
  flex: 1; font: inherit; font-size: 0.75rem; padding: 5px 8px;
  border-radius: 6px; cursor: pointer; border: 1px solid var(--border, #333);
  background: var(--bg-panel, #151820); color: var(--fg-base);
}
.mor-pm-card-actions button.primary {
  background: color-mix(in srgb, var(--accent, #6d8fb8) 85%, #000);
  border-color: transparent; color: #fff; font-weight: 600;
}
.mor-pm-card-actions button.primary:hover {
  background: var(--accent-hover, #82a3c9);
}
.mor-pm-card-actions button.danger {
  color: var(--editor-danger, #f0506b);
  border-color: color-mix(in srgb, var(--editor-danger, #f0506b) 45%, var(--border, #333));
  background: transparent;
}
.mor-pm-card-actions button.update {
  background: color-mix(in srgb, var(--editor-good, #46c08a) 22%, transparent);
  border-color: color-mix(in srgb, var(--editor-good, #46c08a) 45%, var(--border, #333));
  color: var(--editor-good, #46c08a); font-weight: 600;
}
.mor-pm-empty {
  text-align: center; color: var(--fg-muted); font-size: 0.82rem;
  padding: 28px 12px; margin: 0;
}
.mor-pm-install-panel {
  margin-top: 14px; padding-top: 12px; border-top: 1px solid var(--border, #333);
  display: flex; flex-direction: column; gap: 8px;
}
.mor-pm-install-panel h4 {
  margin: 0; font-size: 0.72rem; font-weight: 700; letter-spacing: 0.06em;
  text-transform: uppercase; color: var(--fg-muted);
}
.mor-pm-dev-modal-backdrop {
  position: fixed; inset: 0; z-index: 9000;
  background: rgba(0,0,0,0.45);
  display: flex; align-items: center; justify-content: center;
}
.mor-pm-dev-modal {
  width: min(420px, 92vw); background: var(--bg-panel, #1a1d24);
  border: 1px solid var(--border, #333); border-radius: 10px;
  box-shadow: 0 16px 40px rgba(0,0,0,0.45); padding: 16px 18px;
  display: flex; flex-direction: column; gap: 12px;
}
.mor-pm-dev-modal h3 { margin: 0; font-size: 1rem; }
.mor-pm-dev-modal p { margin: 0; font-size: 0.8rem; line-height: 1.4; color: var(--fg-muted); }
.mor-pm-dev-modal .warn-line {
  color: var(--editor-danger, #f0506b); font-size: 0.78rem; font-weight: 600;
}
.mor-pm-dev-actions { display: flex; gap: 8px; justify-content: flex-end; margin-top: 4px; }
.mor-pm-enable {
  display: inline-flex; align-items: center; gap: 6px;
  font-size: 0.72rem; color: var(--fg-muted); cursor: pointer; user-select: none;
}
.mor-pm-hero {
  margin-bottom: 14px; padding: 12px 14px; border-radius: 10px;
  border: 1px solid color-mix(in srgb, var(--editor-good, #46c08a) 35%, var(--border, #333));
  background: color-mix(in srgb, var(--editor-good, #46c08a) 10%, transparent);
  display: flex; flex-direction: column; gap: 8px;
}
.mor-pm-hero h4 {
  margin: 0; font-size: 0.88rem; font-weight: 700; color: var(--fg-base);
}
.mor-pm-hero p {
  margin: 0; font-size: 0.75rem; line-height: 1.4; color: var(--fg-muted);
}
.mor-pm-hero-actions { display: flex; flex-wrap: wrap; gap: 8px; }
.mor-pm-hero-actions button {
  font: inherit; font-size: 0.78rem; font-weight: 600; padding: 7px 12px;
  border-radius: 6px; cursor: pointer; border: none;
  background: color-mix(in srgb, var(--editor-good, #46c08a) 88%, #000);
  color: #fff;
}
.mor-pm-hero-actions button:disabled { opacity: 0.55; cursor: wait; }
.mor-pm-hero-actions button.secondary {
  background: transparent; color: var(--fg-base);
  border: 1px solid var(--border, #333); font-weight: 500;
}
"#;

fn upsert_mcp_bridge_plugin(plugins: &mut Vec<PluginState>, version: &str) {
    if let Some(p) = plugins.iter_mut().find(|p| p.id == OFFICIAL_MCP_PLUGIN_ID) {
        p.enabled = true;
        p.version = version.to_string();
    } else {
        plugins.push(PluginState {
            id: OFFICIAL_MCP_PLUGIN_ID.to_string(),
            enabled: true,
            version: version.to_string(),
        });
    }
}

#[component]
pub fn PluginManagerDock() -> Element {
    let mut layout = use_context::<LayoutState>();
    let plugin_ctx = use_context::<PluginManagerContext>();
    let pos = (layout.plugin_manager_pos)();

    if pos == DockPosition::Hidden {
        return rsx! {};
    }

    let launch_state = plugin_ctx.launch_plugins;
    let mut current_state = plugin_ctx.current_plugins;
    let mut compendium_registry = plugin_ctx.compendium_registry;

    let mut active_tab = use_signal(|| ManagerTab::Available);
    let needs_restart = use_memo(move || *launch_state.read() != *current_state.read());
    let mut install_status = use_signal(|| Option::<Result<String, String>>::None);
    let mut repo_input = use_signal(String::new);
    let mut installed_plugins = use_signal(|| Vec::<String>::new());
    let mut search = use_signal(String::new);
    let mut category = use_signal(|| "All".to_string());
    let mut show_dev = use_signal(|| false);
    let mut marketplace_url = use_signal(|| DEFAULT_MARKETPLACE_URL.to_string());
    let mut refreshing = use_signal(|| false);
    let mut installing_mcp = use_signal(|| false);

    use_effect(move || {
        installed_plugins.set(list_installed_mcp_binaries());
    });

    let current_read = current_state.read().clone();
    let compendium_read = compendium_registry.read().clone();
    let q = search.read().trim().to_ascii_lowercase();
    let cat = category.read().clone();

    let mcp_daemon_cards = use_memo(move || {
        let Ok(registry) = crate::utils::mcp_installer::read_daemon_registry() else {
            return Vec::new();
        };
        let Some(servers) = registry.get("servers").and_then(|v| v.as_object()) else {
            return Vec::new();
        };

        servers
            .iter()
            .map(|(key, entry)| {
                let display_name = entry
                    .get("display_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(key)
                    .to_string();
                let prompt = entry
                    .get("system_prompt")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                (key.clone(), display_name, prompt)
            })
            .collect::<Vec<_>>()
    });

    let updates_available: Vec<(PluginState, CompendiumManifest)> = current_read
        .iter()
        .filter_map(|local| {
            compendium_read
                .iter()
                .find(|remote| remote.id == local.id)
                .and_then(|remote| {
                    if remote.version != local.version {
                        Some((local.clone(), remote.clone()))
                    } else {
                        None
                    }
                })
        })
        .collect();
    let updates_count = updates_available.len();
    let updates_for_toolbar = updates_available.clone();
    let updates_for_cards = updates_available.clone();

    // Categories present in marketplace
    let mut categories: Vec<String> = vec!["All".into()];
    for remote in &compendium_read {
        let c = infer_category(&remote.id, &remote.display_name).to_string();
        if !categories.iter().any(|x| x == &c) {
            categories.push(c);
        }
    }

    let matches_filter = |id: &str, name: &str, desc: &str| {
        let cat_ok = cat == "All" || infer_category(id, name) == cat.as_str();
        let q_ok = q.is_empty()
            || id.to_ascii_lowercase().contains(&q)
            || name.to_ascii_lowercase().contains(&q)
            || desc.to_ascii_lowercase().contains(&q);
        cat_ok && q_ok
    };

    let refresh_marketplace = move |_| {
        let url = marketplace_url.read().clone();
        refreshing.set(true);
        install_status.set(None);
        spawn(async move {
            let (list, warn) = fetch_marketplace(&url).await;
            compendium_registry.set(list);
            if let Some(msg) = warn {
                // Soft warning — built-in catalog (incl. MCP) is still usable.
                install_status.set(Some(Err(msg)));
            } else {
                install_status.set(Some(Ok("Marketplace registry refreshed.".into())));
            }
            refreshing.set(false);
        });
    };

    // Ensure registry is never empty (e.g. race before shell seed finishes).
    use_effect(move || {
        if compendium_registry.read().is_empty() {
            compendium_registry.set(fallback_compendium());
        }
    });

    let inner_content = rsx! {
        DockChrome {
            title: "Plugin Manager".to_string(),
            dock_id: "plugin_manager".to_string(),
            position: pos,
            on_close: move |_| {
                layout.plugin_manager_pos.set(DockPosition::Hidden);
            },
            style { "{PLUGIN_MGR_CSS}" }
            div { class: "mor-pm",

            if needs_restart() {
                div { class: "mor-pm-banner warn", "Restart required to apply plugin changes." }
            }
            match &*install_status.read() {
                Some(Ok(msg)) => rsx! { div { class: "mor-pm-banner ok", "{msg}" } },
                Some(Err(err)) => rsx! { div { class: "mor-pm-banner err", "{err}" } },
                None => rsx! {}
            }

            // ── Tab row (OnlyOffice: Available | Marketplace) ──────────────
            div { class: "mor-pm-tabs",
                button {
                    class: if active_tab() == ManagerTab::Available { "mor-pm-tab is-active" } else { "mor-pm-tab" },
                    onclick: move |_| active_tab.set(ManagerTab::Available),
                    "Available plugins"
                }
                button {
                    class: if active_tab() == ManagerTab::Marketplace { "mor-pm-tab is-active" } else { "mor-pm-tab" },
                    onclick: move |_| active_tab.set(ManagerTab::Marketplace),
                    "Marketplace"
                    if updates_count > 0 {
                        span { style: "margin-left: 4px; opacity: 0.8;", "({updates_count})" }
                    }
                }
                button {
                    class: "mor-pm-dev-link",
                    title: "Set a custom marketplace registry URL",
                    onclick: move |_| show_dev.set(true),
                    "Developer mode"
                }
            }

            // ── Toolbar: category + update all + search ─────────────────────
            div { class: "mor-pm-toolbar",
                label { "Categories" }
                select {
                    class: "mor-pm-select",
                    value: "{cat}",
                    onchange: move |e| category.set(e.value()),
                    for c in categories.iter() {
                        option { value: "{c}", selected: *c == cat, "{c}" }
                    }
                }
                if active_tab() == ManagerTab::Marketplace && updates_count > 0 {
                    button {
                        class: "mor-pm-tab",
                        title: "Apply all available version bumps",
                        onclick: move |_| {
                            let ups = updates_for_toolbar.clone();
                            let n = ups.len();
                            current_state.with_mut(|s| {
                                for (local, remote) in &ups {
                                    if let Some(p) = s.iter_mut().find(|p| p.id == local.id) {
                                        p.version = remote.version.clone();
                                    }
                                }
                            });
                            install_status.set(Some(Ok(format!("Updated {n} plugin(s)."))));
                        },
                        "Update All"
                    }
                }
                if active_tab() == ManagerTab::Marketplace {
                    button {
                        class: "mor-pm-tab",
                        disabled: refreshing(),
                        onclick: refresh_marketplace,
                        if refreshing() { "Refreshing…" } else { "Refresh" }
                    }
                }
                input {
                    class: "mor-pm-search",
                    r#type: "search",
                    placeholder: "Search plugins…",
                    value: "{search}",
                    oninput: move |e| search.set(e.value()),
                }
            }

            // ── Body ───────────────────────────────────────────────────────
            div { class: "mor-pm-body",
                match active_tab() {
                    ManagerTab::Available => rsx! {
                        {
                            let mcp = mcp_daemon_cards();
                            let plugins: Vec<_> = current_read
                                .iter()
                                .filter(|p| {
                                    matches_filter(&p.id, &p.id, "")
                                })
                                .cloned()
                                .collect();
                            let mcp_filtered: Vec<_> = mcp
                                .into_iter()
                                .filter(|(k, n, d)| matches_filter(k, n, d))
                                .collect();
                            let empty = mcp_filtered.is_empty() && plugins.is_empty();
                            rsx! {
                                if empty {
                                    div { class: "mor-pm-empty", style: "display:flex;flex-direction:column;align-items:center;gap:12px;",
                                        p { style: "margin:0;",
                                            "No plugins installed yet."
                                        }
                                        p { style: "margin:0;font-size:0.78rem;max-width:28rem;",
                                            "Install the MCP AI Bridge to let Claude / Grok build and theme sites (Robot Assist). Or open Marketplace for the full catalog."
                                        }
                                        div { class: "mor-pm-hero-actions", style: "justify-content:center;",
                                            button {
                                                disabled: installing_mcp(),
                                                onclick: move |_| {
                                                    if installing_mcp() { return; }
                                                    installing_mcp.set(true);
                                                    install_status.set(None);
                                                    spawn(async move {
                                                        match install_official_mcp_engine().await {
                                                            Ok(report) => {
                                                                let bin = report.binary_path.file_name()
                                                                    .and_then(|n| n.to_str()).unwrap_or("mor-mcp");
                                                                current_state.with_mut(|s| {
                                                                    upsert_mcp_bridge_plugin(s, "0.1.0");
                                                                });
                                                                installed_plugins.set(list_installed_mcp_binaries());
                                                                install_status.set(Some(Ok(format!(
                                                                    "MCP AI Bridge installed ({bin}). Enable Robot Assist in Preferences, then restart your MCP client."
                                                                ))));
                                                            }
                                                            Err(e) => install_status.set(Some(Err(e))),
                                                        }
                                                        installing_mcp.set(false);
                                                    });
                                                },
                                                if installing_mcp() { "Installing…" } else { "Install MCP AI Bridge" }
                                            }
                                            button {
                                                class: "secondary",
                                                onclick: move |_| active_tab.set(ManagerTab::Marketplace),
                                                "Open Marketplace"
                                            }
                                        }
                                    }
                                } else {
                                    div { class: "mor-pm-grid",
                                        for (server_key, display_name, prompt) in mcp_filtered {
                                            {
                                                let tint = tile_color_for(&server_key);
                                                let glyph = initial_glyph(&display_name);
                                                rsx! {
                                                    div {
                                                        key: "mcp-{server_key}",
                                                        class: "mor-pm-card",
                                                        div {
                                                            class: "mor-pm-card-art",
                                                            style: "background: {tint};",
                                                            "{glyph}"
                                                        }
                                                        div { class: "mor-pm-card-body",
                                                            div { class: "mor-pm-card-title", title: "{display_name}", "{display_name}" }
                                                            p { class: "mor-pm-card-desc",
                                                                if prompt.is_empty() {
                                                                    "MCP daemon server registered with the editor."
                                                                } else {
                                                                    "{prompt}"
                                                                }
                                                            }
                                                            div { class: "mor-pm-card-meta",
                                                                span { class: "mor-pm-badge mcp", "MCP" }
                                                                span { style: "font-family: monospace; font-size: 0.65rem;", "{server_key}" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        for local_plugin in plugins {
                                            {
                                                let remote = compendium_read.iter().find(|r| r.id == local_plugin.id);
                                                let title = remote
                                                    .map(|r| r.display_name.as_str())
                                                    .unwrap_or(local_plugin.id.as_str());
                                                let desc = remote
                                                    .map(|r| r.description.as_str())
                                                    .unwrap_or("Installed plugin");
                                                let tint = tile_color_for(&local_plugin.id);
                                                let glyph = initial_glyph(title);
                                                let id = local_plugin.id.clone();
                                                let id_rm = local_plugin.id.clone();
                                                let enabled = local_plugin.enabled;
                                                let ver = local_plugin.version.clone();
                                                let has_update = updates_for_cards.iter().any(|(l, _)| l.id == local_plugin.id);
                                                rsx! {
                                                    div {
                                                        key: "inst-{id}",
                                                        class: "mor-pm-card",
                                                        div {
                                                            class: "mor-pm-card-art",
                                                            style: "background: {tint};",
                                                            "{glyph}"
                                                        }
                                                        div { class: "mor-pm-card-body",
                                                            div { class: "mor-pm-card-title", title: "{title}", "{title}" }
                                                            p { class: "mor-pm-card-desc", "{desc}" }
                                                            div { class: "mor-pm-card-meta",
                                                                span { class: "mor-pm-badge cat", "{infer_category(&id, title)}" }
                                                                span { style: "font-family: monospace;", "v{ver}" }
                                                                if has_update {
                                                                    span { style: "color: var(--editor-good); font-weight: 600;", "Update available" }
                                                                }
                                                            }
                                                            label { class: "mor-pm-enable",
                                                                input {
                                                                    r#type: "checkbox",
                                                                    checked: enabled,
                                                                    onchange: {
                                                                        let id = id.clone();
                                                                        move |evt: FormEvent| {
                                                                            let on = evt.checked();
                                                                            current_state.with_mut(|s| {
                                                                                if let Some(p) = s.iter_mut().find(|p| p.id == id) {
                                                                                    p.enabled = on;
                                                                                }
                                                                            });
                                                                        }
                                                                    }
                                                                }
                                                                if enabled { "Enabled" } else { "Disabled" }
                                                            }
                                                            div { class: "mor-pm-card-actions",
                                                                button {
                                                                    class: "danger",
                                                                    onclick: move |_| {
                                                                        current_state.with_mut(|s| s.retain(|p| p.id != id_rm));
                                                                    },
                                                                    "Remove"
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
                    },
                    ManagerTab::Marketplace => rsx! {
                        {
                            let market: Vec<_> = compendium_read
                                .iter()
                                .filter(|r| matches_filter(&r.id, &r.display_name, &r.description))
                                .cloned()
                                .collect();
                            let mcp_already = current_read.iter().any(|p| is_mcp_bridge_plugin(&p.id))
                                || installed_plugins.read().iter().any(|n| {
                                    let n = n.to_ascii_lowercase();
                                    n.contains("mcp") || n.contains("mor-mcp")
                                });
                            rsx! {
                                // One-click official MCP engine
                                div { class: "mor-pm-hero",
                                    h4 { "MCP AI Bridge" }
                                    p {
                                        "Download the official engine from GitHub, register it with the editor, and add it to Claude Desktop config. "
                                        "Repo: {OFFICIAL_MCP_REPO}"
                                    }
                                    div { class: "mor-pm-hero-actions",
                                        button {
                                            disabled: installing_mcp(),
                                            onclick: move |_| {
                                                if installing_mcp() {
                                                    return;
                                                }
                                                installing_mcp.set(true);
                                                install_status.set(None);
                                                spawn(async move {
                                                    match install_official_mcp_engine().await {
                                                        Ok(report) => {
                                                            let bin = report
                                                                .binary_path
                                                                .file_name()
                                                                .and_then(|n| n.to_str())
                                                                .unwrap_or("mor-mcp")
                                                                .to_string();
                                                            current_state.with_mut(|s| {
                                                                upsert_mcp_bridge_plugin(
                                                                    s,
                                                                    "1.0.0",
                                                                );
                                                            });
                                                            installed_plugins
                                                                .set(list_installed_mcp_binaries());
                                                            let claude_note = report
                                                                .claude_config
                                                                .as_ref()
                                                                .map(|p| {
                                                                    format!(
                                                                        " Claude config: {}.",
                                                                        p.display()
                                                                    )
                                                                })
                                                                .unwrap_or_default();
                                                            install_status.set(Some(Ok(format!(
                                                                "MCP AI Bridge installed ({bin}). Restart Claude / your agent to load it.{claude_note}"
                                                            ))));
                                                        }
                                                        Err(e) => {
                                                            install_status.set(Some(Err(e)));
                                                        }
                                                    }
                                                    installing_mcp.set(false);
                                                });
                                            },
                                            if installing_mcp() {
                                                "Installing…"
                                            } else if mcp_already {
                                                "Reinstall MCP AI Bridge"
                                            } else {
                                                "Install MCP AI Bridge"
                                            }
                                        }
                                        button {
                                            class: "secondary",
                                            disabled: installing_mcp(),
                                            onclick: move |_| {
                                                if let Some(file_path) = FileDialog::new()
                                                    .set_title("Select mor-mcp binary")
                                                    .pick_file()
                                                {
                                                    match install_mcp_binary_from_disk(&file_path) {
                                                        Ok(report) => {
                                                            let bin = report
                                                                .binary_path
                                                                .file_name()
                                                                .and_then(|n| n.to_str())
                                                                .unwrap_or("mor-mcp")
                                                                .to_string();
                                                            current_state.with_mut(|s| {
                                                                upsert_mcp_bridge_plugin(
                                                                    s, "1.0.0",
                                                                );
                                                            });
                                                            installed_plugins
                                                                .set(list_installed_mcp_binaries());
                                                            install_status.set(Some(Ok(format!(
                                                                "Installed MCP binary from disk ({bin}). Restart your MCP client."
                                                            ))));
                                                        }
                                                        Err(e) => install_status
                                                            .set(Some(Err(format!(
                                                                "Failed to install MCP: {e}"
                                                            )))),
                                                    }
                                                }
                                            },
                                            "Install from Disk…"
                                        }
                                    }
                                }

                                if market.is_empty() {
                                    p { class: "mor-pm-empty",
                                        "No marketplace plugins match. Try Refresh, clear search, or open Developer mode to set a registry URL."
                                    }
                                } else {
                                    div { class: "mor-pm-grid",
                                        for remote in market {
                                            {
                                                let installed = current_read.iter().find(|l| l.id == remote.id);
                                                let update = updates_for_cards.iter().find(|(l, _)| l.id == remote.id);
                                                let tint = tile_color_for(&remote.id);
                                                let glyph = initial_glyph(&remote.display_name);
                                                let cat_label = infer_category(&remote.id, &remote.display_name);
                                                // Decorative “rating” from version hash — visual parity with OnlyOffice cards.
                                                let stars = 3 + (remote.id.bytes().map(|b| b as usize).sum::<usize>() % 3);
                                                let star_str: String = (0..5)
                                                    .map(|i| if i < stars { '★' } else { '☆' })
                                                    .collect();
                                                let rid = remote.id.clone();
                                                let rver = remote.version.clone();
                                                let is_mcp = is_mcp_bridge_plugin(&remote.id);
                                                rsx! {
                                                    div {
                                                        key: "mkt-{remote.id}",
                                                        class: "mor-pm-card",
                                                        div {
                                                            class: "mor-pm-card-art",
                                                            style: "background: {tint};",
                                                            "{glyph}"
                                                        }
                                                        div { class: "mor-pm-card-body",
                                                            div { class: "mor-pm-card-title", title: "{remote.display_name}", "{remote.display_name}" }
                                                            p { class: "mor-pm-card-desc", "{remote.description}" }
                                                            div { class: "mor-pm-card-meta",
                                                                span { class: "mor-pm-stars", title: "Community signal (placeholder)", "{star_str}" }
                                                                if is_mcp {
                                                                    span { class: "mor-pm-badge mcp", "MCP" }
                                                                }
                                                                span { class: "mor-pm-badge cat", "{cat_label}" }
                                                                span { style: "font-family: monospace;", "v{remote.version}" }
                                                            }
                                                            div { class: "mor-pm-card-actions",
                                                                if let Some((_, remote_up)) = update {
                                                                    {
                                                                        let id = rid.clone();
                                                                        let ver = remote_up.version.clone();
                                                                        let mcp_update = is_mcp;
                                                                        rsx! {
                                                                            button {
                                                                                class: "update",
                                                                                disabled: installing_mcp() && mcp_update,
                                                                                onclick: move |_| {
                                                                                    if mcp_update {
                                                                                        if installing_mcp() {
                                                                                            return;
                                                                                        }
                                                                                        installing_mcp.set(true);
                                                                                        install_status.set(None);
                                                                                        let id = id.clone();
                                                                                        let ver = ver.clone();
                                                                                        spawn(async move {
                                                                                            match install_official_mcp_engine().await {
                                                                                                Ok(report) => {
                                                                                                    current_state.with_mut(|s| {
                                                                                                        if let Some(p) = s.iter_mut().find(|p| p.id == id) {
                                                                                                            p.version = ver;
                                                                                                            p.enabled = true;
                                                                                                        }
                                                                                                    });
                                                                                                    installed_plugins.set(list_installed_mcp_binaries());
                                                                                                    let bin = report.binary_path.file_name()
                                                                                                        .and_then(|n| n.to_str()).unwrap_or("mor-mcp");
                                                                                                    install_status.set(Some(Ok(format!(
                                                                                                        "Updated MCP AI Bridge ({bin})."
                                                                                                    ))));
                                                                                                }
                                                                                                Err(e) => install_status.set(Some(Err(e))),
                                                                                            }
                                                                                            installing_mcp.set(false);
                                                                                        });
                                                                                    } else {
                                                                                        current_state.with_mut(|s| {
                                                                                            if let Some(p) = s.iter_mut().find(|p| p.id == id) {
                                                                                                p.version = ver.clone();
                                                                                            }
                                                                                        });
                                                                                    }
                                                                                },
                                                                                if mcp_update { "Update engine" } else { "Update" }
                                                                            }
                                                                        }
                                                                    }
                                                                } else if installed.is_some() && !is_mcp {
                                                                    button {
                                                                        class: "primary",
                                                                        disabled: true,
                                                                        style: "opacity: 0.65; cursor: default;",
                                                                        "Installed"
                                                                    }
                                                                } else if installed.is_some() && is_mcp {
                                                                    button {
                                                                        class: "primary",
                                                                        disabled: installing_mcp(),
                                                                        onclick: move |_| {
                                                                            if installing_mcp() {
                                                                                return;
                                                                            }
                                                                            installing_mcp.set(true);
                                                                            install_status.set(None);
                                                                            spawn(async move {
                                                                                match install_official_mcp_engine().await {
                                                                                    Ok(report) => {
                                                                                        installed_plugins.set(list_installed_mcp_binaries());
                                                                                        let bin = report.binary_path.file_name()
                                                                                            .and_then(|n| n.to_str()).unwrap_or("mor-mcp");
                                                                                        install_status.set(Some(Ok(format!(
                                                                                            "Reinstalled MCP AI Bridge ({bin})."
                                                                                        ))));
                                                                                    }
                                                                                    Err(e) => install_status.set(Some(Err(e))),
                                                                                }
                                                                                installing_mcp.set(false);
                                                                            });
                                                                        },
                                                                        if installing_mcp() { "Installing…" } else { "Reinstall" }
                                                                    }
                                                                } else {
                                                                    button {
                                                                        class: "primary",
                                                                        disabled: installing_mcp() && is_mcp,
                                                                        onclick: move |_| {
                                                                            if is_mcp {
                                                                                if installing_mcp() {
                                                                                    return;
                                                                                }
                                                                                installing_mcp.set(true);
                                                                                install_status.set(None);
                                                                                let ver = rver.clone();
                                                                                spawn(async move {
                                                                                    match install_official_mcp_engine().await {
                                                                                        Ok(report) => {
                                                                                            current_state.with_mut(|s| {
                                                                                                upsert_mcp_bridge_plugin(s, &ver);
                                                                                            });
                                                                                            installed_plugins.set(list_installed_mcp_binaries());
                                                                                            let bin = report.binary_path.file_name()
                                                                                                .and_then(|n| n.to_str()).unwrap_or("mor-mcp");
                                                                                            install_status.set(Some(Ok(format!(
                                                                                                "MCP AI Bridge installed ({bin}). Restart your MCP client."
                                                                                            ))));
                                                                                        }
                                                                                        Err(e) => install_status.set(Some(Err(e))),
                                                                                    }
                                                                                    installing_mcp.set(false);
                                                                                });
                                                                            } else {
                                                                                current_state.with_mut(|s| {
                                                                                    s.push(PluginState {
                                                                                        id: rid.clone(),
                                                                                        enabled: true,
                                                                                        version: rver.clone(),
                                                                                    });
                                                                                });
                                                                                install_status.set(Some(Ok(format!("Installed {rid}."))));
                                                                            }
                                                                        },
                                                                        if is_mcp && installing_mcp() {
                                                                            "Installing…"
                                                                        } else {
                                                                            "Install"
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

                                // Manual install (disk / GitHub) — secondary, under the grid
                                div { class: "mor-pm-install-panel",
                                    h4 { "Install manually" }
                                    button {
                                        class: "mor-btn",
                                        style: "width: 100%; border-style: dashed; font-size: 0.8rem;",
                                        disabled: installing_mcp(),
                                        onclick: move |_| {
                                            if let Some(file_path) = FileDialog::new()
                                                .set_title("Select MorWebsite MCP Binary (mor-mcp)")
                                                .pick_file()
                                            {
                                                match install_mcp_binary_from_disk(&file_path) {
                                                    Ok(report) => {
                                                        let bin = report
                                                            .binary_path
                                                            .file_name()
                                                            .and_then(|n| n.to_str())
                                                            .unwrap_or("mor-mcp")
                                                            .to_string();
                                                        current_state.with_mut(|s| {
                                                            upsert_mcp_bridge_plugin(s, "1.0.0");
                                                        });
                                                        installed_plugins.set(list_installed_mcp_binaries());
                                                        install_status.set(Some(Ok(format!(
                                                            "Successfully installed MCP plugin ({bin}). Restart your MCP client."
                                                        ))));
                                                    }
                                                    Err(e) => {
                                                        install_status.set(Some(Err(format!(
                                                            "Failed to install MCP: {e}"
                                                        ))));
                                                    }
                                                }
                                            }
                                        },
                                        "+ Install from Disk"
                                    }
                                    input {
                                        class: "mor-pm-search",
                                        style: "width: 100%; box-sizing: border-box;",
                                        placeholder: format!("Author/Repo (e.g. {OFFICIAL_MCP_REPO})"),
                                        value: "{repo_input}",
                                        oninput: move |evt| repo_input.set(evt.value())
                                    }
                                    button {
                                        class: "mor-btn-primary",
                                        style: "width: 100%; font-size: 0.8rem;",
                                        disabled: installing_mcp(),
                                        onclick: move |_| {
                                            let repo = repo_input.read().clone();
                                            if repo.is_empty() {
                                                return;
                                            }
                                            installing_mcp.set(true);
                                            install_status.set(None);
                                            spawn(async move {
                                                match install_plugin_from_github(&repo).await {
                                                    Ok(file) => {
                                                        if is_mcp_bridge_plugin(&repo)
                                                            || file.to_ascii_lowercase().contains("mcp")
                                                            || repo.contains("mcp")
                                                        {
                                                            current_state.with_mut(|s| {
                                                                upsert_mcp_bridge_plugin(
                                                                    s, "1.0.0",
                                                                );
                                                            });
                                                        }
                                                        installed_plugins
                                                            .set(list_installed_mcp_binaries());
                                                        install_status.set(Some(Ok(format!(
                                                            "Successfully installed plugin: {file}"
                                                        ))));
                                                    }
                                                    Err(e) => {
                                                        install_status.set(Some(Err(format!(
                                                            "GitHub Install Failed: {e}"
                                                        ))));
                                                    }
                                                }
                                                installing_mcp.set(false);
                                            });
                                        },
                                        "Fetch from GitHub"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Developer mode modal (OnlyOffice-style registry URL)
            if show_dev() {
                div {
                    class: "mor-pm-dev-modal-backdrop",
                    onclick: move |_| show_dev.set(false),
                    div {
                        class: "mor-pm-dev-modal",
                        onclick: move |e| e.stop_propagation(),
                        h3 { "Developer Mode" }
                        p { class: "warn-line", "Be careful! Avoid untrusted registry URLs." }
                        p { "Enter the URL for a marketplace registry JSON (array of plugin manifests)." }
                        input {
                            class: "mor-pm-search",
                            style: "width: 100%; box-sizing: border-box;",
                            placeholder: "https://…/registry.json",
                            value: "{marketplace_url}",
                            oninput: move |e| marketplace_url.set(e.value()),
                        }
                        button {
                            class: "mor-pm-dev-link",
                            style: "margin: 0; align-self: flex-start; text-align: left;",
                            onclick: move |_| marketplace_url.set(DEFAULT_MARKETPLACE_URL.to_string()),
                            "Return to default settings"
                        }
                        div { class: "mor-pm-dev-actions",
                            button {
                                class: "mor-btn",
                                onclick: move |_| show_dev.set(false),
                                "Cancel"
                            }
                            button {
                                class: "mor-btn-primary",
                                onclick: move |_| {
                                    show_dev.set(false);
                                    let url = marketplace_url.read().clone();
                                    refreshing.set(true);
                                    install_status.set(None);
                                    spawn(async move {
                                        let (list, warn) = fetch_marketplace(&url).await;
                                        compendium_registry.set(list);
                                        if let Some(msg) = warn {
                                            install_status.set(Some(Err(msg)));
                                        } else {
                                            install_status.set(Some(Ok(
                                                "Marketplace registry refreshed.".into(),
                                            )));
                                        }
                                        refreshing.set(false);
                                    });
                                },
                                "Ok"
                            }
                        }
                    }
                }
            }
            }
        }
    };

    rsx! {
        crate::ui_kit::MorPanelWrapper {
            position: pos,
            default_position: DockPosition::mor_panel_left,
            floating_class: "floating-landscape",
            style { {crate::ui::layout::docks::shared::PANE_CSS} }
            script { dangerous_inner_html: crate::ui::layout::docks::shared::PANE_DRAG_JS }
            script { dangerous_inner_html: crate::ui::layout::docks::shared::PANE_RESIZE_JS }
            {inner_content}
        }
    }
}
