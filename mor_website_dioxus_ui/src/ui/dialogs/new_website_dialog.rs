//! File → New Website… — pick a folder and stack options, then scaffold + open.

use crate::app::config_bridge::EditorPrefs;
use crate::app::shell_file_actions;
use crate::app::state::{ThemeState, WebsiteState};
use crate::app::vfs::VfsDictionary;
use crate::ui::components::form::{MorCheckbox, MorTextInput};
use crate::ui::dialogs::modal::Modal;
use dioxus::prelude::*;
use mor_website_core::website::{NewSiteOptions, NewSitePages};

fn slugify_folder(title: &str) -> String {
    let mut s: String = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else if c.is_whitespace() || c == '-' || c == '_' {
                '-'
            } else {
                '\0'
            }
        })
        .filter(|c| *c != '\0')
        .collect();
    while s.contains("--") {
        s = s.replace("--", "-");
    }
    s.trim_matches('-').to_string()
}

#[component]
pub fn NewWebsiteDialog(mut open: Signal<bool>) -> Element {
    let mut theme = use_context::<ThemeState>();
    let website = use_context::<WebsiteState>();
    let vfs = use_context::<VfsDictionary>().0;
    let mut original_toml = use_context::<Signal<String>>();
    let mut workbench_status =
        use_context::<crate::app::shell::WorkbenchEditState>().workbench_status;

    let mut site_title = use_signal(|| "My Website".to_string());
    let mut folder_name = use_signal(|| "my-website".to_string());
    let mut parent_path = use_signal(String::new);
    let mut use_php = use_signal(|| true);
    let mut include_css = use_signal(|| true);
    let mut include_js = use_signal(|| true);
    let mut about_page = use_signal(|| true);
    let mut status = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut title_drives_folder = use_signal(|| true);

    if !open() {
        return rsx! { Fragment {} };
    }

    let parent_label = {
        let p = parent_path();
        if p.is_empty() {
            "(choose a parent folder)".to_string()
        } else {
            p
        }
    };
    let preview_path = {
        let parent = parent_path();
        let name = folder_name();
        if parent.is_empty() || name.trim().is_empty() {
            String::new()
        } else {
            std::path::Path::new(&parent)
                .join(name.trim())
                .display()
                .to_string()
        }
    };

    rsx! {
        Modal {
            open: open,
            title: "New Website".to_string(),
            on_close: move |_| {
                status.set(String::new());
                busy.set(false);
            },
            style: "max-width: 520px; width: min(520px, 94vw);".to_string(),

            div {
                style: "display: flex; flex-direction: column; gap: 14px; padding: 4px 2px 8px;",

                p {
                    style: "margin: 0; font-size: 0.85rem; color: var(--fg-muted); line-height: 1.45;",
                    "Create a folder of pages on disk, write a starter theme, and open it in the editor."
                }

                MorTextInput {
                    label: "Site title".to_string(),
                    value: site_title(),
                    onchange: move |v: String| {
                        site_title.set(v.clone());
                        if title_drives_folder() {
                            let slug = slugify_folder(&v);
                            if !slug.is_empty() {
                                folder_name.set(slug);
                            }
                        }
                    }
                }

                MorTextInput {
                    label: "Folder name".to_string(),
                    value: folder_name(),
                    onchange: move |v: String| {
                        title_drives_folder.set(false);
                        folder_name.set(v);
                    }
                }

                div { style: "display: flex; flex-direction: column; gap: 6px;",
                    div { style: "font-size: 0.8rem; color: var(--fg-muted);", "Location" }
                    div { style: "display: flex; gap: 8px; align-items: center; flex-wrap: wrap;",
                        button {
                            class: "editor-button",
                            disabled: busy(),
                            onclick: move |_| {
                                spawn(async move {
                                    let Some(folder) = rfd::AsyncFileDialog::new()
                                        .set_title("Choose parent folder for the new website")
                                        .pick_folder()
                                        .await
                                    else {
                                        return;
                                    };
                                    parent_path.set(folder.path().display().to_string());
                                });
                            },
                            "Choose parent folder…"
                        }
                        span {
                            style: "font-family: monospace; font-size: 0.75rem; color: var(--fg-muted); overflow: hidden; text-overflow: ellipsis; max-width: 100%;",
                            title: "{parent_label}",
                            "{parent_label}"
                        }
                    }
                    if !preview_path.is_empty() {
                        p {
                            style: "margin: 0; font-size: 0.75rem; color: var(--fg-muted); font-family: monospace; word-break: break-all;",
                            "Will create: {preview_path}"
                        }
                    }
                }

                div {
                    style: "display: flex; flex-direction: column; gap: 8px; padding: 10px 12px; border: 1px solid var(--border-color, #333); border-radius: 6px; background: color-mix(in srgb, var(--bg-elevated, #1a1a1a) 80%, transparent);",
                    div { style: "font-size: 0.8rem; font-weight: 600;", "Starting options" }

                    div { style: "display: flex; flex-direction: column; gap: 6px;",
                        label { style: "display: flex; gap: 8px; align-items: flex-start; cursor: pointer; font-size: 0.85rem;",
                            input {
                                r#type: "radio",
                                name: "mor-new-site-pages",
                                checked: use_php(),
                                onchange: move |_| use_php.set(true),
                            }
                            span {
                                strong { "PHP (modular)" }
                                " — index + shared includes/header.php & footer.php (recommended for Hostinger-style hosts)."
                            }
                        }
                        label { style: "display: flex; gap: 8px; align-items: flex-start; cursor: pointer; font-size: 0.85rem;",
                            input {
                                r#type: "radio",
                                name: "mor-new-site-pages",
                                checked: !use_php(),
                                onchange: move |_| use_php.set(false),
                            }
                            span {
                                strong { "Static HTML" }
                                " — plain .html pages, no PHP required."
                            }
                        }
                    }

                    MorCheckbox {
                        label: "Site CSS (css/site.css linked from every page)".to_string(),
                        checked: include_css(),
                        onchange: move |v| include_css.set(v),
                    }
                    MorCheckbox {
                        label: "JavaScript (mor-card component + js/site.js)".to_string(),
                        checked: include_js(),
                        onchange: move |v| include_js.set(v),
                    }
                    MorCheckbox {
                        label: "About page".to_string(),
                        checked: about_page(),
                        onchange: move |v| about_page.set(v),
                    }
                }

                if !status().is_empty() {
                    p {
                        style: "margin: 0; font-size: 0.8rem; color: var(--d29922, #d29922); line-height: 1.4;",
                        "{status}"
                    }
                }

                div { style: "display: flex; gap: 8px; justify-content: flex-end; margin-top: 4px;",
                    button {
                        class: "editor-button",
                        disabled: busy(),
                        onclick: move |_| {
                            open.set(false);
                            status.set(String::new());
                        },
                        "Cancel"
                    }
                    button {
                        class: "editor-button editor-button-good",
                        disabled: busy()
                            || parent_path().trim().is_empty()
                            || folder_name().trim().is_empty()
                            || site_title().trim().is_empty(),
                        onclick: move |_| {
                            if busy() {
                                return;
                            }
                            let parent = parent_path();
                            let name = folder_name().trim().to_string();
                            let title = site_title().trim().to_string();
                            if parent.is_empty() || name.is_empty() || title.is_empty() {
                                status.set("Choose a parent folder, folder name, and site title.".into());
                                return;
                            }
                            // Guard path traversal in folder name.
                            if name.contains("..") || name.contains('/') || name.contains('\\') {
                                status.set("Folder name cannot contain path separators.".into());
                                return;
                            }
                            let dest = std::path::PathBuf::from(&parent).join(&name);
                            if dest.exists()
                                && dest
                                    .read_dir()
                                    .map(|mut d| d.next().is_some())
                                    .unwrap_or(true)
                            {
                                status.set(format!(
                                    "Folder already exists and is not empty: {}",
                                    dest.display()
                                ));
                                return;
                            }

                            let opts = NewSiteOptions {
                                pages: if use_php() {
                                    NewSitePages::PhpModular
                                } else {
                                    NewSitePages::StaticHtml
                                },
                                include_site_css: include_css(),
                                include_js: include_js(),
                                about_page: about_page(),
                            };

                            busy.set(true);
                            status.set("Creating website…".into());

                            spawn(async move {
                                let result = tokio::task::spawn_blocking(move || {
                                    let mut config =
                                        mor_website_core::config::defaults::default_theme_config();
                                    if let Some(pack) = EditorPrefs::load().default_template_pack {
                                        config.template_pack = pack;
                                    }
                                    config.site.site_title = title;
                                    if config.site.site_subtitle.trim().is_empty()
                                        || config.site.site_subtitle.contains("starter")
                                    {
                                        config.site.site_subtitle =
                                            "A clean starter for pages and posts.".into();
                                    }
                                    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
                                    let written =
                                        mor_website_core::website::scaffold_new_site(
                                            &dest, &config, &opts,
                                        )
                                        .map_err(|e| e.to_string())?;
                                    Ok::<_, String>((dest, config, written))
                                })
                                .await;

                                busy.set(false);
                                match result {
                                    Ok(Ok((dest, config, written))) => {
                                        theme.signals.apply_config(&config);
                                        original_toml.set(
                                            toml::to_string_pretty(&config).unwrap_or_default(),
                                        );
                                        theme.active_preset.set(None);
                                        theme.commit();

                                        shell_file_actions::open_website_path(
                                            website,
                                            vfs,
                                            dest.clone(),
                                            theme,
                                            original_toml,
                                        )
                                        .await;

                                        let msg = format!(
                                            "Created website ({} files) → {}",
                                            written.len(),
                                            dest.display()
                                        );
                                        status.set(msg.clone());
                                        workbench_status.set(msg);
                                        open.set(false);
                                    }
                                    Ok(Err(e)) => status.set(format!("Could not create site: {e}")),
                                    Err(e) => status.set(format!("Create task failed: {e}")),
                                }
                            });
                        },
                        if busy() { "Creating…" } else { "Create website" }
                    }
                }
            }
        }
    }
}
