//! Insert dock — Google Sites–style content blocks for the preview rich editor.
//!
//! Blocks can be clicked (into active text edit) or **dragged onto the page**.
//! Images: from disk or from URL (optional download into `images/`).

use crate::app::state::{DockPosition, LayoutState, WebsiteState};
use crate::ui::components::dock_chrome::DockChrome;
use crate::ui::workspace::rich_edit::{
    download_image_to_project, iframe_rich_cmd, iframe_rich_insert_html, img_tag,
    import_image_to_project, MEDIA_BLOCKS, TEXT_BLOCKS,
};
use dioxus::prelude::*;

const INSERT_DRAG_JS: &str = r"
(function(){
  if (window.__morInsertDragInstalled) return;
  window.__morInsertDragInstalled = true;
  document.addEventListener('dragstart', function(e) {
    var t = e.target && e.target.closest && e.target.closest('[data-mor-insert-html]');
    if (!t || !e.dataTransfer) return;
    var html = t.getAttribute('data-mor-insert-html') || '';
    if (!html) return;
    try {
      e.dataTransfer.setData('application/x-mor-insert-html', html);
      e.dataTransfer.setData('text/plain', 'MOR_INSERT_HTML:' + html);
      e.dataTransfer.effectAllowed = 'copy';
    } catch (err) {}
  }, true);
})();
";

