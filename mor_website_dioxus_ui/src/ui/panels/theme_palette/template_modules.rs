//! Starter kits (optional): HTML partials for scaffolding a *new* site.
//!
//! Not the primary layout model for an existing PHP/HTML folder — structure
//! lives on disk. Selection still uses `template_pack` slots so export can
//! fold module CSS into `mor-theme.css` and write `mor-starter.html`.

use dioxus::prelude::*;
use mor_website_core::config::ThemeConfig;
use mor_website_core::website::html_modules::{
    module_by_id, modules_for_slot, ModuleSlot, NONE_ID,
};

#[component]
pub fn TemplateModulesPanel(
    current_config: ThemeConfig,
    on_apply_theme: EventHandler<ThemeConfig>,
) -> Element {
    let status = use_signal(String::new);
    let pack = current_config.template_pack.clone();

    rsx! {
        div { class: "editor-panel-content", style: "display: flex; flex-direction: column; gap: 12px;",
            p {
                style: "margin: 0; font-size: 13px; color: var(--editor-fg-muted); line-height: 1.4;",
                "Optional kits for scaffolding a new site — not for rearranging an existing project. "
                "Your real structure is the files on disk (includes, CSS, pages). "
                "Copy kit HTML into pages or write mor-starter.html from Export."
            }

            ModuleSlotSelect {
                label: "Header",
                slot: ModuleSlot::Header,
                default_id: "header_topbar",
                val: pack.header_variant.clone(),
                status,
                on_change: { let c = current_config.clone(); let f = on_apply_theme.clone(); move |v| { let mut nc = c.clone(); nc.template_pack.header_variant = v; f.call(nc); } },
            }
            ModuleSlotSelect {
                label: "Left Sidebar",
                slot: ModuleSlot::Sidebar,
                default_id: "sidebar_nav",
                val: pack.left_sidebar_variant.clone(),
                status,
                on_change: { let c = current_config.clone(); let f = on_apply_theme.clone(); move |v| { let mut nc = c.clone(); nc.template_pack.left_sidebar_variant = v; f.call(nc); } },
            }
            ModuleSlotSelect {
                label: "Right Sidebar",
                slot: ModuleSlot::Sidebar,
                default_id: "sidebar_toc",
                val: pack.right_sidebar_variant.clone(),
                status,
                on_change: { let c = current_config.clone(); let f = on_apply_theme.clone(); move |v| { let mut nc = c.clone(); nc.template_pack.right_sidebar_variant = v; f.call(nc); } },
            }
            ModuleSlotSelect {
                label: "Footer",
                slot: ModuleSlot::Footer,
                default_id: "footer_grid",
                val: pack.footer_variant.clone(),
                status,
                on_change: { let c = current_config.clone(); let f = on_apply_theme.clone(); move |v| { let mut nc = c.clone(); nc.template_pack.footer_variant = v; f.call(nc); } },
            }

            if !status().is_empty() {
                div { class: "restore-status", "{status}" }
            }

            button {
                class: "editor-button",
                onclick: move |_| crate::app::config_bridge::EditorPrefs::update_default_template_pack(current_config.template_pack.clone()),
                "Save as default starter kit"
            }
        }
    }
}

