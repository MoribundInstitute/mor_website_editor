//! Application root.
//!
//! The app module is split into focused submodules under `src/app/`.
//! `src/lib.rs` declares `pub mod app;`, so this file is the module root
//! when the old flat `src/app.rs` file is removed.

use dioxus::prelude::*;

pub mod config_bridge;
pub mod edit_context;
pub mod hotswap;
pub mod keyboard;
pub mod plugin_registry;
mod restore_drop;
pub mod robot_session;
pub mod services;
pub mod shell;
pub mod shell_dialogs;
pub mod shell_file_actions;
pub mod shell_layout;
pub mod state;
mod theme_hot_reload;
pub mod theme_signals;
pub mod vfs;

use crate::app::vfs::VfsDictionary;
use keyboard::use_keyboard_shortcuts;
use restore_drop::use_restore_drop_bridge;
use robot_session::{use_project_hot_reload, use_robot_session_bridge};
use shell::render_app_shell;
use state::{LayoutState, RenderState, SiteData, ThemeState, WebsiteState};
use theme_hot_reload::use_theme_config_hot_reload;

#[allow(non_snake_case)]
pub fn App() -> Element {
    // VFS must be provided before any hook that calls use_context::<VfsDictionary>()
    let vfs_map = use_signal(|| {
        let mut map = mor_website_core::utils::fs_bridge::load_all_custom_css().unwrap_or_default();
        if let Ok(js_map) = mor_website_core::utils::fs_bridge::load_all_custom_js() {
            map.extend(js_map);
        }
        map
    });
    provide_context(VfsDictionary(vfs_map));

    let theme = ThemeState::new();
    let layout = LayoutState::new();
    // Sample post list for offline/demo preview panels.
    let site_data = use_signal(|| SiteData::default());
    let website = WebsiteState::new();
    let render = RenderState::new(theme, website);

    // Last-saved ThemeConfig TOML (dirty = live config differs). Shared so
    // File → Open / CLI folder open can seed it from workspace.toml on disk.
    let original_toml =
        use_signal(|| toml::to_string_pretty(&(render.current_config)()).unwrap_or_default());

    provide_context(theme);
    provide_context(layout);
    provide_context(site_data);
    provide_context(website);
    provide_context(render);
    provide_context(original_toml);

    // `mor_website_dioxus_ui <folder>`: auto-open a project passed on the
    // command line (also how scripted/CI launches point the editor at a site).
    use_hook(move || {
        if let Some(arg) = std::env::args().nth(1) {
            let path = std::path::PathBuf::from(&arg);
            if path.is_dir() {
                spawn(async move {
                    shell_file_actions::open_website_path(
                        website,
                        vfs_map,
                        path,
                        theme,
                        original_toml,
                    )
                    .await;
                });
            } else {
                log::warn!("Ignoring CLI argument (not a folder): {arg}");
            }
        }
    });

    // Refetch the previewed page whenever the server, page, or nonce changes.
    // The fetch is blocking std I/O, so it runs off the UI thread.
    use_effect(move || {
        let Some(info) = (website.server)() else { return };
        let Some(page) = (website.current_page)() else { return };
        let _ = (website.preview_nonce)(); // subscribe: bumps force a refetch
        let mut raw = website.raw_page_html;
        spawn(async move {
            let fetched = tokio::task::spawn_blocking(move || {
                // A freshly spawned php server may not be listening yet — retry briefly.
                let mut last = services::site_server::fetch_page(info.port, &page);
                for _ in 0..4 {
                    if last.is_ok() {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    last = services::site_server::fetch_page(info.port, &page);
                }
                last
            })
            .await;
            match fetched {
                Ok(Ok(html)) => raw.set(html),
                Ok(Err(e)) => log::error!("Preview fetch failed: {e}"),
                Err(e) => log::error!("Preview fetch task failed: {e}"),
            }
        });
    });

    // Customized keybinds, shared by the dispatch layers (keyboard.rs JS map,
    // shortcut registry), the menu-bar chips, and the rebind dialog.
    let shortcut_prefs =
        provide_context(Signal::new(config_bridge::ShortcutPrefs::load()));

    use_keyboard_shortcuts(layout, shortcut_prefs);
    use_restore_drop_bridge(theme);
    use_theme_config_hot_reload(theme);
    use_robot_session_bridge(theme, website);
    use_project_hot_reload(theme, website);

    render_app_shell(theme, layout, render)
}
