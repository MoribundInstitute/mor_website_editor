pub use mor_website_core::utils::mcp_install::{
    install_local_mcp_plugin, install_mcp_to_path, list_installed_mcp_binaries,
    list_registered_plugin_ids, load_local_mcp_manifest, mcp_daemon_registry_path, mcp_plugins_dir,
    read_daemon_registry, LocalMcpManifest, McpInstallReport,
};

use mor_website_core::utils::mcp_install::{
    install_mcp_to_claude as core_install_mcp_to_claude,
    mcp_plugins_dir as core_plugins_dir, register_mcp_daemon_entry,
    register_plugin_in_editor_prefs,
};
use reqwest::header::USER_AGENT;
use std::fs;
use std::path::{Path, PathBuf};

/// Official MorWebsite MCP engine (user-facing install target).
pub const OFFICIAL_MCP_REPO: &str = "MoribundMurdoch/mor-website-editor-mcp";
pub const OFFICIAL_MCP_PLUGIN_ID: &str = "mcp_bridge";
pub const OFFICIAL_MCP_SERVER_KEY: &str = "mor_website_engine";
pub const OFFICIAL_MCP_DISPLAY_NAME: &str = "MCP AI Bridge";
const OFFICIAL_MCP_VERSION: &str = "1.0.0";

/// Marketplace / prefs ids that should trigger a real MCP engine install.
pub fn is_mcp_bridge_plugin(id: &str) -> bool {
    let id = id.to_ascii_lowercase();
    id == OFFICIAL_MCP_PLUGIN_ID
        || id == "mor_website_engine"
        || id.contains("mcp_bridge")
        || id == "mor-website-editor-mcp"
}

fn official_manifest(version: &str) -> LocalMcpManifest {
    LocalMcpManifest {
        id: OFFICIAL_MCP_PLUGIN_ID.to_string(),
        display_name: OFFICIAL_MCP_DISPLAY_NAME.to_string(),
        version: version.to_string(),
        description: "Connect Claude / IDE agents to MorWebsite core (presets, CSS compile, diagnostics). Opt-in only."
            .to_string(),
        system_prompt: "You are the MorWebsite MCP engine. Call get_robot_policy and get_agent_handbook first. Respect Robot Assist tiers. Prefer workspace.toml + mor-theme.css; never invent Blogger XML APIs."
            .to_string(),
        entrypoint: String::new(),
        mcp_server_key: OFFICIAL_MCP_SERVER_KEY.to_string(),
    }
}

/// Stable on-disk name after install (release assets may be `mor-mcp-linux`, etc.).
pub fn stable_mcp_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "mor-mcp.exe"
    } else {
        "mor-mcp"
    }
}

/// Copy binary into the plugins dir, mark prefs, daemon registry, and Claude Desktop.
pub fn register_mcp_binary(
    binary_src: &Path,
    version: Option<&str>,
) -> Result<McpInstallReport, String> {
    if !binary_src.exists() {
        return Err(format!("Binary not found: {}", binary_src.display()));
    }

    let version = version.unwrap_or(OFFICIAL_MCP_VERSION);
    let plugins_dir = core_plugins_dir();
    fs::create_dir_all(&plugins_dir).map_err(|e| e.to_string())?;

    // Always install as the stable name so client configs don't break across releases.
    let installed_binary = plugins_dir.join(stable_mcp_binary_name());

    if binary_src != installed_binary {
        fs::copy(binary_src, &installed_binary).map_err(|e| {
            format!(
                "Failed to copy '{}' -> '{}': {e}",
                binary_src.display(),
                installed_binary.display()
            )
        })?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&installed_binary)
            .map_err(|e| e.to_string())?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&installed_binary, perms).map_err(|e| e.to_string())?;
    }

    let manifest = official_manifest(version);
    register_plugin_in_editor_prefs(&manifest.id, &manifest.version)?;
    register_mcp_daemon_entry(&manifest, &installed_binary)?;
    let claude_config =
        core_install_mcp_to_claude(&installed_binary, &manifest.mcp_server_key).ok();

    Ok(McpInstallReport {
        plugin_id: manifest.id,
        binary_path: installed_binary,
        plugin_dir: plugins_dir,
        daemon_registry: mcp_daemon_registry_path(),
        editor_prefs: mor_website_core::config::prefs::editor_prefs_path(),
        claude_config,
    })
}

fn current_os_target() -> &'static str {
    if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "unknown"
    }
}

fn asset_matches_os(name: &str, os_target: &str) -> bool {
    let n = name.to_ascii_lowercase();
    match os_target {
        "linux" => {
            n.contains("linux")
                || n.ends_with("-gnu")
                || (n.contains("mor-mcp") && !n.contains("windows") && !n.contains("darwin") && !n.contains("macos") && !n.contains(".exe"))
        }
        "windows" => n.contains("windows") || n.ends_with(".exe"),
        "macos" => n.contains("macos") || n.contains("darwin") || n.contains("apple"),
        _ => false,
    }
}

