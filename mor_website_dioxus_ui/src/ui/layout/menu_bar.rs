use crate::app::state::{DockPosition, LayoutState, ThemeState};
use crate::ui::layout::shortcut::use_shortcut;

use dioxus::desktop::window;
use dioxus::prelude::*;

const LOGO: Asset = asset!("/assets/images/my_logo.png");

// =========================================================================
// 1. GENERIC BUILDING BLOCKS
// =========================================================================
#[derive(Props, Clone, PartialEq)]
pub struct MorMenuDropdownProps {
    pub label: String,
    pub children: Element,
}

#[component]
pub fn MorMenuDropdown(props: MorMenuDropdownProps) -> Element {
    rsx! {
        div { class: "mor-menu-item",
            "{props.label}"
            div { class: "mor-menu-dropdown",
                {props.children}
            }
        }
    }
}

#[component]
pub fn MorMenuBar(children: Element) -> Element {
    rsx! {
        nav { class: "mor-menu-bar",
            // System menu: window controls on the app icon, like a classic titlebar icon.
            div { class: "mor-menu-item", style: "padding: 0 6px;",
                img {
                    src: LOGO,
                    alt: "MorWebsite",
                    style: "height: 22px; width: auto; flex-shrink: 0; object-fit: contain; pointer-events: none; -webkit-user-select: none;",
                }
                div { class: "mor-menu-dropdown",
                    MenuItem {
                        label: "Minimize".to_string(),
                        on_action: move |_| window().set_minimized(true)
                    }
                    MenuItem {
                        label: "Maximize / Restore".to_string(),
                        on_action: move |_| window().toggle_maximized()
                    }
                    MenuSeparator {}
                    MenuItem {
                        label: "Close".to_string(),
                        on_action: move |_| { window().close(); }
                    }
                }
            }
            {children}
        }
    }
}

#[component]
pub fn MenuItem(
    label: String,
    #[props(default = None)] shortcut: Option<String>,
    #[props(default = false)] disabled: bool,
    #[props(default = None)] on_action: Option<EventHandler<()>>,
) -> Element {
    // Empty string = "no binding" so callers can pass prefs fields directly.
    let shortcut = shortcut.filter(|s| !s.is_empty());
    let bind_shortcut = if disabled { None } else { shortcut.clone() };
    use_shortcut(bind_shortcut, on_action.clone());

    rsx! {
        button {
            class: if disabled { "mor-menu-item disabled" } else { "mor-menu-item" },
            onmousedown: move |evt| evt.stop_propagation(),
            onclick: move |e| {
                e.stop_propagation();
                if !disabled {
                    if let Some(h) = on_action { h.call(()); }
                }
            },
            span { "{label}" }
            if let Some(sc) = shortcut {
                span { class: "shortcut", "{sc}" }
            }
        }
    }
}

#[component]
pub fn MenuSeparator() -> Element {
    rsx! { div { class: "mor-menu-divider" } }
}

