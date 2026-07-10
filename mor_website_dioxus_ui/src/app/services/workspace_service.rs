use dioxus::prelude::*;
use mor_website_core::config::ThemeConfig;
use mor_website_core::utils::svg_icons::{is_svg, svg_to_data_uri};
use std::collections::HashMap;

/// The Website plug's export artifact: the finished standalone stylesheet.
/// Website-native CSS/theme export helpers (replaces legacy XML export).
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

/// Webflow-style page content edit: replace `old_text` with `new_text` in the
/// opened page file when the old string appears **exactly once** (so we never
/// clobber a repeated phrase). Returns true if the file was written.
/// Project-relative path of the sidebar nav data source of truth.
pub const NAV_DATA_FILE: &str = "components/mor-nav-data.js";

/// Quote bare JS object keys so a MorNavData array can be parsed as JSON.
/// Does not touch keys already inside strings.
fn js_object_keys_to_json(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 64);
    let mut in_str = false;
    let mut esc = false;
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_str {
            out.push(c);
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        if c == '"' {
            in_str = true;
            out.push(c);
            i += 1;
            continue;
        }
        // Bare identifier key: start after { or ,
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            let mut j = i + 1;
            while j < bytes.len() {
                let d = bytes[j] as char;
                if d.is_ascii_alphanumeric() || d == '_' {
                    j += 1;
                } else {
                    break;
                }
            }
            // Skip whitespace then look for ':'
            let mut k = j;
            while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                k += 1;
            }
            if k < bytes.len() && bytes[k] == b':' {
                out.push('"');
                out.push_str(&input[start..j]);
                out.push('"');
                i = j;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Emit a JS-friendly object array (unquoted keys) from a JSON value tree.
fn json_to_js_object(value: &serde_json::Value, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let pad_in = "  ".repeat(indent + 1);
    match value {
        serde_json::Value::Null => "null".into(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
        }
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                return "[]".into();
            }
            let mut parts = Vec::new();
            for v in arr {
                parts.push(format!(
                    "{pad_in}{}",
                    json_to_js_object(v, indent + 1)
                ));
            }
            format!("[\n{}\n{pad}]", parts.join(",\n"))
        }
        serde_json::Value::Object(map) => {
            if map.is_empty() {
                return "{}".into();
            }
            // Prefer a stable field order for nav items.
            let order = ["section", "collapsible", "items", "icon", "label", "href", "active"];
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_by_key(|k| {
                order
                    .iter()
                    .position(|o| *o == k.as_str())
                    .unwrap_or(100)
            });
            let mut parts = Vec::new();
            for k in keys {
                let v = &map[k];
                parts.push(format!(
                    "{pad_in}{k}: {}",
                    json_to_js_object(v, indent + 1)
                ));
            }
            format!("{{\n{}\n{pad}}}", parts.join(",\n"))
        }
    }
}

