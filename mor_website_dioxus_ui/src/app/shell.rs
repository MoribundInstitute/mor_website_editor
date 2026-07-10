use dioxus::prelude::*;

use super::state::{
    CenterView, DockPosition, LayoutState, PluginManagerContext, RenderState, ThemeState,
};
use mor_website_core::diagnostics::Severity;
use crate::app::config_bridge::EditorPrefs;
use crate::ui::layout::menu_bar::AppMenuBar;
use crate::ui::layout::theme::MorStyleProvider;
use crate::ui::layout::window_frame::{MorHeaderBar, MorShell, MorWindowTitle};

use super::shell_dialogs::MorDialogs;
use super::shell_file_actions;
use super::shell_layout::{DockLocalSignals, FloatingWindowManager, MorLayoutChrome};

const EDITOR_UI_CSS: &str = include_str!("../../assets/css/editor_ui.css");
use crate::ui::components::code_editor::CM6_BUNDLE_JS;

#[derive(Clone, Copy)]
pub struct WorkbenchEditState {
    pub edited_xml: Signal<String>,
    pub workbench_status: Signal<String>,
    /// Set by the Widgets dock to ask the workbench to open a blueprint for editing.
    /// The workbench consumes it (loads the buffer) and resets it to None.
    pub edit_widget_request:
        Signal<Option<mor_website_core::utils::fs_bridge::WidgetBlueprint>>,
    /// Set by the Widgets dock to insert a blueprint into the active module. The
    /// workbench consumes it and resets to None.
    pub add_widget_request:
        Signal<Option<mor_website_core::utils::fs_bridge::WidgetBlueprint>>,
    /// Where an added gadget lands: Some(socket_key) → that slot; None → appended
    /// to the active module buffer. Set by the workbench's "+ Add" buttons.
    pub add_target: Signal<Option<String>>,
    /// Set by the Template Modules dock's file picker or a file dropped onto the
    /// workspace: the XML to load into the active slot's editor buffer (the sender
    /// also sets `active_workbench_module`). Non-destructive — edits the buffer
    /// until the user hits Save. The workbench consumes it and resets to None.
    pub load_module_request: Signal<Option<String>>,
}