// =========================================================================
// 2. THE APP MENU INSTANCE
// =========================================================================
#[component]
pub fn AppMenuBar(
    mut show_prefs: Signal<bool>,
    mut show_editor_settings: Signal<bool>,
    mut show_about: Signal<bool>,
    mut show_shortcuts: Signal<bool>,
    mut show_docs: Signal<bool>,
    mut show_ssh_publish: Signal<bool>,
    on_new_workspace: EventHandler<()>,
    on_open_folder: EventHandler<()>,
    on_load_theme: EventHandler<()>,
    on_save_theme: EventHandler<()>,
    on_export_css: EventHandler<()>,
    on_export_zip: EventHandler<()>,
    on_copy_css: EventHandler<()>,
    on_toggle_preview: EventHandler<()>,
    on_toggle_split: EventHandler<()>,
    on_reset_viewport: EventHandler<()>,
    on_hard_refresh: EventHandler<()>,
) -> Element {
    let theme = use_context::<ThemeState>();
    let mut layout = use_context::<LayoutState>();
    let website = use_context::<crate::app::state::WebsiteState>();
    let project_open = website.project.read().is_open();

    // Customized keybinds; chips re-render live when the rebind dialog writes
    // this signal (the dialog also rekeys the live registry — registrations
    // here only bind once, at mount).
    let sc = use_context::<Signal<crate::app::config_bridge::ShortcutPrefs>>()();
    let combo = |v: &Option<String>| v.clone().unwrap_or_default();

    rsx! {
        MorMenuBar {
            // 1. FILE
            MorMenuDropdown { label: "File".to_string(),
                MenuItem {
                    label: "New Theme".to_string(),
                    on_action: move |_| on_new_workspace.call(())
                }
                MenuItem {
                    label: "Open Website Folder…".to_string(),
                    on_action: move |_| on_open_folder.call(())
                }
                MenuSeparator {}
                MenuItem {
                    label: "Load Site Config (.toml)".to_string(),
                    shortcut: combo(&sc.open_project),
                    on_action: move |_| on_load_theme.call(())
                }
                MenuItem {
                    label: "Save Site Config (.toml)".to_string(),
                    shortcut: combo(&sc.save_project),
                    on_action: move |_| on_save_theme.call(())
                }
                MenuSeparator {}
                MenuItem {
                    label: "Export mor-theme.css".to_string(),
                    shortcut: combo(&sc.export_xml),
                    on_action: move |_| on_export_css.call(())
                }
                MenuItem {
                    label: "Export Site Zip…".to_string(),
                    on_action: move |_| on_export_zip.call(())
                }
                MenuItem {
                    label: "SSH Publish…".to_string(),
                    disabled: !project_open,
                    on_action: move |_| show_ssh_publish.set(true),
                }
                MenuSeparator {}
                MenuItem {
                    label: "Exit".to_string(),
                    shortcut: combo(&sc.exit_architect),
                    on_action: move |_| -> () { std::process::exit(0); }
                }
            }

            // 2. EDIT
            MorMenuDropdown { label: "Edit".to_string(),
                MenuItem {
                    label: "Undo".to_string(),
                    shortcut: combo(&sc.undo),
                    disabled: !theme.can_undo(),
                    on_action: move |_| theme.undo(),
                }
                MenuItem {
                    label: "Redo".to_string(),
                    shortcut: combo(&sc.redo),
                    disabled: !theme.can_redo(),
                    on_action: move |_| theme.redo(),
                }
                MenuSeparator {}
                MenuItem {
                    label: "Copy Theme CSS".to_string(),
                    shortcut: combo(&sc.copy_raw_xml),
                    on_action: move |_| on_copy_css.call(())
                }
            }

            // 3. VIEW
            MorMenuDropdown { label: "View".to_string(),
                MenuItem {
                    label: "Toggle Preview Monitor".to_string(),
                    shortcut: combo(&sc.toggle_preview),
                    on_action: move |_| on_toggle_preview.call(())
                }
                MenuItem {
                    label: "Hard Refresh Preview".to_string(),
                    shortcut: combo(&sc.hard_refresh_preview),
                    on_action: move |_| on_hard_refresh.call(())
                }
                MenuItem {
                    label: "Toggle Code Split".to_string(),
                    on_action: move |_| on_toggle_split.call(())
                }
                MenuItem {
                    label: "Reset Viewport Scale".to_string(),
                    shortcut: combo(&sc.reset_zoom),
                    on_action: move |_| on_reset_viewport.call(())
                }
                MenuSeparator {}
                MenuItem {
                    label: if (layout.designer_mode)() {
                        "Designer Mode ✓ (golden path)".to_string()
                    } else {
                        "Designer Mode (golden path)".to_string()
                    },
                    on_action: move |_| layout.set_designer_mode(true),
                }
                MenuItem {
                    label: if !(layout.designer_mode)() {
                        "Advanced Mode ✓ (all docks)".to_string()
                    } else {
                        "Advanced Mode (all docks)".to_string()
                    },
                    on_action: move |_| layout.set_designer_mode(false),
                }
            }

            // 4. DOCKS
            MorMenuDropdown { label: "Docks".to_string(),
                MenuItem {
                    label: format!("Theme Palette {}", if (layout.theme_palette_pos)() != DockPosition::Hidden { "✓" } else { "" }),
                    on_action: move |_| {
                        if (layout.theme_palette_pos)() == DockPosition::Hidden {
                            layout.theme_palette_pos.set(DockPosition::mor_panel_left);
                        } else {
                            layout.theme_palette_pos.set(DockPosition::Hidden);
                        }
                    }
                }
                MenuItem {
                    label: format!("Site Pages {}", if (layout.site_pages_pos)() != DockPosition::Hidden { "✓" } else { "" }),
                    on_action: move |_| {
                        if (layout.site_pages_pos)() == DockPosition::Hidden {
                            layout.site_pages_pos.set(DockPosition::mor_panel_right);
                        } else {
                            layout.site_pages_pos.set(DockPosition::Hidden);
                        }
                    }
                }
                MenuItem {
                    label: format!("Static Pages {}", if (layout.static_pages_pos)() != DockPosition::Hidden { "✓" } else { "" }),
                    on_action: move |_| {
                        if (layout.static_pages_pos)() == DockPosition::Hidden {
                            layout.static_pages_pos.set(DockPosition::mor_panel_left);
                        } else {
                            layout.static_pages_pos.set(DockPosition::Hidden);
                        }
                    }
                }
                MenuItem {
                    label: format!("CSS Editor {}", if (layout.css_editor_pos)() != DockPosition::Hidden { "✓" } else { "" }),
                    on_action: move |_| {
                        if (layout.css_editor_pos)() == DockPosition::Hidden {
                            layout.css_editor_pos.set(DockPosition::mor_panel_left);
                        } else {
                            layout.css_editor_pos.set(DockPosition::Hidden);
                        }
                    }
                }
                MenuItem {
                    label: format!("JS Editor {}", if (layout.js_editor_pos)() != DockPosition::Hidden { "✓" } else { "" }),
                    on_action: move |_| {
                        if (layout.js_editor_pos)() == DockPosition::Hidden {
                            layout.js_editor_pos.set(DockPosition::mor_panel_left);
                        } else {
                            layout.js_editor_pos.set(DockPosition::Hidden);
                        }
                    }
                }
            }

            // 5. PROFILE
            MorMenuDropdown { label: "Profile".to_string(),
                MenuItem {
                    label: "User Preferences".to_string(),
                    shortcut: combo(&sc.user_prefs),
                    on_action: move |_| show_prefs.set(true)
                }
                MenuItem {
                    label: "Editor Settings (Graphical)".to_string(),
                    on_action: move |_| show_editor_settings.set(true)
                }
                MenuItem {
                    label: "Open Config File (.toml)".to_string(),
                    on_action: move |_| {
                        let path = mor_website_core::config::prefs::editor_prefs_path();
                        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
                    }
                }
            }

            // 6. TOOLS
            MorMenuDropdown { label: "Tools".to_string(),
                MenuItem {
                    label: "Site Diagnostics".to_string(),
                    shortcut: combo(&sc.theme_diagnostics),
                    on_action: move |_| {
                        let pos = (layout.diagnostics_pos)();
                        if pos == DockPosition::Hidden {
                            layout.request_dock("diagnostics", DockPosition::mor_panel_left);
                        } else {
                            layout.diagnostics_pos.set(DockPosition::Hidden);
                        }
                    }
                }
                MenuItem {
                    label: "CSS Token Builder".to_string(),
                    on_action: move |_| {
                        let pos = (layout.css_builder_pos)();
                        if pos == DockPosition::Hidden {
                            layout.request_dock("css_builder", DockPosition::mor_panel_left);
                        } else {
                            layout.css_builder_pos.set(DockPosition::Hidden);
                        }
                    }
                }
                MenuItem {
                    label: "JS Behaviors".to_string(),
                    on_action: move |_| {
                        let pos = (layout.js_builder_pos)();
                        if pos == DockPosition::Hidden {
                            layout.request_dock("js_builder", DockPosition::mor_panel_left);
                        } else {
                            layout.js_builder_pos.set(DockPosition::Hidden);
                        }
                    }
                }
                MenuSeparator {}
                MenuItem {
                    label: "Plugin Manager".to_string(),
                    on_action: move |_| {
                        layout.toggle_dock_by_id("plugin_manager");
                    }
                }
            }

            // 7. HELP
            MorMenuDropdown { label: "Help".to_string(),
                MenuItem {
                    label: "Documentation".to_string(),
                    on_action: move |_| show_docs.set(true)
                }
                MenuItem {
                    label: "Keyboard Shortcuts".to_string(),
                    on_action: move |_| show_shortcuts.set(true)
                }
                MenuSeparator {}
                MenuItem {
                    label: "About MorWebsite".to_string(),
                    on_action: move |_| show_about.set(true)
                }
            }
        }
    }
}
