use crate::app::state::{ThemeState, WebsiteState};
use dioxus::prelude::*;
use mor_website_core::config::ThemeConfig;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Load a ThemeConfig TOML from a user-picked file (advanced / import).
pub fn load_theme_config(theme: ThemeState, mut original_toml: Signal<String>) {
    if let Some(content) = crate::utils::io::load_toml() {
        if let Ok(new_config) = toml::from_str::<ThemeConfig>(&content) {
            theme.signals.apply_config(&new_config);
            original_toml.set(content);
        }
    }
}

/// Save ThemeConfig TOML to a user-picked path (backup / share outside the site).
pub fn save_theme_config_as(config_toml: &str, mut original_toml: Signal<String>) {
    crate::utils::io::save_toml(config_toml);
    original_toml.set(config_toml.to_string());
}

/// Primary website save: write `workspace.toml` + `mor-theme.css` (and optional
/// `mor-theme.js`) into the open project root.
///
/// When no folder is open, falls back to **Save Theme Config As…** so the user
/// still has a way to persist theme tokens.
///
/// Returns a short status line for the workbench status bar.
pub fn save_theme_to_site(
    site: WebsiteState,
    config: &ThemeConfig,
    config_toml: &str,
    mut original_toml: Signal<String>,
) -> String {
    let project = site.project.peek().clone();
    if !project.is_open() {
        crate::utils::io::save_toml(config_toml);
        original_toml.set(config_toml.to_string());
        return "Saved theme config. Open a website folder to write mor-theme.css into the site."
            .into();
    }

    let toml_path = project.root.join("workspace.toml");
    if let Err(e) = std::fs::write(&toml_path, config_toml) {
        return format!("Failed to write workspace.toml: {e}");
    }

    match mor_website_core::website::export_theme_bundle(&project, config) {
        Ok(written) => {
            original_toml.set(config_toml.to_string());
            site.bump_preview();
            let files = std::iter::once("workspace.toml".to_string())
                .chain(written)
                .collect::<Vec<_>>()
                .join(", ");
            format!("Saved to site: {files}")
        }
        Err(e) => {
            // Theme tokens are on disk even if CSS export failed.
            original_toml.set(config_toml.to_string());
            format!("Wrote workspace.toml but theme CSS failed: {e}")
        }
    }
}

/// Pick a website folder, inventory it, seed the VFS with its css/js files,
/// start the preview server, and point the preview at the default page.
///
/// If the folder contains `workspace.toml` (or legacy `theme_config.toml`),
/// loads it into the editor so theme tokens match the site on disk.
pub fn open_website_folder(
    site: WebsiteState,
    vfs: Signal<HashMap<String, String>>,
    theme: ThemeState,
    original_toml: Signal<String>,
) {
    spawn(async move {
        let Some(folder) = rfd::AsyncFileDialog::new()
            .set_title("Open Website Folder")
            .pick_folder()
            .await
        else {
            return;
        };
        open_website_path(
            site,
            vfs,
            folder.path().to_path_buf(),
            theme,
            original_toml,
        )
        .await;
    });
}

/// Open a project folder by path — shared by the dialog above and the
/// `mor_website_dioxus_ui <folder>` command-line argument at startup.
pub async fn open_website_path(
    site: WebsiteState,
    mut vfs: Signal<HashMap<String, String>>,
    root: PathBuf,
    theme: ThemeState,
    mut original_toml: Signal<String>,
) {
    {
        // Scanning + reading the folder is disk-bound — keep it off the UI thread.
        let scanned = tokio::task::spawn_blocking(move || {
            mor_website_core::website::scan_project(&root).map(|project| {
                let files = mor_website_core::website::load_project_vfs(&project);
                let workspace = load_workspace_toml_from_root(&project.root);
                let server = crate::app::services::site_server::start_server(&project.root);
                (project, files, workspace, server)
            })
        })
        .await;
        let (project, files, workspace, server) = match scanned {
            Ok(Ok(bundle)) => bundle,
            Ok(Err(e)) => {
                log::error!("Failed to scan website folder: {e}");
                return;
            }
            Err(e) => {
                log::error!("Folder scan task failed: {e}");
                return;
            }
        };
        let server = match server {
            Ok(info) => Some(info),
            Err(e) => {
                log::error!("Preview server failed to start: {e}");
                None
            }
        };

        if let Some((raw, config)) = workspace {
            theme.signals.apply_config(&config);
            original_toml.set(raw);
            log::info!("Loaded workspace.toml from {}", project.root.display());
        }

        vfs.write().extend(files);
        let mut site = site;
        site.current_page
            .set(project.default_page().map(str::to_string));
        // Pin project path for Robot Assist agents.
        {
            let mut pol = mor_website_core::utils::robot_assist::load_policy();
            if pol.enabled {
                pol.project_path = Some(project.root.display().to_string());
                let _ = mor_website_core::utils::robot_assist::save_policy(pol);
            }
        }
        site.project.set(project);
        site.server.set(server);
        site.bump_preview();
    }
}

/// Read ThemeConfig from the project root if present.
/// Prefers `workspace.toml`, then legacy `theme_config.toml`.
fn load_workspace_toml_from_root(root: &Path) -> Option<(String, ThemeConfig)> {
    for name in ["workspace.toml", "theme_config.toml"] {
        let path = root.join(name);
        if !path.is_file() {
            continue;
        }
        let raw = std::fs::read_to_string(&path).ok()?;
        if let Ok(config) = toml::from_str::<ThemeConfig>(&raw) {
            return Some((raw, config));
        }
        log::warn!("Ignoring unreadable {name} in {}", root.display());
    }
    None
}

/// Write only the generated stylesheet into the open project (Advanced /
/// export panel). Prefer [`save_theme_to_site`] for normal editing.
pub fn export_theme_css(site: WebsiteState, config: &ThemeConfig) -> String {
    let project = site.project.peek().clone();
    if !project.is_open() {
        return "Open a website folder first.".into();
    }
    match mor_website_core::website::export_theme_bundle(&project, config) {
        Ok(written) => {
            log::info!("Wrote {}", written.join(", "));
            site.bump_preview();
            format!("Wrote {}", written.join(", "))
        }
        Err(e) => {
            log::error!("Export theme CSS failed: {e}");
            format!("Export failed: {e}")
        }
    }
}

/// Zip the open project (with a fresh mor-theme.css) to a user-picked path.
pub fn export_site_zip(site: WebsiteState, config: ThemeConfig) {
    let project = site.project.peek().clone();
    if !project.is_open() {
        log::warn!("Export site zip: no website folder open");
        return;
    }
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
            mor_website_core::website::zip_site(&project, &config, &dest).map(|_| dest)
        })
        .await;
        match result {
            Ok(Ok(dest)) => log::info!("Exported site zip → {}", dest.display()),
            Ok(Err(e)) => log::error!("Site zip failed: {e}"),
            Err(e) => log::error!("Site zip task failed: {e}"),
        }
    });
}
