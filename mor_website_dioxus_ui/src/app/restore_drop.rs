use dioxus::prelude::*;

use crate::app::state::ThemeState;
use crate::app::vfs::VfsDictionary;
use mor_website_core::utils::rehydration::{
    extract_and_decode, extract_workspace_state, persist_vfs_overrides,
};

pub fn use_restore_drop_bridge(theme: ThemeState) {
    let signals = theme.signals;
    let active_preset = theme.active_preset;
    use_effect(move || {
        let mut eval = dioxus::document::eval(
            r#"
            (function () {
                if (window.__morRestoreWorkspaceNativeDropInstalled) {
                    dioxus.send({
                        kind: "restore_drop_ready",
                        message: "Restore drop bridge already installed."
                    });
                    return;
                }

                window.__morRestoreWorkspaceNativeDropInstalled = true;

                function hasFileDrag(event) {
                    if (!event.dataTransfer) return false;

                    if (event.dataTransfer.types) {
                        for (const item of event.dataTransfer.types) {
                            if (item === "Files") return true;
                        }
                    }

                    return event.dataTransfer.files && event.dataTransfer.files.length > 0;
                }

                function findThemeFile(files) {
                    for (const file of files) {
                        const name = (file.name || "").toLowerCase();
                        const type = (file.type || "").toLowerCase();

                        if (
                            name.endsWith(".toml") ||
                            name === "workspace.toml" ||
                            name.endsWith(".xml") ||
                            type === "text/xml" ||
                            type === "application/xml" ||
                            type === "application/toml" ||
                            type === "text/plain"
                        ) {
                            return file;
                        }
                    }

                    return files.length > 0 ? files[0] : null;
                }

                window.addEventListener("dragover", function (event) {
                    if (!hasFileDrag(event)) return;

                    event.preventDefault();
                    if (event.dataTransfer) {
                        event.dataTransfer.dropEffect = "copy";
                    }
                }, true);

                window.addEventListener("dragenter", function (event) {
                    if (!hasFileDrag(event)) return;

                    event.preventDefault();
                    if (event.dataTransfer) {
                        event.dataTransfer.dropEffect = "copy";
                    }
                }, true);

                window.addEventListener("drop", async function (event) {
                    if (!hasFileDrag(event)) return;

                    event.preventDefault();

                    const files = Array.from(event.dataTransfer.files || []);
                    const file = findThemeFile(files);

                    if (!file) {
                        dioxus.send({
                            kind: "restore_drop_error",
                            message: "Drop detected, but no file was exposed by the desktop webview."
                        });
                        return;
                    }

                    try {
                        const text = await file.text();

                        dioxus.send({
                            kind: "restore_workspace_drop",
                            name: file.name || "dropped theme file",
                            text: text
                        });
                    } catch (error) {
                        dioxus.send({
                            kind: "restore_drop_error",
                            message: "Could not read dropped file: " + String(error)
                        });
                    }
                }, true);

                dioxus.send({
                    kind: "restore_drop_ready",
                    message: "Restore drop bridge installed."
                });
            })();
            "#,
        );

        let restore_signals = signals;
        let mut restored_active_preset = active_preset;
        let mut vfs = use_context::<VfsDictionary>().0;

        spawn(async move {
            while let Ok(value) = eval.recv::<serde_json::Value>().await {
                let Some(kind) = value.get("kind").and_then(|v| v.as_str()) else {
                    continue;
                };

                match kind {
                    // Prefer new event name; keep legacy id for mid-session reloads.
                    "restore_workspace_drop" | "restore_xml_drop" => {
                        let name = value
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("dropped theme file");

                        let Some(text) = value.get("text").and_then(|v| v.as_str()) else {
                            log::error!(
                                "Dropped theme event from desktop bridge did not include text."
                            );
                            continue;
                        };

                        // Full payload (TOML or XML marker + optional VFS) when present;
                        // otherwise plain workspace.toml → ThemeConfig only.
                        let restored = extract_workspace_state(text).or_else(|_| {
                            extract_and_decode(text).map(|config| {
                                mor_website_core::utils::rehydration::RehydrationPayload::from_config(
                                    config,
                                )
                            })
                        });

                        match restored {
                            Ok(payload) => {
                                restore_signals.apply_config(&payload.config);
                                if !payload.vfs.is_empty() {
                                    vfs.write().clear();
                                    vfs.write().extend(payload.vfs.clone());
                                    if let Err(err) = persist_vfs_overrides(&payload.vfs) {
                                        log::error!(
                                            "Failed to persist restored VFS overrides: {}",
                                            err
                                        );
                                    }
                                }
                                restored_active_preset.set(None);
                                theme.commit();
                                log::info!("Workspace restored from dropped file: {}", name);
                            }
                            Err(err) => {
                                log::error!("Failed to restore dropped file {}: {}", name, err);
                            }
                        }
                    }
                    "restore_drop_error" => {
                        let message = value
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown restore drop error.");
                        log::error!("{}", message);
                    }
                    "restore_drop_ready" => {
                        let message = value
                            .get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Restore drop bridge ready.");
                        log::info!("{}", message);
                    }
                    _ => {}
                }
            }
        });
    });
}
