//! Protect nested app installs (MediaWiki, WordPress, …) from SSH/rsync publish.
//!
//! Hostinger-style layouts often put the main site in `public_html/` and a
//! separate CMS in `public_html/wiki/` (or a subdomain folder that still
//! lives under the same tree). A naive `rsync` of the project root would
//! overwrite or — with `--delete` — **wipe** those installs.
//!
//! This module builds the exclude list that both `mor-publish` and the
//! in-app SSH Publish dialog always apply.

use std::collections::BTreeSet;
use std::path::Path;

/// Marker files that identify a nested CMS / app install (same idea as scan).
pub const APP_INSTALL_MARKERS: &[&str] = &[
    "LocalSettings.php", // MediaWiki
    "wp-config.php",     // WordPress
    "configuration.php", // Joomla
    "sites/default/settings.php", // Drupal
];

/// Well-known top-level directory names that almost always hold a separate
/// site/subdomain deploy. Always excluded unless the user opts in via
/// `sync_protected = true` in mor-publish.toml.
pub const DEFAULT_PROTECT_DIRS: &[&str] = &[
    "wiki",
    "mediawiki",
    "wordpress",
    "wp",
    "blog", // common WP/subdomain folder name — not blog.php files
];

/// Paths that are never uploaded (tooling / VCS), regardless of protect.
pub const ALWAYS_EXCLUDE: &[&str] = &[
    ".git",
    ".gitignore",
    "mor-publish.toml",
    "node_modules",
    ".env",
    ".env.local",
];

/// Normalize an exclude pattern to rsync directory form (`name/`).
pub fn normalize_exclude_dir(name: &str) -> String {
    let t = name.trim().trim_matches('/').replace('\\', "/");
    if t.is_empty() {
        return String::new();
    }
    format!("{t}/")
}

/// Load `.morignore` path prefixes (one per line), same rules as project scan.
pub fn load_morignore(project_root: &Path) -> Vec<String> {
    std::fs::read_to_string(project_root.join(".morignore"))
        .map(|s| {
            s.lines()
                .map(|l| l.trim().trim_end_matches('/').to_string())
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .collect()
        })
        .unwrap_or_default()
}

/// Top-level subdirectories that contain an app-install marker.
pub fn detect_app_install_dirs(project_root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(project_root) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if is_app_install_dir(&path) {
            found.push(name);
        }
    }
    found.sort();
    found
}

fn is_app_install_dir(dir: &Path) -> bool {
    APP_INSTALL_MARKERS.iter().any(|m| dir.join(m).exists())
}

/// Full set of rsync `--exclude` patterns for a publish run.
///
/// * Always: VCS, env, mor-publish.toml, node_modules
/// * Default protect dirs (`wiki/`, `wordpress/`, …) unless `sync_protected`
/// * Auto-detected CMS installs under the project root
/// * `.morignore` prefixes
/// * User `exclude` from mor-publish.toml
///
/// Deduped, sorted for stable logs/tests.
/// `sync_protected`: when true, skip the built-in protect-dir name list
/// (dangerous). Marker-detected installs and `.morignore` still stay excluded.
pub fn publish_excludes(
    project_root: &Path,
    user_exclude: &[String],
    sync_protected: bool,
) -> Vec<String> {
    let mut set = BTreeSet::new();

    for p in ALWAYS_EXCLUDE {
        set.insert((*p).to_string());
    }

    if !sync_protected {
        for d in DEFAULT_PROTECT_DIRS {
            let n = normalize_exclude_dir(d);
            if !n.is_empty() {
                set.insert(n);
            }
        }
    }

    // Marker-based detection always applies — even if the folder isn't in
    // DEFAULT_PROTECT_DIRS (e.g. `cms/` with LocalSettings.php).
    for d in detect_app_install_dirs(project_root) {
        let n = normalize_exclude_dir(&d);
        if !n.is_empty() {
            set.insert(n);
        }
    }

    for d in load_morignore(project_root) {
        let n = normalize_exclude_dir(&d);
        if !n.is_empty() {
            set.insert(n);
        }
    }

    for e in user_exclude {
        let t = e.trim();
        if t.is_empty() {
            continue;
        }
        // Keep user patterns as-is (may be files, not only dirs).
        set.insert(t.replace('\\', "/"));
    }

    set.into_iter().collect()
}

/// Human-readable summary of which protected trees are being shielded.
pub fn protect_summary(project_root: &Path, sync_protected: bool) -> String {
    let excludes = publish_excludes(project_root, &[], sync_protected);
    let protect: Vec<_> = excludes
        .iter()
        .filter(|e| e.ends_with('/') && !ALWAYS_EXCLUDE.contains(&e.as_str().trim_end_matches('/')))
        .cloned()
        .collect();
    if protect.is_empty() {
        return "No nested app folders protected (none detected).".into();
    }
    format!(
        "Protected from publish (not uploaded; safe from --delete): {}",
        protect.join(" ")
    )
}

/// Reject remote destinations that look like the whole hosting account.
pub fn validate_remote_dir(remote_dir: &str) -> Result<(), String> {
    let remote = remote_dir.trim().trim_end_matches('/');
    if remote.is_empty() || remote == "/" || remote == "." {
        return Err(
            "remote_dir must name a site folder (e.g. domains/example.com/public_html), not account root."
                .into(),
        );
    }
    // Hostinger home-ish paths that would clobber every domain.
    let lower = remote.to_ascii_lowercase();
    if lower == "domains" || lower == "public_html" || lower.ends_with("/domains") {
        return Err(
            "remote_dir looks too broad (domains/ or public_html alone). Point at one site's public_html."
                .into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_dir(label: &str) -> std::path::PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("mor_pub_prot_{label}_{n}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn default_protect_includes_wiki() {
        let dir = tmp_dir("def");
        let ex = publish_excludes(&dir, &[], false);
        assert!(ex.iter().any(|e| e == "wiki/"));
        assert!(ex.iter().any(|e| e == "wordpress/"));
        assert!(ex.iter().any(|e| e == ".git"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn detects_mediawiki_install() {
        let dir = tmp_dir("mw");
        fs::create_dir_all(dir.join("custom-wiki")).unwrap();
        fs::write(dir.join("custom-wiki/LocalSettings.php"), "<?php").unwrap();
        let ex = publish_excludes(&dir, &[], true); // even with sync_protected
        assert!(
            ex.iter().any(|e| e == "custom-wiki/"),
            "marker dirs always protected: {ex:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn morignore_and_user_exclude_merge() {
        let dir = tmp_dir("mi");
        fs::write(dir.join(".morignore"), "subdomain\n# c\n").unwrap();
        let ex = publish_excludes(&dir, &["drafts/".into()], false);
        assert!(ex.iter().any(|e| e == "subdomain/"));
        assert!(ex.iter().any(|e| e == "drafts/"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_remote_dir_blocks_root() {
        assert!(validate_remote_dir("/").is_err());
        assert!(validate_remote_dir("domains").is_err());
        assert!(validate_remote_dir("domains/x.com/public_html").is_ok());
    }
}