#[component]
pub fn InsertDock() -> Element {
    let mut layout = use_context::<LayoutState>();
    let site = use_context::<WebsiteState>();
    let pos = (layout.insert_dock_pos)();
    let mut status = use_signal(String::new);
    let mut image_url = use_signal(String::new);
    let mut image_alt = use_signal(String::new);
    let mut download_url = use_signal(|| true);

    if pos == DockPosition::Hidden {
        return rsx! {};
    }

    let project_open = site.project.read().is_open();

    rsx! {
        crate::ui_kit::MorPanelWrapper {
            position: pos,
            default_position: DockPosition::mor_panel_right,
            DockChrome {
                title: "Insert".to_string(),
                dock_id: "insert".to_string(),
                position: pos,
                on_close: move |_| {
                    layout.insert_dock_pos.set(DockPosition::Hidden);
                },
                section {
                    class: "insert-dock-panel",
                    style: "padding: 12px; height: calc(100% - 45px); overflow-y: auto; background: var(--bg-panel); color: var(--fg-base); font-size: 0.85rem; display: flex; flex-direction: column; gap: 14px;",

                    p {
                        style: "margin: 0; font-size: 0.78rem; line-height: 1.45; color: var(--fg-muted);",
                        strong { "Drag" }
                        " a block onto the preview (saves automatically), or click while text is selected. "
                        "Double-click text for the rich toolbar. Drag page sections to reorder."
                    }

                    if !project_open {
                        div { class: "editor-note",
                            p { class: "editor-note-body", "Open a website folder first (File → Open Website Folder…)." }
                        }
                    }

                    div {
                        h4 { style: "margin: 0 0 8px 0; font-size: 0.72rem; letter-spacing: 0.04em; color: var(--fg-muted); font-weight: 700;",
                            "Text & layout"
                        }
                        div { style: "display: flex; flex-direction: column; gap: 6px;",
                            for b in TEXT_BLOCKS {
                                button {
                                    class: "editor-button",
                                    style: "text-align: left; display: flex; flex-direction: column; gap: 2px; padding: 10px 12px; cursor: grab;",
                                    title: "{b.hint}",
                                    disabled: !project_open,
                                    draggable: "true",
                                    "data-mor-insert-html": "{b.html}",
                                    onclick: {
                                        let html = b.html;
                                        let label = b.label;
                                        move |_| {
                                            iframe_rich_insert_html(html);
                                            status.set(format!("Inserted {label}"));
                                        }
                                    },
                                    span { style: "font-weight: 600; font-size: 0.85rem;", "{b.label}" }
                                    span { style: "font-size: 0.72rem; color: var(--fg-muted);", "{b.hint}" }
                                }
                            }
                        }
                    }

                    div {
                        h4 { style: "margin: 0 0 8px 0; font-size: 0.72rem; letter-spacing: 0.04em; color: var(--fg-muted); font-weight: 700;",
                            "Media & chrome"
                        }
                        div { style: "display: flex; flex-direction: column; gap: 6px;",
                            for b in MEDIA_BLOCKS {
                                button {
                                    class: "editor-button",
                                    style: "text-align: left; display: flex; flex-direction: column; gap: 2px; padding: 10px 12px; cursor: grab;",
                                    title: "{b.hint}",
                                    disabled: !project_open,
                                    draggable: "true",
                                    "data-mor-insert-html": "{b.html}",
                                    onclick: {
                                        let html = b.html;
                                        let label = b.label;
                                        move |_| {
                                            iframe_rich_insert_html(html);
                                            status.set(format!("Inserted {label}"));
                                        }
                                    },
                                    span { style: "font-weight: 600; font-size: 0.85rem;", "{b.label}" }
                                    span { style: "font-size: 0.72rem; color: var(--fg-muted);", "{b.hint}" }
                                }
                            }

                            label {
                                style: "display: flex; flex-direction: column; gap: 4px; font-size: 0.78rem; color: var(--fg-muted);",
                                "Image alt text (accessibility)"
                                input {
                                    class: "editor-input",
                                    r#type: "text",
                                    placeholder: "Describe the image…",
                                    value: "{image_alt}",
                                    style: "width: 100%; box-sizing: border-box;",
                                    oninput: move |e| image_alt.set(e.value()),
                                }
                            }

                            button {
                                class: "editor-button editor-button-good",
                                style: "text-align: left; padding: 10px 12px;",
                                disabled: !project_open,
                                title: "Copy an image into images/ and insert an <img> tag",
                                onclick: move |_| {
                                    let site = site;
                                    let mut status = status;
                                    let alt = image_alt();
                                    spawn(async move {
                                        let Some(handle) = rfd::AsyncFileDialog::new()
                                            .set_title("Insert image into site")
                                            .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp", "svg"])
                                            .pick_file()
                                            .await
                                        else {
                                            return;
                                        };
                                        let src = handle.path().to_path_buf();
                                        let fallback_alt = src
                                            .file_stem()
                                            .and_then(|s| s.to_str())
                                            .unwrap_or("image")
                                            .to_string();
                                        let alt = if alt.trim().is_empty() {
                                            fallback_alt
                                        } else {
                                            alt
                                        };
                                        let project = site.project.peek().clone();
                                        let result = tokio::task::spawn_blocking(move || {
                                            import_image_to_project(&project.root, &src)
                                        })
                                        .await;
                                        match result {
                                            Ok(Ok(rel)) => {
                                                iframe_rich_insert_html(&img_tag(&rel, &alt));
                                                site.bump_preview();
                                                status.set(format!("Inserted {rel}"));
                                            }
                                            Ok(Err(e)) => status.set(e),
                                            Err(e) => status.set(format!("Task failed: {e}")),
                                        }
                                    });
                                },
                                span { style: "font-weight: 600;", "Image from disk…" }
                                span { style: "display:block; font-size: 0.72rem; color: var(--fg-muted); margin-top: 2px;",
                                    "Copies into images/ and inserts <img>"
                                }
                            }

                            div {
                                style: "display: flex; flex-direction: column; gap: 6px; padding: 10px 12px; border: 1px solid var(--editor-border-soft, #333); border-radius: 8px;",
                                span { style: "font-weight: 600; font-size: 0.85rem;", "Image from URL" }
                                input {
                                    class: "editor-input",
                                    r#type: "url",
                                    placeholder: "https://example.com/photo.jpg",
                                    value: "{image_url}",
                                    style: "width: 100%; box-sizing: border-box;",
                                    oninput: move |e| image_url.set(e.value()),
                                }
                                label {
                                    style: "display: flex; align-items: center; gap: 8px; font-size: 0.78rem; color: var(--fg-muted);",
                                    input {
                                        r#type: "checkbox",
                                        checked: download_url(),
                                        onchange: move |e| {
                                            download_url.set(e.value() == "true" || e.value() == "on");
                                        },
                                    }
                                    "Download into images/ (recommended)"
                                }
                                button {
                                    class: "editor-mini-button",
                                    disabled: !project_open || image_url().trim().is_empty(),
                                    onclick: move |_| {
                                        let url = image_url().trim().to_string();
                                        if url.is_empty() {
                                            return;
                                        }
                                        let do_dl = download_url();
                                        let alt = image_alt();
                                        let site = site;
                                        let mut status = status;
                                        spawn(async move {
                                            let alt = if alt.trim().is_empty() {
                                                "Image".into()
                                            } else {
                                                alt
                                            };
                                            if do_dl {
                                                let root = site.project.peek().root.clone();
                                                match download_image_to_project(root, url.clone()).await {
                                                    Ok(rel) => {
                                                        iframe_rich_insert_html(&img_tag(&rel, &alt));
                                                        site.bump_preview();
                                                        status.set(format!("Downloaded and inserted {rel}"));
                                                    }
                                                    Err(e) => {
                                                        // Fall back to hotlink so the user still gets something.
                                                        iframe_rich_insert_html(&img_tag(&url, &alt));
                                                        status.set(format!(
                                                            "Download failed ({e}); inserted hotlink instead"
                                                        ));
                                                    }
                                                }
                                            } else {
                                                iframe_rich_insert_html(&img_tag(&url, &alt));
                                                status.set("Inserted image hotlink".into());
                                            }
                                        });
                                    },
                                    "Insert image URL"
                                }
                            }
                        }
                    }

                    div {
                        h4 { style: "margin: 0 0 8px 0; font-size: 0.72rem; letter-spacing: 0.04em; color: var(--fg-muted); font-weight: 700;",
                            "Format selection"
                        }
                        div { style: "display: flex; flex-wrap: wrap; gap: 6px;",
                            button {
                                class: "editor-mini-button",
                                disabled: !project_open,
                                onclick: move |_| iframe_rich_cmd("bold", None),
                                "Bold"
                            }
                            button {
                                class: "editor-mini-button",
                                disabled: !project_open,
                                onclick: move |_| iframe_rich_cmd("italic", None),
                                "Italic"
                            }
                            button {
                                class: "editor-mini-button",
                                disabled: !project_open,
                                onclick: move |_| iframe_rich_cmd("underline", None),
                                "Underline"
                            }
                            button {
                                class: "editor-mini-button",
                                disabled: !project_open,
                                onclick: move |_| iframe_rich_cmd("createLink", None),
                                "Link"
                            }
                            button {
                                class: "editor-mini-button",
                                disabled: !project_open,
                                onclick: move |_| iframe_rich_cmd("insertUnorderedList", None),
                                "List"
                            }
                            button {
                                class: "editor-mini-button",
                                disabled: !project_open,
                                onclick: move |_| iframe_rich_cmd("removeFormat", None),
                                "Clear"
                            }
                        }
                    }

                    if !status().is_empty() {
                        p {
                            style: "margin: 0; font-size: 0.75rem; color: var(--editor-accent, #6d8fb8); line-height: 1.4;",
                            "{status}"
                        }
                    }

                    p {
                        style: "margin: 0; font-size: 0.72rem; color: var(--fg-muted); line-height: 1.4; opacity: 0.9;",
                        "Saves use PHP-aware matching (HTML islands outside <?php ?>). Ambiguous or fully dynamic copy still needs Code view."
                    }

                    // Enable HTML5 drag of [data-mor-insert-html] buttons onto the preview iframe.
                    script {
                        dangerous_inner_html: "{INSERT_DRAG_JS}"
                    }
                }
            }
        }
    }
}
