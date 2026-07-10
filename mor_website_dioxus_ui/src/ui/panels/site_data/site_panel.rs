use dioxus::prelude::*;

use crate::ui::components::inputs::{EditorCard, EditorInput};

#[component]
pub fn SitePanel() -> Element {
    let theme_state = use_context::<crate::app::state::ThemeState>();
    rsx! {
        EditorCard {
            title: "Site Identity".to_string(),

            EditorInput {
                label: "Site Title".to_string(),
                value: theme_state.signals.site_title,
                input_type: "text".to_string(),
                placeholder: "My Site".to_string()
            }

            EditorInput {
                label: "Site Subtitle".to_string(),
                value: theme_state.signals.site_subtitle,
                input_type: "text".to_string(),
                placeholder: "A short tagline".to_string()
            }

            EditorInput {
                label: "Header Logo URL".to_string(),
                value: theme_state.signals.header_logo_url,
                input_type: "text".to_string(),
                placeholder: "/images/logo.svg".to_string()
            }

            EditorInput {
                label: "Home URL".to_string(),
                value: theme_state.signals.home_url,
                input_type: "text".to_string(),
                placeholder: "/".to_string()
            }
        }

        EditorCard {
            title: "How this is used".to_string(),
            div { class: "editor-help-text", style: "margin-bottom: 8px; line-height: 1.45;",
                "These fields live in "
                b { "ThemeConfig" }
                " and are stamped into modular markup via "
                code { "data-mor-edit" }
                " / "
                code { "data-field-path" }
                " markers (see the Site Contract). They restyle and retitle elements the editor knows about — they do not rewrite your whole site."
            }
            div { class: "editor-help-text", style: "margin-bottom: 8px; line-height: 1.45;",
                b { "Tip:" }
                " open the starter project ("
                code { "examples/mor_starter" }
                "), switch the preview to "
                b { "Edit" }
                " mode, and double-click the site title to change it in place."
            }
            div { class: "editor-help-text", style: "line-height: 1.45;",
                "Full rules: "
                code { "docs/SITE_CONTRACT.md" }
            }
        }
    }
}
