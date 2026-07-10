use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginState {
    pub id: String,
    pub enabled: bool,
    #[serde(default = "default_plugin_version")]
    pub version: String,
}

fn default_plugin_version() -> String {
    "1.0.0".to_string()
}

/// Blueprint for a plugin/compendium entry (GitHub or local payload).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompendiumManifest {
    pub id: String,
    pub display_name: String,
    pub version: String,
    pub description: String,
    pub payload_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct CustomEditorColors {
    pub bg: Option<String>,
    pub panel: Option<String>,
    pub header: Option<String>,
    pub text: Option<String>,
    pub text_muted: Option<String>,
    pub border: Option<String>,
    pub border_light: Option<String>,
    pub accent: Option<String>,
    pub accent_hover: Option<String>,
    pub btn: Option<String>,
    pub btn_hover: Option<String>,
    pub font_family: Option<String>,
    pub font_size_base: Option<String>,
    pub font_size_h1: Option<String>,
    pub padding_base: Option<String>,
    pub border_radius: Option<String>,
    pub destructive: Option<String>,
    pub success: Option<String>,
    pub warning: Option<String>,
    #[serde(default)]
    pub panel_title_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutPrefs {
    pub undo: Option<String>,
    pub redo: Option<String>,
    /// Copy generated mor-theme.css. Alias keeps old prefs files loading.
    #[serde(alias = "copy_raw_xml")]
    pub copy_theme_css: Option<String>,
    pub toggle_left_dock: Option<String>,
    pub toggle_right_dock: Option<String>,
    #[serde(default)]
    pub close_left_dock: Option<String>,
    #[serde(default)]
    pub close_right_dock: Option<String>,
    #[serde(default)]
    pub user_prefs: Option<String>,
    #[serde(default)]
    pub theme_diagnostics: Option<String>,
    #[serde(default)]
    pub toggle_preview: Option<String>,
    #[serde(default, alias = "exit_architect")]
    pub exit_editor: Option<String>,
    #[serde(default)]
    pub open_project: Option<String>,
    #[serde(default)]
    pub save_project: Option<String>,
    /// Save theme config TOML to a chosen path. Alias: legacy `export_xml`.
    #[serde(default, alias = "export_xml")]
    pub save_theme_config_as: Option<String>,
    #[serde(default)]
    pub reset_zoom: Option<String>,
    /// Force-refetch the preview page from disk and fully rewrite the iframe
    /// (clears morph cache + cache-busts local assets). Default: Ctrl+Shift+R.
    #[serde(default = "default_hard_refresh_preview")]
    pub hard_refresh_preview: Option<String>,
}

fn default_hard_refresh_preview() -> Option<String> {
    Some("Ctrl+Shift+R".to_string())
}

impl Default for ShortcutPrefs {
    fn default() -> Self {
        Self {
            undo: Some("Ctrl+Z".to_string()),
            redo: Some("Ctrl+Y".to_string()),
            // Shift-modified: bare Ctrl+C must stay the system clipboard copy.
            copy_theme_css: Some("Ctrl+Shift+C".to_string()),
            toggle_left_dock: Some("Ctrl+B".to_string()),
            toggle_right_dock: Some("Ctrl+E".to_string()),
            close_left_dock: Some("Ctrl+Shift+Left".to_string()),
            close_right_dock: Some("Ctrl+Shift+Right".to_string()),
            user_prefs: Some("Ctrl+P".to_string()),
            theme_diagnostics: Some("Ctrl+D".to_string()),
            toggle_preview: Some("F9".to_string()),
            exit_editor: Some("Ctrl+Q".to_string()),
            open_project: Some("Ctrl+O".to_string()),
            save_project: Some("Ctrl+S".to_string()),
            save_theme_config_as: Some("Shift+Ctrl+E".to_string()),
            reset_zoom: Some("Ctrl+0".to_string()),
            hard_refresh_preview: Some("Ctrl+Shift+R".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayoutPrefs {
    /// Dock IDs pinned to the activity bar, in display order.
    #[serde(default)]
    pub pinned_docks: Vec<String>,
    /// Dock IDs hidden from the activity / quick-launch bar (e.g. "js_editor").
    #[serde(default)]
    pub quick_launch_hidden: Vec<String>,
    /// Per-dock activity-bar icon overrides, keyed by dock ID. Value is a tagged
    /// string: "emoji:🎨", "svg:palette", or "raw:<svg…>". Absent = built-in default.
    #[serde(default)]
    pub activity_icons: HashMap<String, String>,
    /// Designer mode: golden-path docks only (theme, pages, presets). Default true
    /// when unset so first-run is approachable; Advanced mode unhides code docks.
    #[serde(default)]
    pub designer_mode: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EditorPrefs {
    #[serde(default)]
    pub plugins: Vec<PluginState>,
    #[serde(default)]
    pub ui_mode: Option<String>,
    #[serde(default)]
    pub workspace_theme: Option<String>,
    #[serde(default)]
    pub workspace_theme_preset: Option<String>,
    #[serde(default)]
    pub custom_editor_colors: CustomEditorColors,
    #[serde(default)]
    pub default_template_pack: Option<mor_website_core::config::TemplatePackConfig>,
    #[serde(default)]
    pub pinned_docks: Vec<String>,
    /// VS Code-style code minimap: global default for editors with no per-workspace
    /// override. Unset = on.
    #[serde(default)]
    pub show_minimap: Option<bool>,
    /// Per-workspace minimap overrides, keyed by editor/workspace id. Takes
    /// precedence over `show_minimap` for that editor.
    #[serde(default)]
    pub minimap_overrides: HashMap<String, bool>,
    /// Per-workspace word-wrap overrides, same keying as `minimap_overrides`.
    #[serde(default)]
    pub wrap_overrides: HashMap<String, bool>,
}
