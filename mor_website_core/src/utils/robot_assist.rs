//! Opt-in Robot Assist policy and live session handoff for external agents.
//!
//! Default is **off**. When enabled, MCP tools may mutate the open website
//! project (tier-gated). The editor writes a session snapshot so agents can
//! discover the active project without guessing paths.

use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// How much power robots are granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RobotTier {
    /// No agent writes (default).
    #[default]
    Off,
    /// Read project + write/read presets + export theme CSS.
    Theme,
    /// Theme + write pages/assets/config under the project.
    Site,
    /// Site + scaffold, zip, optional delete.
    Full,
}

impl RobotTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Theme => "theme",
            Self::Site => "site",
            Self::Full => "full",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "theme" => Self::Theme,
            "site" => Self::Site,
            "full" => Self::Full,
            _ => Self::Off,
        }
    }

    pub fn rank(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Theme => 1,
            Self::Site => 2,
            Self::Full => 3,
        }
    }

    pub fn allows(self, needed: RobotTier) -> bool {
        if needed == Self::Off {
            return true;
        }
        if self == Self::Off {
            return false;
        }
        self.rank() >= needed.rank()
    }

    /// True if this tier grants any write capability.
    pub fn writes_enabled(self) -> bool {
        !matches!(self, Self::Off)
    }
}

/// Persistent policy read by `mor-mcp` and written by the editor UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RobotPolicy {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub tier: RobotTier,
    /// Last opened / pinned project absolute path (optional).
    #[serde(default)]
    pub project_path: Option<String>,
    /// Full tier only: allow `delete_file`.
    #[serde(default)]
    pub allow_delete: bool,
    #[serde(default)]
    pub updated_unix: u64,
}

impl Default for RobotPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            tier: RobotTier::Off,
            project_path: None,
            allow_delete: false,
            updated_unix: 0,
        }
    }
}

impl RobotPolicy {
    pub fn effective_tier(&self) -> RobotTier {
        if !self.enabled {
            RobotTier::Off
        } else if self.tier == RobotTier::Off {
            // enabled with unset tier → treat as theme (minimum useful)
            RobotTier::Theme
        } else {
            self.tier
        }
    }

    pub fn require(&self, needed: RobotTier) -> Result<(), String> {
        let have = self.effective_tier();
        if needed == RobotTier::Off {
            return Ok(());
        }
        if have == RobotTier::Off {
            return Err(format!(
                "robot_assist_disabled: enable Robot Assist in MorWebsite Editor Preferences \
                 (need tier ≥ {}). Currently off.",
                needed.as_str()
            ));
        }
        if have.rank() < needed.rank() {
            return Err(format!(
                "robot_assist_tier: need tier ≥ {} (have {}). Raise Robot Assist power in Preferences.",
                needed.as_str(),
                have.as_str()
            ));
        }
        Ok(())
    }

    pub fn require_delete(&self) -> Result<(), String> {
        self.require(RobotTier::Full)?;
        if !self.allow_delete {
            return Err(
                "robot_assist_delete_disabled: enable “Allow delete” under Robot Assist (Full)."
                    .into(),
            );
        }
        Ok(())
    }
}

/// Live handoff the editor writes so agents can attach to the open site.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct SessionSnapshot {
    #[serde(default)]
    pub project_root: Option<String>,
    #[serde(default)]
    pub active_page: Option<String>,
    #[serde(default)]
    pub workspace_toml_path: Option<String>,
    #[serde(default)]
    pub active_preset: Option<String>,
    #[serde(default)]
    pub tier: String,
    #[serde(default)]
    pub enable_ai_bridge: bool,
    #[serde(default)]
    pub timestamp: u64,
    /// Optional unsaved buffer from an asset editor (CSS/JS workbench).
    #[serde(default)]
    pub live_buffer: Option<LiveBuffer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct LiveBuffer {
    #[serde(default)]
    pub active_file: String,
    #[serde(default)]
    pub unsaved_content: String,
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `~/.config/mor_website/robot_assist.toml`
pub fn policy_path() -> PathBuf {
    BaseDirs::new()
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(std::env::temp_dir)
        .join("mor_website/robot_assist.toml")
}

/// Prefer `$XDG_RUNTIME_DIR`, then `/tmp`.
pub fn session_path() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        let p = PathBuf::from(dir);
        if p.is_dir() {
            return p.join("mor_website_session.json");
        }
    }
    PathBuf::from("/tmp/mor_website_session.json")
}

/// Legacy path still written for older agents.
pub fn legacy_live_state_path() -> PathBuf {
    PathBuf::from("/tmp/mor_website_live_state.json")
}

pub fn load_policy() -> RobotPolicy {
    let path = policy_path();
    if !path.exists() {
        return RobotPolicy::default();
    }
    let Ok(raw) = fs::read_to_string(&path) else {
        return RobotPolicy::default();
    };
    toml::from_str(&raw).unwrap_or_default()
}