/// One module slot: "None" + the slot's modules, plus the selected module's
/// description and a Copy-HTML button.
#[component]
fn ModuleSlotSelect(
    label: &'static str,
    slot: ModuleSlot,
    default_id: &'static str,
    val: String,
    on_change: EventHandler<String>,
    status: Signal<String>,
) -> Element {
    // Legacy Blogger ids stored in old configs resolve to the slot default in
    // core — mirror that here so the select shows what actually renders, and
    // any change writes the new module ids.
    let effective = if val == NONE_ID || module_by_id(&val).is_some() {
        val
    } else {
        default_id.to_string()
    };
    let selected = module_by_id(&effective);

    rsx! {
        div { class: "editor-card", style: "padding: 8px 12px;",
            label { class: "editor-label", style: "display: block; margin-bottom: 4px; font-size: 0.75rem;", "{label}" }
            select {
                class: "editor-input", style: "width: 100%; font-size: 0.8rem; padding: 4px;",
                value: "{effective}",
                onchange: move |evt| on_change.call(evt.value()),
                option { value: NONE_ID, "None" }
                for m in modules_for_slot(slot) {
                    option { value: "{m.id}", "{m.name}" }
                }
            }
            if let Some(m) = selected {
                p { style: "margin: 6px 0 6px; font-size: 0.72rem; color: var(--editor-fg-muted); line-height: 1.4;", "{m.description}" }
                button {
                    class: "editor-mini-button",
                    title: "Copy this module's HTML to paste into your pages",
                    onclick: move |_| {
                        crate::utils::clipboard::copy_to_clipboard(m.html.to_string());
                        let mut status = status;
                        status.set(format!("{} HTML copied to clipboard.", m.name));
                    },
                    "Copy HTML"
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Legacy Blogger module registry — kept only because the Advanced Module
// Options dialog (template_grid_dialog.rs) still renders from these lists.
// The panel above no longer uses them.
// ─────────────────────────────────────────────────────────────────────────────

pub(crate) struct ModuleDef {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
}

pub(crate) const HEADERS: &[ModuleDef] = &[
    ModuleDef { id: "mor", name: "Mor (Default)", desc: "The standard multi-row header with centered navigation." },
    ModuleDef { id: "mor_search_center", name: "Mor — Centered Search", desc: "Multi-row header with a centered, rounded, slightly larger search bar." },
    ModuleDef { id: "gtk_headerbar", name: "GTK4 Headerbar", desc: "A compact, desktop-style unified titlebar and navigation row." },
    ModuleDef { id: "minimal", name: "Minimal Flexbox", desc: "A lean single-stack flexbox header with branding and links." },
];

pub(crate) const MAIN_CANVASES: &[ModuleDef] = &[
    ModuleDef { id: "sidebars", name: "Three Column (Sidebars)", desc: "Classic blog layout with left and right docking panels." },
    ModuleDef { id: "single_column", name: "Single Column", desc: "A focused, distraction-free reading environment without sidebars." },
    ModuleDef { id: "two_column_right", name: "Two Column Right CSS Grid", desc: "Content with a single right rail on a CSS grid." },
];

pub(crate) const CONTENT_LAYOUTS: &[ModuleDef] = &[
    ModuleDef { id: "standard_feed", name: "Standard Feed (Default)", desc: "Chronological vertical list of full posts." },
    ModuleDef { id: "mor_magazine", name: "Mor Magazine (Hero + Grid)", desc: "A large featured hero post followed by a structured grid." },
    ModuleDef { id: "mor_masonry", name: "Mor Masonry (Pinterest Grid)", desc: "A dense, interlocking Pinterest-style grid of post cards." },
    ModuleDef { id: "mor_minimal", name: "Mor Minimal (Dense List)", desc: "Stripped-down, text-heavy list for rapid scanning." },
];

pub(crate) const LEFT_SIDEBARS: &[ModuleDef] = &[
    ModuleDef { id: "sidebar_labels", name: "Sidebar labels & archive", desc: "Sidebar chrome for labels and archives (Advanced)." },
];

pub(crate) const RIGHT_SIDEBARS: &[ModuleDef] = &[
    ModuleDef { id: "toc_right", name: "Table of Contents", desc: "An empty socket ready for the Dewey Indexer plugin to inject the TOC." },
];

pub(crate) const FOOTERS: &[ModuleDef] = &[
    ModuleDef { id: "mega", name: "Mega Grid (Default)", desc: "Massive 6-column link directory for institutional sites." },
    ModuleDef { id: "basic", name: "Basic Columns", desc: "A standard 4-column layout for links and resources." },
    ModuleDef { id: "compact", name: "Compact Centered", desc: "A single minimal line for copyright and legal links." },
    ModuleDef { id: "social", name: "Social Centered Row", desc: "A centered row of social links with the copyright line." },
];

pub(crate) const JS_BEHAVIORS: &[ModuleDef] = &[
    ModuleDef { id: "mor_collapsible_sidebars", name: "Mor Collapsible Sidebars", desc: "Includes the core framework for mobile collapsible sidebars." },
    ModuleDef { id: "vanilla_base", name: "Vanilla Base (No JS)", desc: "No panel toggle behaviors. Purely static CSS grids." },
];

/// Native file picker that installs the chosen XML as this slot's
/// custom_<key>.xml override (same file the Module Workbench saves to).
#[component]
pub(crate) fn ModuleFileButton(
    module_key: &'static str,
    label: &'static str,
    status: Signal<String>,
    on_loaded: EventHandler<()>,
) -> Element {
    rsx! {
        button {
            class: "editor-mini-button",
            style: "padding: 2px 8px; min-height: 22px; font-size: 0.75rem;",
            title: "Load {label} markup from a file (overrides the dropdown until reset in the Module Workbench)",
            onclick: move |_| {
                let mut status = status;
                let category = crate::ui::workspace::module_workbench::module_key_to_category(module_key);
                let start_dir = mor_website_core::utils::fs_bridge::category_dir(category)
                    .or_else(mor_website_core::utils::fs_bridge::templates_root);
                spawn(async move {
                    let mut dlg = rfd::AsyncFileDialog::new()
                        .add_filter("Module markup", &["xml", "html", "php"])
                        .set_title(format!("Load {label} module markup"));
                    if let Some(dir) = start_dir {
                        dlg = dlg.set_directory(dir);
                    }
                    let Some(handle) = dlg.pick_file().await else { return };
                    let content = match std::fs::read_to_string(handle.path()) {
                        Ok(c) => c,
                        Err(e) => { status.set(format!("Could not read file: {e}")); return; }
                    };
                    match mor_website_core::utils::fs_bridge::save_custom_module(category, &format!("custom_{module_key}"), &content) {
                        Ok(_) => {
                            status.set(format!("{label} loaded from file — overrides the dropdown until reset in the Module Workbench."));
                            on_loaded.call(());
                        }
                        Err(e) => status.set(format!("Load failed: {e}")),
                    }
                });
            },
            "📂"
        }
    }
}