/// Prefer release assets that look like the MCP binary for this OS.
fn pick_release_asset(assets: &[serde_json::Value], os_target: &str) -> Option<(String, String)> {
    let mut candidates: Vec<(i32, String, String)> = Vec::new();
    for asset in assets {
        let name = asset["name"].as_str().unwrap_or("").to_string();
        if name.is_empty() || !asset_matches_os(&name, os_target) {
            continue;
        }
        let url = asset["browser_download_url"]
            .as_str()
            .unwrap_or("")
            .to_string();
        if url.is_empty() {
            continue;
        }
        let lower = name.to_ascii_lowercase();
        let mut score = 0;
        if lower.contains("mor-mcp") || lower.contains("mor_website") || lower.contains("mor-website-mcp")
        {
            score += 10;
        }
        if lower.contains(os_target) {
            score += 5;
        }
        if lower.ends_with(".tar.gz") || lower.ends_with(".zip") {
            score -= 3; // prefer raw binary when both exist
        }
        candidates.push((score, name, url));
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    candidates.into_iter().next().map(|(_, n, u)| (n, u))
}

async fn download_to_plugins(url: &str, file_name: &str) -> Result<PathBuf, String> {
    let client = reqwest::Client::new();
    let plugin_dir = core_plugins_dir();
    fs::create_dir_all(&plugin_dir).map_err(|e| e.to_string())?;
    let out_path = plugin_dir.join(file_name);

    let res = client
        .get(url)
        .header(USER_AGENT, "MorWebsite-Plugin-Manager")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!(
            "Download failed with status {} for {}",
            res.status(),
            url
        ));
    }

    let bytes = res.bytes().await.map_err(|e| e.to_string())?;
    if bytes.is_empty() {
        return Err("Downloaded file was empty.".to_string());
    }
    // GitHub soft-404 HTML is large; binary should not start with '<'
    if bytes.starts_with(b"<") {
        return Err(format!(
            "Download did not return a binary (got HTML). Is a release published at {}?",
            OFFICIAL_MCP_REPO
        ));
    }

    fs::write(&out_path, &bytes).map_err(|e| e.to_string())?;
    Ok(out_path)
}

/// Download a plugin binary from any `owner/repo` latest release (generic marketplace path).
pub async fn install_plugin_from_github(repo_path: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let api_url = format!("https://api.github.com/repos/{}/releases/latest", repo_path);

    let res = client
        .get(&api_url)
        .header(USER_AGENT, "MorWebsite-Plugin-Manager")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!(
            "Repository not found or no releases exist for '{repo_path}'. (Status: {})",
            res.status()
        ));
    }

    let release_data: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let version = release_data["tag_name"]
        .as_str()
        .unwrap_or(OFFICIAL_MCP_VERSION)
        .trim_start_matches('v')
        .to_string();
    let assets = release_data["assets"]
        .as_array()
        .ok_or("No compiled assets found in this release.")?;

    let os_target = current_os_target();
    let (file_name, download_url) = pick_release_asset(assets, os_target).ok_or_else(|| {
        format!("Found the release, but no binary matched your OS ({os_target}).")
    })?;

    let out_path = download_to_plugins(&download_url, &file_name).await?;

    let lower = file_name.to_ascii_lowercase();
    if lower.contains("mcp") || repo_path.contains("mcp") {
        let report = register_mcp_binary(&out_path, Some(&version))?;
        return Ok(report
            .binary_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file_name)
            .to_string());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&out_path)
            .map_err(|e| e.to_string())?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&out_path, perms).map_err(|e| e.to_string())?;
    }

    Ok(file_name)
}

/// One-click install of the official MorWebsite MCP engine.
///
/// Uses GitHub Releases for `MoribundMurdoch/mor-website-editor-mcp`. When no
/// release assets exist yet, returns a clear error with build instructions.
pub async fn fetch_and_install_from_github() -> Result<McpInstallReport, String> {
    install_official_mcp_engine().await
}

