use crate::app::config_bridge::EditorPrefs;
use crate::app::state::{ThemeState, WebsiteState};
use dioxus::prelude::*;
use mor_website_core::config::ThemeConfig;
use std::collections::HashMap;

pub fn new_workspace(mut theme: ThemeState, mut original_toml: Signal<String>) {
    let fresh_prefs = EditorPrefs::load();
    let mut config = mor_website_core::config::defaults::default_theme_config();
    if let Some(pack) = fresh_prefs.default_template_pack {
        config.template_pack = pack;
    }
    theme.signals.apply_config(&config);
    original_toml.set(toml::to_string_pretty(&config).unwrap_or_default());
    theme.active_preset.set(None);
    theme.commit();
}

pub fn load_theme(theme: ThemeState, mut original_toml: Signal<String>) {
    if let Some(content) = crate::utils::io::load_toml() {
        if let Ok(new_config) = toml::from_str::<ThemeConfig>(&content) {
            theme.signals.apply_config(&new_config);
            original_toml.set(content);
        }
    }
}

pub fn save_theme(config_toml: &str, mut original_toml: Signal<String>) {
    crate::utils::io::save_toml(config_toml);
    original_toml.set(config_toml.to_string());
}

/// Pick a website folder, inventory it, seed the VFS with its css/js files,
/// start the preview server, and point the preview at the default page.
pub fn open_website_folder(site: WebsiteState, vfs: Signal<HashMap<String, String>>) {
    spawn(async move {
        let Some(folder) = rfd::AsyncFileDialog::new()
            .set_title("Open Website Folder")
            .pick_folder()
            .await
        else {
            return;
        };
        open_website_path(site, vfs, folder.path().to_path_buf()).await;
    });
}

/// Open a project folder by path — shared by the dialog above and the
/// `mor_website_dioxus_ui <folder>` command-line argument at startup.
pub async fn open_website_path(
    site: WebsiteState,
    mut vfs: Signal<HashMap<String, String>>,
    root: std::path::PathBuf,
) {
    {
        // Scanning + reading the folder is disk-bound — keep it off the UI thread.
        let scanned = tokio::task::spawn_blocking(move || {
            mor_website_core::website::scan_project(&root).map(|project| {
                let files = mor_website_core::website::load_project_vfs(&project);
                let server = crate::app::services::site_server::start_server(&project.root);
                (project, files, server)
            })
        })
        .await;
        let (project, files, server) = match scanned {
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

        vfs.write().extend(files);
        let mut site = site;
        site.current_page
            .set(project.default_page().map(str::to_string));
        site.project.set(project);
        site.server.set(server);
        site.bump_preview();
    }
}

/// Write the generated `mor-theme.css` into the open project's root.
pub fn export_theme_css(site: WebsiteState, config: &ThemeConfig) {
    let project = site.project.peek().clone();
    if !project.is_open() {
        log::warn!("Export mor-theme.css: no website folder open");
        return;
    }
    match mor_website_core::website::export_theme_css(&project, config) {
        Ok(path) => {
            log::info!("Wrote {}", path.display());
            site.bump_preview();
        }
        Err(e) => log::error!("Export mor-theme.css failed: {e}"),
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
