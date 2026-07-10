use crate::app::config_bridge::EditorPrefs;
use crate::app::robot_session::{self, sync_policy_from_ui};
use crate::app::state::{ThemeState, WebsiteState};
use crate::ui::components::form::MorCheckbox;
use crate::ui::dialogs::modal::Modal;
use dioxus::prelude::*;
use mor_website_core::utils::robot_assist::{self, RobotTier};

#[component]
pub fn UserPreferencesDialog(
    mut show_prefs: Signal<bool>,
    mut ui_mode_pref: Signal<String>,
    active_ui_mode: String,
) -> Element {
    let theme_state = use_context::<ThemeState>();
    let website = use_context::<WebsiteState>();
    let mut enable_ai_bridge = theme_state.enable_ai_bridge;
    let mut robot_tier = theme_state.robot_tier;
    let mut robot_allow_delete = theme_state.robot_allow_delete;

    let policy_path = robot_assist::policy_path().display().to_string();
    let session_path = robot_assist::session_path().display().to_string();

    let mut persist = move |enabled: bool, tier_str: String, allow_delete: bool| {
        let tier = RobotTier::parse(&tier_str);
        let project_path = {
            let p = website.project.peek();
            if p.is_open() {
                Some(p.root.display().to_string())
            } else {
                robot_assist::load_policy().project_path
            }
        };
        enable_ai_bridge.set(enabled && tier != RobotTier::Off);
        robot_tier.set(if enabled {
            tier.as_str().to_string()
        } else {
            "off".into()
        });
        robot_allow_delete.set(allow_delete);
        sync_policy_from_ui(
            enabled && tier != RobotTier::Off,
            tier,
            allow_delete,
            project_path,
        );
        robot_session::publish_session(theme_state, website, None);
    };

    let tier_now = robot_tier();
    let assist_on = enable_ai_bridge() && tier_now != "off";

    rsx! {
        Modal {
            open: show_prefs,
            title: "User Preferences".to_string(),
            style: "min-width:440px;max-width:580px;".to_string(),
            on_close: move |_| show_prefs.set(false),

            div { class: "editor-field-group",
                label { class: "editor-field-label", "Window Mode" }
                select {
                    class: "editor-select",
                    value: "{ui_mode_pref}",
                    onchange: move |evt| {
                        let new_mode = evt.value();
                        ui_mode_pref.set(new_mode.clone());
                        EditorPrefs::update_ui_mode(new_mode);
                    },
                    option { value: "frameless", "Frameless (Custom OS Buttons)" }
                    option { value: "native", "Native OS Window" }
                    option { value: "tiling", "Tiling WM (No Buttons)" }
                }
                if ui_mode_pref() != active_ui_mode {
                    div { class: "editor-note", style: "margin-top:12px;border-color:var(--editor-warning);background:rgba(210,153,34,0.05);",
                        p { class: "editor-note-title", style: "color:var(--editor-warning);", "Restart Required" }
                        p { class: "editor-note-body", "Restart to apply the new window border setting." }
                    }
                }
            }

            div { class: "editor-field-group",
                label { class: "editor-field-label", "Robot Assist (opt-in MCP power)" }
                p { class: "editor-note-body", style: "margin:0 0 8px;color:var(--fg-muted);font-size:0.8rem;line-height:1.4;",
                    "Lets external agents (Claude, Grok, Cursor) build and theme sites through the MorWebsite MCP engine. Off by default — the editor stays offline until you grant power."
                }
                MorCheckbox {
                    label: "Enable Robot Assist".to_string(),
                    checked: assist_on,
                    onchange: move |val| {
                        let tier = if val {
                            let t = robot_tier();
                            if t == "off" { "site".into() } else { t }
                        } else {
                            "off".into()
                        };
                        persist(val, tier, robot_allow_delete());
                    }
                }
                if assist_on {
                    label { class: "editor-field-label", style: "margin-top:10px;", "Power level" }
                    select {
                        class: "editor-select",
                        value: "{tier_now}",
                        onchange: move |evt| {
                            let t = evt.value();
                            persist(true, t, robot_allow_delete());
                        },
                        option { value: "theme", "Theme — presets + export CSS only" }
                        option { value: "site", "Site — write pages, config, inject links (recommended)" }
                        option { value: "full", "Full — scaffold, zip, optional delete" }
                    }
                    if tier_now == "full" {
                        div { style: "margin-top:8px;",
                            MorCheckbox {
                                label: "Allow delete_file (agents may remove project files)".to_string(),
                                checked: robot_allow_delete(),
                                onchange: move |val| {
                                    persist(true, robot_tier(), val);
                                }
                            }
                        }
                    }
                    div { class: "editor-note", style: "margin-top:10px;",
                        p { class: "editor-note-body", style: "font-size:0.75rem;line-height:1.4;",
                            "Policy: {policy_path}"
                        }
                        p { class: "editor-note-body", style: "font-size:0.75rem;line-height:1.4;",
                            "Session: {session_path}"
                        }
                        p { class: "editor-note-body", style: "font-size:0.75rem;line-height:1.4;margin-top:6px;",
                            "Agents should call get_robot_policy + get_agent_handbook first. Install the MCP bridge from Plugin Manager if needed."
                        }
                    }
                }
            }

            div { class: "editor-field-group",
                label { class: "editor-field-label", "Workspace Defaults" }
                button {
                    class: "editor-btn",
                    onclick: move |_| crate::app::config_bridge::EditorPrefs::clear_default_template_pack(),
                    "Clear Default Template"
                }
            }
        }
    }
}