/// Same as [`fetch_and_install_from_github`] — preferred name for UI call sites.
pub async fn install_official_mcp_engine() -> Result<McpInstallReport, String> {
    let client = reqwest::Client::new();
    let api_url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        OFFICIAL_MCP_REPO
    );

    let res = client
        .get(&api_url)
        .header(USER_AGENT, "MorWebsite-Plugin-Manager")
        .send()
        .await
        .map_err(|e| format!("Network error contacting GitHub: {e}"))?;

    if res.status().as_u16() == 404 {
        return Err(format!(
            "No GitHub release published yet for {OFFICIAL_MCP_REPO}. \
             Build from source (sibling checkout) and use “Install from Disk”, or publish a release with a mor-mcp binary asset."
        ));
    }
    if !res.status().is_success() {
        return Err(format!(
            "GitHub release lookup failed for {OFFICIAL_MCP_REPO} (HTTP {}).",
            res.status()
        ));
    }

    let release_data: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let version = release_data["tag_name"]
        .as_str()
        .unwrap_or(OFFICIAL_MCP_VERSION)
        .trim_start_matches('v')
        .to_string();
    let assets = release_data["assets"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let os_target = current_os_target();
    if os_target == "unknown" {
        return Err("OS not supported for auto-install yet.".to_string());
    }

    let (file_name, download_url) = if let Some(picked) = pick_release_asset(&assets, os_target) {
        picked
    } else {
        // Fallback direct URLs (when assets exist under conventional names).
        let (name, url) = conventional_release_url(os_target);
        // Probe with HEAD/GET — download_to_plugins will fail clearly on HTML.
        (name, url)
    };

    let out_path = download_to_plugins(&download_url, &file_name).await?;
    register_mcp_binary(&out_path, Some(&version))
}

/// Canonical GitHub release asset names (must match MCP repo release workflow).
pub const ASSET_LINUX: &str = "mor-mcp-linux";
pub const ASSET_WINDOWS: &str = "mor-mcp-windows.exe";
pub const ASSET_MACOS: &str = "mor-mcp-macos";

fn conventional_release_url(os_target: &str) -> (String, String) {
    let base = format!(
        "https://github.com/{}/releases/latest/download",
        OFFICIAL_MCP_REPO
    );
    match os_target {
        "windows" => (ASSET_WINDOWS.to_string(), format!("{base}/{ASSET_WINDOWS}")),
        "macos" => (ASSET_MACOS.to_string(), format!("{base}/{ASSET_MACOS}")),
        _ => (ASSET_LINUX.to_string(), format!("{base}/{ASSET_LINUX}")),
    }
}

pub fn install_mcp_to_claude(binary_path: &PathBuf) -> Result<(), String> {
    core_install_mcp_to_claude(binary_path, OFFICIAL_MCP_SERVER_KEY).map(|_| ())
}

/// Install a user-picked binary from disk (Plugin Manager “Install from Disk”).
pub fn install_mcp_binary_from_disk(path: &Path) -> Result<McpInstallReport, String> {
    register_mcp_binary(path, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_temp_file() -> PathBuf {
        let mut path = std::env::temp_dir();
        let name = format!(
            "claude_config_test_{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos().to_string())
                .unwrap_or_else(|_| "fallback".to_string())
        );
        path.push(name);
        path
    }

    #[test]
    fn test_install_mcp_creates_new_config() {
        let config_file = get_temp_file();
        let binary_path = PathBuf::from("/usr/bin/mor_website_engine");

        let res = install_mcp_to_path(&config_file, &binary_path, "mor_website_engine");
        assert!(res.is_ok());

        let content = fs::read_to_string(&config_file).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(
            parsed["mcpServers"]["mor_website_engine"]["command"],
            "/usr/bin/mor_website_engine"
        );

        let _ = fs::remove_file(config_file);
    }

    #[test]
    fn test_install_mcp_updates_existing_config() {
        let config_file = get_temp_file();
        let binary_path = PathBuf::from("/usr/bin/mor_website_engine");

        let initial_json = serde_json::json!({
            "existingKey": "existingValue",
            "mcpServers": {
                "other_server": {
                    "command": "other_binary"
                }
            }
        });
        fs::write(&config_file, initial_json.to_string()).unwrap();

        let res = install_mcp_to_path(&config_file, &binary_path, "mor_website_engine");
        assert!(res.is_ok());

        let content = fs::read_to_string(&config_file).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(parsed["existingKey"], "existingValue");
        assert_eq!(
            parsed["mcpServers"]["other_server"]["command"],
            "other_binary"
        );
        assert_eq!(
            parsed["mcpServers"]["mor_website_engine"]["command"],
            "/usr/bin/mor_website_engine"
        );

        let _ = fs::remove_file(config_file);
    }

    #[test]
    fn test_is_mcp_bridge_plugin() {
        assert!(is_mcp_bridge_plugin("mcp_bridge"));
        assert!(is_mcp_bridge_plugin("MCP_BRIDGE"));
        assert!(!is_mcp_bridge_plugin("ssh_publish"));
    }

    #[test]
    fn test_asset_matches_os() {
        assert!(asset_matches_os("mor-mcp-linux", "linux"));
        assert!(asset_matches_os("mor-mcp.exe", "windows"));
        assert!(asset_matches_os("mor-mcp-darwin", "macos"));
        assert!(!asset_matches_os("mor-mcp-windows.exe", "linux"));
    }
}