pub fn save_policy(mut policy: RobotPolicy) -> Result<PathBuf, String> {
    policy.updated_unix = now_unix();
    // Keep enabled in sync with tier.
    if policy.tier == RobotTier::Off {
        policy.enabled = false;
    } else if policy.enabled && policy.tier == RobotTier::Off {
        policy.tier = RobotTier::Theme;
    }
    let path = policy_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = toml::to_string_pretty(&policy).map_err(|e| e.to_string())?;
    fs::write(&path, raw).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn write_session(mut snap: SessionSnapshot) -> Result<PathBuf, String> {
    snap.timestamp = now_unix();
    let path = session_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let raw = serde_json::to_string_pretty(&snap).map_err(|e| e.to_string())?;
    fs::write(&path, raw).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn read_session() -> Result<SessionSnapshot, String> {
    let path = session_path();
    if !path.exists() {
        return Ok(SessionSnapshot::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| format!("invalid session json: {e}"))
}

pub fn clear_session() {
    let _ = fs::remove_file(session_path());
    let _ = fs::remove_file(legacy_live_state_path());
}

/// Resolve a project-relative path; refuse escape.
pub fn jail_rel_path(project_root: &Path, rel: &str) -> Result<PathBuf, String> {
    let rel = rel.trim().trim_start_matches('/');
    if rel.is_empty() {
        return Err("empty path".into());
    }
    if rel.contains("..") || Path::new(rel).is_absolute() {
        return Err("path escapes project (no .. or absolute paths)".into());
    }
    let forbidden = [".git/", "node_modules/", "target/", ".svn/", ".hg/"];
    let lower = rel.replace('\\', "/");
    for f in forbidden {
        if lower.starts_with(f) || lower.contains(&format!("/{f}")) {
            return Err(format!("writes into {f} are not allowed"));
        }
    }
    let dest = project_root.join(rel);
    // Extra check after join
    let root = project_root
        .canonicalize()
        .unwrap_or_else(|_| project_root.to_path_buf());
    if let Ok(canon) = dest.canonicalize() {
        if !canon.starts_with(&root) {
            return Err("path escapes project root".into());
        }
    } else if let Some(parent) = dest.parent() {
        if let Ok(p) = parent.canonicalize() {
            if !p.starts_with(&root) && p != root {
                // parent may not exist yet — allow if relative under root by components
                if dest.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
                    return Err("path escapes project root".into());
                }
            }
        }
    }
    Ok(dest)
}

/// Agent handbook: how to build sites with MorWebsite Editor via MCP.
pub fn agent_handbook() -> &'static str {
    r#"# MorWebsite Robot Assist — Agent Handbook

You are helping a human build a **local hand-rolled website** with MorWebsite Editor.
There is no Blogger XML here. Prefer `workspace.toml` + HTML/PHP pages + `mor-theme.css`.

## Opt-in policy

Power is granted by the human in **Preferences → Robot Assist**:

| Tier | You may |
|------|---------|
| off | Read session/policy only; writes fail |
| theme | Read project, presets, compile/export theme CSS |
| site | theme + write pages/CSS/JS, workspace.toml, inject links |
| full | site + scaffold starter, zip bundle, optional delete |

Call `get_robot_policy` and `get_session` first. If disabled, tell the human to enable Robot Assist.

## Site Contract (short)

- Tokens live in ThemeConfig / compiled `mor-theme.css` (`--bg-base`, `--accent`, …).
- Markup uses hooks: `.mor-topbar`, `.mor-card`, `.mor-pill`, …
- Optional edit markers: `data-mor-edit="site.site_title"`.
- Every page should link `/mor-theme.css` (use `inject_theme_links` after export).

## Workflows

### New site
1. `scaffold_site` (full) or open an empty folder
2. `write_site_config` with a ThemeConfig TOML
3. `write_page` for index + about
4. `export_theme_css` then `inject_theme_links`
5. `run_diagnostics` and fix warnings

### Restyle existing site
1. `get_session` → `open_project`
2. `list_presets` / `read_preset` / `write_preset` (live preview if editor open)
3. or `apply_preset` into workspace.toml
4. `export_theme_css` + `inject_theme_links`

### Fix broken theme
1. `run_diagnostics`
2. `read_file` / `read_page`
3. `write_file` / `write_page`
4. re-export and diagnose

## Rules

- Stay inside the project root (and presets dir for presets).
- Validate TOML through tools (they use the editor's loaders).
- Prefer modular hooks over one-off hex in every page.
- Never invent Blogger `<b:…>` APIs.
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_ranking() {
        assert!(RobotTier::Full.allows(RobotTier::Site));
        assert!(RobotTier::Site.allows(RobotTier::Theme));
        assert!(!RobotTier::Theme.allows(RobotTier::Site));
        assert!(!RobotTier::Off.allows(RobotTier::Theme));
    }

    #[test]
    fn require_messages() {
        let off = RobotPolicy::default();
        assert!(off.require(RobotTier::Theme).unwrap_err().contains("robot_assist_disabled"));

        let theme = RobotPolicy {
            enabled: true,
            tier: RobotTier::Theme,
            ..Default::default()
        };
        assert!(theme.require(RobotTier::Theme).is_ok());
        assert!(theme.require(RobotTier::Site).unwrap_err().contains("robot_assist_tier"));
    }

    #[test]
    fn jail_blocks_escape() {
        let root = std::env::temp_dir().join("mor_robot_jail_test");
        let _ = fs::create_dir_all(&root);
        assert!(jail_rel_path(&root, "../etc/passwd").is_err());
        assert!(jail_rel_path(&root, "index.html").is_ok());
        assert!(jail_rel_path(&root, ".git/config").is_err());
    }

    #[test]
    fn policy_roundtrip() {
        // Use unique env-free path by writing via save then load — may clobber user policy
        // in real home; instead test serialize only.
        let p = RobotPolicy {
            enabled: true,
            tier: RobotTier::Full,
            project_path: Some("/tmp/site".into()),
            allow_delete: true,
            updated_unix: 1,
        };
        let raw = toml::to_string(&p).unwrap();
        let back: RobotPolicy = toml::from_str(&raw).unwrap();
        assert_eq!(back.tier, RobotTier::Full);
        assert!(back.allow_delete);
    }
}
