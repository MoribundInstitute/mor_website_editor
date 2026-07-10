use dioxus::prelude::*;
use mor_website_core::config::ThemeConfig;
use mor_website_core::utils::svg_icons::{is_svg, svg_to_data_uri};
use std::collections::HashMap;

/// The Website plug's export artifact: the finished standalone stylesheet.
/// (Replaces build_fresh_export_xml from the Blogger lineage.)
pub fn build_fresh_export_css(config: &ThemeConfig) -> String {
    mor_website_core::website::generate_theme_css(config)
}

pub fn handle_text_edit(target: &str, val: String, cfg: &str) -> Option<ThemeConfig> {
    if target.is_empty() {
        return None;
    }
    let mut config = toml::from_str::<ThemeConfig>(cfg).unwrap_or_default();
    if let Some(widget_id) = target
        .strip_prefix("widget.")
        .and_then(|s| s.strip_suffix(".title"))
    {
        config
            .template_pack
            .widget_titles
            .insert(widget_id.to_string(), val);
    } else {
        match target {
            "site.site_title" => config.site.site_title = val,
            "site.site_subtitle" => config.site.site_subtitle = val,
            "footer.footer_text" => config.footer.footer_text = val,
            "typography.body_font_stack" => config.typography.body_font_stack = val,
            "typography.heading_font_stack" => config.typography.heading_font_stack = val,
            "typography.mono_font_stack" => config.typography.mono_font_stack = val,
            _ => return None,
        }
    }
    Some(config)
}

pub fn handle_widget_move(id: &str, dest: &str, cfg: &str) -> Option<ThemeConfig> {
    if id.is_empty() || dest.is_empty() {
        return None;
    }
    let mut config = toml::from_str::<ThemeConfig>(cfg).unwrap_or_default();
    config.template_pack.move_widget(id, dest);
    Some(config)
}

pub fn handle_drop_svg(target: &str, content: &str, cfg: &str) -> Option<ThemeConfig> {
    if target.is_empty() || !is_svg(content) {
        return None;
    }
    let mask = svg_to_data_uri(content);
    let mut config = toml::from_str::<ThemeConfig>(cfg).unwrap_or_default();
    match target {
        "icons.panel_close" => config.icons.panel_close = mask,
        "icons.search" => config.icons.search = mask,
        "icons.menu" => config.icons.menu = mask,
        "icons.sidebar_left" => config.icons.sidebar_left = mask,
        "icons.sidebar_right" => config.icons.sidebar_right = mask,
        "icons.archive" => config.icons.archive = mask,
        "icons.label" => config.icons.label = mask,
        "icons.share" => config.icons.share = mask,
        "icons.user" => config.icons.user = mask,
        "icons.comment" => config.icons.comment = mask,
        "icons.arrow_up" => config.icons.arrow_up = mask,
        "icons.external_link" => config.icons.external_link = mask,
        _ => {}
    }
    Some(config)
}

pub fn persist_asset_editor(
    theme: crate::app::state::ThemeState,
    vfs: &HashMap<String, String>,
    ext: &str,
) {
    sync_vfs_to_disk(vfs, ext);
    theme.commit();
}

pub fn sync_vfs_to_disk(vfs: &HashMap<String, String>, ext: &str) {
    // When a website project is open, VFS keys that are project files write
    // straight back into the project folder; everything else takes the legacy
    // per-user override path. Saved project files refresh the preview.
    let site = dioxus::prelude::try_consume_context::<crate::app::state::WebsiteState>();
    let project = site.map(|s| s.project.peek().clone()).unwrap_or_default();
    let mut project_saved = false;

    for (filename, content) in vfs {
        let is_project_file = project.is_open()
            && (project.css_files.iter().any(|f| f == filename)
                || project.js_files.iter().any(|f| f == filename));
        if is_project_file {
            match mor_website_core::website::save_vfs_file(&project, filename, content) {
                Ok(path) => {
                    log::info!("Saved project file {}", path.display());
                    project_saved = true;
                }
                Err(e) => log::error!("Failed to save project file {}: {}", filename, e),
            }
            continue;
        }
        if ext == "css" {
            if filename == "preset_css.css" || !filename.ends_with(".css") {
                continue;
            }
            match mor_website_core::utils::fs_bridge::save_custom_css(filename, content) {
                Ok(path) => log::info!("Successfully synced {} to OS at {}", filename, path.display()),
                Err(e) => log::error!("Failed to sync {} to OS: {}", filename, e),
            }
        } else if ext == "js" {
            if filename == "custom_js.js" || !filename.ends_with(".js") {
                continue;
            }
            match mor_website_core::utils::fs_bridge::save_custom_js(filename, content) {
                Ok(path) => log::info!("Successfully synced {} to OS at {}", filename, path.display()),
                Err(e) => log::error!("Failed to sync {} to OS: {}", filename, e),
            }
        }
    }

    if project_saved {
        if let Some(site) = site {
            site.bump_preview();
        }
    }
}