/// VS Code-style bottom status strip, three zones:
/// left = health (diagnostics count + unsaved dot), center = transient workbench
/// messages (`WorkbenchEditState::workbench_status`, otherwise unrendered), right =
/// active-workspace context, generated `mor-theme.css` size, active preset.
#[component]
fn StatusBar(config_toml_signal: Memo<String>, original_toml: Signal<String>) -> Element {
    let mut layout = use_context::<LayoutState>();
    let render = use_context::<RenderState>();
    let theme = use_context::<ThemeState>();
    let edit_state = use_context::<WorkbenchEditState>();
    let website = use_context::<crate::app::state::WebsiteState>();

    let result = render.diag.read();
    let error_count = result.errors.len();
    let warning_count = result
        .warnings
        .iter()
        .filter(|w| w.severity == Severity::Warning)
        .count();
    let clean = error_count == 0 && warning_count == 0;
    // Surface the first problem in the tooltip so the count is actionable on hover.
    let diag_title = result
        .errors
        .first()
        .cloned()
        .or_else(|| {
            result
                .warnings
                .iter()
                .find(|w| w.severity == Severity::Warning)
                .map(|w| w.message.clone())
        })
        .map(|m| format!("{m} — click for all diagnostics"))
        .unwrap_or_else(|| "Site Diagnostics — click to open panel".to_string());

    let dirty = config_toml_signal() != original_toml();
    let message = (edit_state.workbench_status)();

    // What the active workspace is looking at.
    let context = match (layout.center_view)() {
        CenterView::Preview | CenterView::Split => format!(
            "{} \u{00b7} {}px \u{00b7} {}",
            (layout.preview_viewport)().label(),
            (layout.preview_width)(),
            (layout.preview_template_mode)().label(),
        ),
        CenterView::CodeEditor => {
            let page = (website.current_page)()
                .or_else(|| website.project.read().default_page().map(str::to_string));
            if (layout.code_show_compiled)() {
                "compiled CSS".to_string()
            } else if let Some(p) = page {
                format!("code · {p}")
            } else {
                "code editor".to_string()
            }
        },
        CenterView::ModuleWorkbench => match (layout.active_workbench_module)() {
            Some(key) => {
                let name =
                    crate::ui::layout::docks::template_editor_dock::slot_display_name(key);
                if mor_website_core::render::template_resolver::module_override(key).is_some() {
                    format!("{name} \u{00b7} customized")
                } else {
                    name.to_string()
                }
            }
            None => "no module selected".to_string(),
        },
        CenterView::StaticPageEditor => {
            (layout.active_static_page)().unwrap_or_else(|| "no page selected".to_string())
        }
        CenterView::PageMap => {
            let page = (website.current_page)()
                .or_else(|| website.project.read().default_page().map(str::to_string))
                .unwrap_or_else(|| "no page".into());
            format!("page map · {page}")
        }
        _ => String::new(),
    };

    // Size of the generated mor-theme.css (no platform cap for plain websites).
    let css_kb = render.generated_css.read().len() as f64 / 1024.0;

    let preset_label = (theme.active_preset)()
        .map(|id| {
            mor_website_core::presets::all_presets()
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.name)
                .unwrap_or(id)
        })
        .unwrap_or("no preset");

    rsx! {
        div {
            class: "editor-statusbar",
            style: "flex-shrink: 0; display: flex; align-items: center; gap: 8px; height: 24px; padding: 0 8px; font-size: 0.72rem; background: var(--mor-header, #10161f); border-top: 1px solid var(--mor-border, #2a2a2a); color: var(--editor-text, #ddd); user-select: none; white-space: nowrap;",
            button {
                class: "statusbar-diagnostics",
                title: "{diag_title}",
                style: "display: flex; align-items: center; gap: 12px; background: transparent; border: none; color: inherit; font: inherit; cursor: pointer; padding: 2px 6px; border-radius: 3px;",
                onclick: move |_| {
                    if (layout.diagnostics_pos)() == DockPosition::Hidden {
                        layout.request_dock("diagnostics", DockPosition::mor_panel_left);
                    } else {
                        layout.diagnostics_pos.set(DockPosition::Hidden);
                    }
                },
                if clean {
                    span { style: "color: #73c991;", "\u{2713} no problems" }
                } else {
                    span { style: "color: #ea8285; font-weight: bold;", "\u{2298} {error_count}" }
                    span { style: "color: #d29922; font-weight: bold;", "\u{25B3} {warning_count}" }
                }
            }
            if dirty {
                span {
                    title: "Theme has unsaved changes (File \u{2192} Save Theme to Site)",
                    style: "color: #d29922;",
                    "\u{25cf} unsaved"
                }
            }
            {
                let tier = (theme.robot_tier)();
                let robots_on = *theme.enable_ai_bridge.read() && tier != "off";
                let robots_label = if robots_on {
                    format!("robots: {tier}")
                } else {
                    "robots: off".into()
                };
                let robots_color = if robots_on { "#73c991" } else { "#888" };
                let robots_title = if robots_on {
                    "Robot Assist is on — external MCP agents may write to the open project (Preferences to change tier)"
                } else {
                    "Robot Assist is off — open Preferences to grant agents site-building power"
                };
                rsx! {
                    span {
                        title: "{robots_title}",
                        style: "color: {robots_color}; cursor: default;",
                        "{robots_label}"
                    }
                }
            }

            // Center: transient workbench/dock messages; click to dismiss.
            div {
                style: "flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; text-align: center;",
                if !message.is_empty() {
                    button {
                        title: "Click to dismiss",
                        style: "background: transparent; border: none; color: inherit; font: inherit; cursor: pointer; opacity: 0.85; max-width: 100%; overflow: hidden; text-overflow: ellipsis;",
                        onclick: move |_| {
                            let mut status = edit_state.workbench_status;
                            status.set(String::new());
                        },
                        "{message}"
                    }
                }
            }

            if !context.is_empty() {
                span { style: "opacity: 0.8;", "{context}" }
            }
            span {
                title: "Generated mor-theme.css size",
                style: "opacity: 0.9;",
                "theme.css {css_kb:.1} KB"
            }
            button {
                title: "Active preset \u{2014} click to open Presets",
                style: "background: transparent; border: none; color: inherit; font: inherit; cursor: pointer; padding: 2px 6px; border-radius: 3px; opacity: 0.9;",
                onclick: move |_| {
                    if (layout.presets_pos)() == DockPosition::Hidden {
                        layout.request_dock("presets", DockPosition::mor_panel_left);
                    } else {
                        layout.presets_pos.set(DockPosition::Hidden);
                    }
                },
                "\u{25c6} {preset_label}"
            }
        }
    }
}

