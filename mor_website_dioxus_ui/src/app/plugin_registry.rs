//! Built-in plugin marketplace registry (always available offline).
//!
//! Remote registry URLs are optional enrichment; if they 404 or return
//! non-JSON, the UI keeps this fallback so MCP AI Bridge remains installable.

use crate::app::config_bridge::CompendiumManifest;

/// Default remote registry (editor repo — always ships with the app source).
pub const DEFAULT_MARKETPLACE_URL: &str =
    "https://raw.githubusercontent.com/MoribundInstitute/mor_website_editor/main/docs/plugin_registry.json";

/// Offline catalog so Plugin Manager never depends on a live network.
pub fn fallback_compendium() -> Vec<CompendiumManifest> {
    vec![
        CompendiumManifest {
            id: "mcp_bridge".to_string(),
            display_name: "MCP AI Bridge".to_string(),
            version: "0.1.0".to_string(),
            description: "One-click install of the MorWebsite MCP engine (GitHub: MoribundMurdoch/mor-website-editor-mcp). Opt-in Robot Assist; editor stays offline otherwise."
                .to_string(),
            payload_url: "https://github.com/MoribundMurdoch/mor-website-editor-mcp".to_string(),
        },
        CompendiumManifest {
            id: "os_chameleon".to_string(),
            display_name: "OS Chameleon".to_string(),
            version: "1.0.0".to_string(),
            description:
                "Automatically toggles dark mode based on the user's OS preference. Offline fallback entry for the marketplace."
                    .to_string(),
            payload_url: "".to_string(),
        },
        CompendiumManifest {
            id: "notification_bell".to_string(),
            display_name: "Notification Bell".to_string(),
            version: "1.0.0".to_string(),
            description:
                "A header bell that opens a dropdown previewing your newest post. Colors follow the active theme."
                    .to_string(),
            payload_url: "".to_string(),
        },
        CompendiumManifest {
            id: "ssh_publish".to_string(),
            display_name: "SSH Publish".to_string(),
            version: "1.0.0".to_string(),
            description:
                "Export mor-theme.css and rsync the project to any SSH host (Hostinger defaults available)."
                    .to_string(),
            payload_url: "https://github.com/MoribundInstitute/mor-website-editor-ssh-publish"
                .to_string(),
        },
    ]
}

/// Merge remote entries over the built-in catalog (by id). Built-ins that the
/// remote omits stay present — so `mcp_bridge` never disappears.
pub fn merge_compendium(remote: Vec<CompendiumManifest>) -> Vec<CompendiumManifest> {
    let mut out = fallback_compendium();
    for remote_entry in remote {
        if let Some(slot) = out.iter_mut().find(|e| e.id == remote_entry.id) {
            *slot = remote_entry;
        } else {
            out.push(remote_entry);
        }
    }
    // Keep MCP first for discoverability.
    out.sort_by(|a, b| {
        let rank = |id: &str| if id == "mcp_bridge" { 0 } else { 1 };
        rank(&a.id).cmp(&rank(&b.id)).then(a.display_name.cmp(&b.display_name))
    });
    out
}

/// Fetch a marketplace registry URL; on any failure return built-in catalog
/// plus a human-readable warning (empty string when ok).
pub async fn fetch_marketplace(url: &str) -> (Vec<CompendiumManifest>, Option<String>) {
    let client = reqwest::Client::new();
    let res = match client
        .get(url)
        .header(reqwest::header::USER_AGENT, "MorWebsite-Plugin-Manager")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return (
                fallback_compendium(),
                Some(format!(
                    "Marketplace offline ({e}). Showing built-in catalog — MCP AI Bridge still works."
                )),
            );
        }
    };

    if !res.status().is_success() {
        return (
            fallback_compendium(),
            Some(format!(
                "Marketplace registry returned HTTP {}. Showing built-in catalog — MCP AI Bridge still works.",
                res.status()
            )),
        );
    }

    let body = match res.text().await {
        Ok(b) => b,
        Err(e) => {
            return (
                fallback_compendium(),
                Some(format!(
                    "Could not read registry body ({e}). Showing built-in catalog."
                )),
            );
        }
    };

    // Trim BOM / leading whitespace; reject HTML/plain 404 pages early.
    let trimmed = body.trim_start_matches('\u{feff}').trim_start();
    if !trimmed.starts_with('[') && !trimmed.starts_with('{') {
        return (
            fallback_compendium(),
            Some(
                "Registry URL did not return JSON (got HTML or plain text). Showing built-in catalog — MCP AI Bridge still works."
                    .into(),
            ),
        );
    }

    match serde_json::from_str::<Vec<CompendiumManifest>>(trimmed) {
        Ok(list) => (merge_compendium(list), None),
        Err(e) => (
            fallback_compendium(),
            Some(format!(
                "Invalid registry JSON ({e}). Showing built-in catalog — MCP AI Bridge still works."
            )),
        ),
    }
}