/// Update one item in `components/mor-nav-data.js` (MorNavData array).
/// `href` and/or `label` when `Some` overwrite that field.
pub fn handle_nav_link_edit(
    project: &mor_website_core::website::WebsiteProject,
    group: usize,
    item: usize,
    href: Option<&str>,
    label: Option<&str>,
) -> Result<(), String> {
    if !project.is_open() {
        return Err("No website folder open".into());
    }
    if href.is_none() && label.is_none() {
        return Ok(());
    }
    let path = project.root.join(NAV_DATA_FILE);
    let src = std::fs::read_to_string(&path)
        .map_err(|e| format!("Read {}: {e}", path.display()))?;

    // Extract the array after `window.MorNavData =`.
    let Some(eq) = src.find('=') else {
        return Err(format!("{NAV_DATA_FILE}: expected window.MorNavData = […]"));
    };
    let after = src[eq + 1..].trim_start();
    let Some(start_rel) = after.find('[') else {
        return Err(format!("{NAV_DATA_FILE}: no array found"));
    };
    let abs_start = eq + 1 + (src[eq + 1..].len() - after.len()) + start_rel;
    let bytes = src.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    let mut abs_end = None;
    for (i, &b) in bytes.iter().enumerate().skip(abs_start) {
        let c = b as char;
        if in_str {
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    abs_end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(abs_end) = abs_end else {
        return Err(format!("{NAV_DATA_FILE}: unclosed array"));
    };
    let js_slice = &src[abs_start..=abs_end];
    let json_slice = js_object_keys_to_json(js_slice);
    let mut data: serde_json::Value = serde_json::from_str(&json_slice).map_err(|e| {
        format!("{NAV_DATA_FILE} could not be parsed as data: {e}")
    })?;
    let groups = data
        .as_array_mut()
        .ok_or_else(|| format!("{NAV_DATA_FILE}: root is not an array"))?;
    let group_obj = groups
        .get_mut(group)
        .ok_or_else(|| format!("Nav group {group} out of range"))?;
    let items = group_obj
        .get_mut("items")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| format!("Nav group {group} has no items array"))?;
    let entry = items
        .get_mut(item)
        .ok_or_else(|| format!("Nav item {group}.{item} out of range"))?;
    let obj = entry
        .as_object_mut()
        .ok_or("nav item is not an object")?;
    if let Some(h) = href {
        // Skip no-op writes.
        if obj.get("href").and_then(|v| v.as_str()) == Some(h) && label.is_none() {
            return Ok(());
        }
        obj.insert("href".into(), serde_json::Value::String(h.to_string()));
    }
    if let Some(l) = label {
        if obj.get("label").and_then(|v| v.as_str()) == Some(l) && href.is_none() {
            return Ok(());
        }
        obj.insert("label".into(), serde_json::Value::String(l.to_string()));
    }
    let pretty = json_to_js_object(&data, 0);
    let header = &src[..abs_start];
    let mut tail = src[abs_end + 1..].trim_start();
    if let Some(rest) = tail.strip_prefix(';') {
        tail = rest.trim_start();
    }
    let out = if tail.is_empty() {
        format!("{header}{pretty};\n")
    } else {
        format!("{header}{pretty};\n\n{tail}")
    };
    std::fs::write(&path, out).map_err(|e| format!("Write {}: {e}", path.display()))?;
    log::info!("Nav link updated in {}", path.display());
    Ok(())
}

pub fn handle_page_text_edit(
    project: &mor_website_core::website::WebsiteProject,
    page_rel: &str,
    old_text: &str,
    new_text: &str,
) -> Result<bool, String> {
    if !project.is_open() {
        return Err("No website folder open".into());
    }
    let rich = old_text.contains('<') || new_text.contains('<');
    let (old_text, new_text) = if rich {
        (old_text.to_string(), new_text.to_string())
    } else {
        (old_text.trim().to_string(), new_text.trim().to_string())
    };
    if old_text.is_empty() || old_text == new_text {
        return Ok(false);
    }
    let max_old = if rich { 24_000 } else { 4_000 };
    let max_new = if rich { 32_000 } else { 8_000 };
    if old_text.len() > max_old || new_text.len() > max_new {
        return Err("Edit too large to auto-save safely".into());
    }
    let path = project.root.join(page_rel);
    let src = std::fs::read_to_string(&path).map_err(|e| format!("Read {}: {e}", path.display()))?;

    // Ambiguous exact multi-match: refuse.
    let exact = src.matches(&old_text).count();
    if exact > 1 {
        return Err(format!(
            "That text appears {exact} times in the file — edit it in the Code view to be safe"
        ));
    }

    match mor_website_core::website::page_edit::apply_page_edit(&src, &old_text, &new_text) {
        Some(res) => {
            std::fs::write(&path, &res.updated)
                .map_err(|e| format!("Write {}: {e}", path.display()))?;
            log::info!(
                "Page edit saved ({}) → {}",
                res.method,
                path.display()
            );
            Ok(true)
        }
        None => Err(
            "Could not find that text/HTML in the page source (PHP-generated or ambiguous). Open Code view to edit safely.".into(),
        ),
    }
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