pub fn render_app_shell(
    theme: ThemeState,
    mut layout: LayoutState,
    render: RenderState,
) -> Element {
    let _signals = theme.signals;
    let current_config = render.current_config;
    let active_preset = theme.active_preset;
    let mut center_view = layout.center_view;
    let website = use_context::<crate::app::state::WebsiteState>();
    let vfs = use_context::<crate::app::vfs::VfsDictionary>().0;

    provide_context(render);

    let show_preview = use_signal(|| true);
    let show_undocked_pages = use_signal(|| false);

    let show_about = use_signal(|| false);
    let show_prefs = use_signal(|| false);
    let show_editor_settings = use_signal(|| false);
    let show_shortcuts = use_signal(|| false);
    let show_docs = use_signal(|| false);
    let show_ssh_publish = use_signal(|| false);
    let show_new_website = use_signal(|| false);

    let prefs = use_signal(|| EditorPrefs::load());

    // Editor minimap: global default (Editor Settings → General) plus per-workspace
    // overrides, both seeded from prefs and provided as context so toggles apply live.
    let minimap_setting = use_signal(|| prefs().show_minimap.unwrap_or(true));
    provide_context(crate::ui::components::code_editor::MinimapSetting(minimap_setting));
    let minimap_overrides = use_signal(|| prefs().minimap_overrides.clone());
    provide_context(crate::ui::components::code_editor::MinimapOverrides(minimap_overrides));
    let launch_plugins = use_signal(|| prefs().plugins.clone());
    let current_plugins = use_signal(|| prefs().plugins.clone());
    // Seed immediately so Plugin Manager never opens empty while waiting on network.
    let mut compendium_registry =
        use_signal(crate::app::plugin_registry::fallback_compendium);

    provide_context(PluginManagerContext {
        launch_plugins,
        current_plugins,
        compendium_registry,
    });

    use_effect(|| {
        if let Err(e) = mor_website_core::utils::fs_bridge::init_template_dirs() {
            log::warn!("[startup] Template dir init failed: {}", e);
        }
    });

    use_effect(move || {
        let plugins = current_plugins();
        let mut p = EditorPrefs::load();
        if p.plugins != plugins {
            p.plugins = plugins;
            let _ = p.save();
        }
    });

    use_effect(move || {
        spawn(async move {
            let (list, warn) = crate::app::plugin_registry::fetch_marketplace(
                crate::app::plugin_registry::DEFAULT_MARKETPLACE_URL,
            )
            .await;
            if let Some(msg) = warn {
                log::warn!("[plugin marketplace] {msg}");
            }
            compendium_registry.set(list);
        });
    });

    let active_ui_mode =
        std::env::var("MOR_ACTIVE_UI_MODE").unwrap_or_else(|_| "frameless".to_string());
    let ui_mode_pref = use_signal(|| {
        prefs()
            .ui_mode
            .clone()
            .unwrap_or_else(|| active_ui_mode.clone())
    });
    let ui_theme_pref = use_signal(|| {
        prefs()
            .workspace_theme
            .clone()
            .unwrap_or_else(|| crate::ui::layout::theme::MOR_STUDIO_TOML.to_string())
    });
    let show_window_buttons = active_ui_mode == "frameless";
    let show_custom_title = active_ui_mode != "native";

    // Provided from app root so Open Folder / CLI open can seed from workspace.toml.
    let original_toml = use_context::<Signal<String>>();
    let config_toml_signal = use_memo(move || {
        let updated = current_config();
        mor_website_core::config::update_toml_preserve_comments(&original_toml(), &updated)
    });
    let mut tv_monitor = use_signal(|| String::new());

    use_effect(move || {
        tv_monitor.set((render.preview_html)());
    });

    let edited_xml = use_signal(String::new);
    let workbench_status = use_signal(String::new);

    provide_context(WorkbenchEditState {
        edited_xml,
        workbench_status,
        edit_widget_request: use_signal(|| None),
        add_widget_request: use_signal(|| None),
        add_target: use_signal(|| None),
        load_module_request: use_signal(|| None),
    });

    provide_context(DockLocalSignals {
        show_preview,
        show_undocked_pages,
        tv_monitor,
    });

    rsx! {
        script { dangerous_inner_html: "document.addEventListener('contextmenu', event => event.preventDefault());" }
        script { dangerous_inner_html: "{CM6_BUNDLE_JS}" }
        // Theme-aware JS completions (read lazily by the CM6 bundle's js mode).
        script { dangerous_inner_html: "window.MOR_JS_HINTS = {mor_website_core::render::js_behaviors::editor_hints_json()};" }
        MorStyleProvider { theme_toml: ui_theme_pref() }
        style { "{EDITOR_UI_CSS}" }

        MorShell {
            if active_ui_mode != "native" {
                MorHeaderBar {
                    show_controls: show_window_buttons,
                    start: rsx! { div { style: "width: 16px;" } },
                    center: rsx! {
                        if show_custom_title {
                            MorWindowTitle {
                                title: "MorWebsite Editor".to_string(),
                                subtitle: Some(format!("{} Mode", active_ui_mode))
                            }
                        }
                    },
                    end: rsx! { div { style: "width: 16px;" } }
                }
            }

            MorDialogs {
                show_about,
                show_prefs,
                show_editor_settings,
                show_shortcuts,
                show_docs,
                show_ssh_publish,
                show_new_website,
                ui_mode_pref,
                ui_theme_pref,
                active_ui_mode: active_ui_mode.clone(),
            }

            div { class: "editor-shell", style: "height: 100%;",
                AppMenuBar {
                    show_prefs,
                    show_editor_settings,
                    show_about,
                    show_shortcuts,
                    show_docs,
                    show_ssh_publish,
                    show_new_website,

                    on_open_folder: move |_| {
                        shell_file_actions::open_website_folder(
                            website,
                            vfs,
                            theme,
                            original_toml,
                        );
                    },
                    on_save_to_site: move |_| {
                        let msg = shell_file_actions::save_theme_to_site(
                            website,
                            &current_config(),
                            &config_toml_signal(),
                            original_toml,
                        );
                        let mut status = workbench_status;
                        status.set(msg);
                    },
                    on_load_theme_config: move |_| {
                        shell_file_actions::load_theme_config(theme, original_toml);
                    },
                    on_save_theme_config_as: move |_| {
                        shell_file_actions::save_theme_config_as(
                            &config_toml_signal(),
                            original_toml,
                        );
                        let mut status = workbench_status;
                        status.set("Theme config saved to chosen path.".into());
                    },
                    on_export_zip: move |_| {
                        shell_file_actions::export_site_zip(website, current_config());
                    },
                    on_copy_css: move |_| {
                        crate::utils::clipboard::copy_to_clipboard((render.generated_css)());
                    },
                    on_toggle_preview: move |_| {
                        // Route through enter_workspace so dock state (incl. Code Nav)
                        // stays consistent however you reach the Code Editor.
                        if center_view() == CenterView::Preview {
                            layout.enter_workspace(CenterView::CodeEditor);
                        } else {
                            layout.enter_workspace(CenterView::Preview);
                        }
                    },
                    on_toggle_split: move |_| { center_view.set(CenterView::Split); },
                    on_reset_viewport: move |_| { layout.preview_width.set(1200u32); },
                    on_hard_refresh: move |_| {
                        website.hard_refresh_preview();
                    },
                }

                MorLayoutChrome {
                    show_preview,
                    center_view,
                    tv_monitor,
                    config_toml_signal,
                    active_preset,
                    original_toml,
                }

                StatusBar { config_toml_signal, original_toml }
            }

            FloatingWindowManager {
                show_preview,
                show_undocked_pages,
                tv_monitor,
            }
        }
    }
}
