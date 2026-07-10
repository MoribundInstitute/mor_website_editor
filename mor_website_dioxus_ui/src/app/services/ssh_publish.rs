//! SSH Publish — load/save `mor-publish.toml` and rsync a website project
//! over SSH (Hostinger defaults). Mirrors the companion `mor-publish` CLI so
//! the in-app dialog works without installing the binary.
//!
//! Nested CMS installs (MediaWiki, WordPress, …) are excluded by default via
//! [`mor_website_core::website::publish_protect`].
//!
//! # Security model (keys only)
//!
//! - **Never** store or accept Hostinger panel passwords in config or UI.
//! - All SSH/rsync uses `BatchMode=yes` + `PasswordAuthentication=no` so the
//!   app cannot fall back to interactive password auth (no hang, no leak).
//! - Private key path must be a local file with owner-only permissions.
//! - `mor-publish.toml` is auto-gitignored (host/user only — still not a secret
//!   store, but avoids accidental commit of account usernames).

use mor_website_core::website::publish_protect::{
    protect_summary, publish_excludes, validate_remote_dir,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const CONFIG_FILE: &str = "mor-publish.toml";

/// Shared OpenSSH options for every connection from this app.
const SECURE_SSH_OPTS: &[&str] = &[
    "BatchMode=yes",
    "PasswordAuthentication=no",
    "KbdInteractiveAuthentication=no",
    "PreferredAuthentications=publickey",
    "PubkeyAuthentication=yes",
    "ConnectTimeout=15",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PublishConfig {
    /// Hostinger IP or hostname (hPanel → SSH Access → IP).
    pub host: String,
    /// Hostinger username (e.g. u123456789).
    pub user: String,
    /// Hostinger SSH port (often 65002, not 22).
    #[serde(default = "default_port")]
    pub port: u16,
    pub remote_dir: String,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub delete: bool,
    /// When true, also sync well-known protect dirs (`wiki/`, `wordpress/`, …).
    /// Marker-detected installs and `.morignore` entries stay excluded.
    #[serde(default)]
    pub sync_protected: bool,
    /// Optional private key path (e.g. `~/.ssh/id_ed25519`). Empty = ssh default.
    #[serde(default)]
    pub identity_file: String,
}

fn default_port() -> u16 {
    65002
}

impl Default for PublishConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            user: String::new(),
            port: 65002,
            remote_dir: "domains/example.com/public_html".into(),
            exclude: Vec::new(),
            delete: false,
            sync_protected: false,
            identity_file: String::new(),
        }
    }
}

impl PublishConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.host.trim().is_empty() {
            return Err("IP / host is required (hPanel → Advanced → SSH Access).".into());
        }
        if self.user.trim().is_empty() {
            return Err("Username is required (hPanel → Advanced → SSH Access).".into());
        }
        validate_remote_dir(&self.remote_dir)?;
        if self.port == 0 {
            return Err("Port must be a positive number (Hostinger often uses 65002).".into());
        }
        validate_identity_path(self.identity_file.trim())?;
        Ok(())
    }
}

/// Reject paths that look like secrets pasted into the path field; require
/// owner-only permissions on real key files.
pub fn validate_identity_path(raw: &str) -> Result<Option<PathBuf>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    if raw.contains(' ')
        || raw.starts_with("ssh-")
        || raw.starts_with("-----BEGIN")
        || raw.to_ascii_lowercase().contains("password")
        || (!raw.contains('/') && !raw.starts_with('~'))
    {
        return Err(
            "Secure mode: private key field is a file PATH only (e.g. ~/.ssh/id_ed25519).\n\
             Never paste passwords or public key text here.\n\
             Hostinger password stays in hPanel interactive login only — this app uses keys exclusively."
                .into(),
        );
    }
    let p = expand_tilde(raw);
    if !p.is_file() {
        return Err(format!(
            "Private key file not found: {}\n\
             Use a path like ~/.ssh/id_ed25519. Paste the matching .pub into Hostinger → SSH keys.",
            p.display()
        ));
    }
    check_key_permissions(&p)?;
    Ok(Some(p))
}

#[cfg(unix)]
fn check_key_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(path)
        .map_err(|e| format!("Cannot read key permissions: {e}"))?
        .permissions()
        .mode()
        & 0o777;
    // Must not be group/world readable or writable.
    if mode & 0o077 != 0 {
        return Err(format!(
            "Private key {} is too open (mode {:o}). Fix with:\n  chmod 600 {}",
            path.display(),
            mode,
            path.display()
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_key_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

/// Snapshot of local key readiness for the UI checklist.
#[derive(Debug, Clone)]
pub struct KeySecurityStatus {
    pub has_public_key: bool,
    pub public_key_path: Option<PathBuf>,
    pub private_key_ok: bool,
    pub private_key_path: Option<PathBuf>,
    pub private_key_error: Option<String>,
    pub notes: Vec<String>,
}

pub fn key_security_status(identity_file: &str) -> KeySecurityStatus {
    let pubs = local_public_keys();
    let has_public_key = !pubs.is_empty();
    let public_key_path = pubs.first().map(|(p, _)| p.clone());
    let mut notes = vec![
        "This app never stores or transmits Hostinger panel passwords.".into(),
        "Publish uses SSH public-key auth only (password login disabled in the client).".into(),
    ];
    if !has_public_key {
        notes.push("No ~/.ssh/*.pub found — generate: ssh-keygen -t ed25519 -a 100".into());
    } else {
        notes.push("Add the public key in hPanel → SSH Access → SSH keys (never the private key).".into());
    }
    let (private_key_ok, private_key_path, private_key_error) =
        match validate_identity_path(identity_file) {
            Ok(Some(p)) => (true, Some(p), None),
            Ok(None) => {
                // Empty: check default id_ed25519 / id_rsa
                let home = std::env::var("HOME").ok().map(PathBuf::from);
                let defaults = ["id_ed25519", "id_rsa", "id_ecdsa"];
                let mut found = None;
                if let Some(h) = home {
                    for name in defaults {
                        let p = h.join(".ssh").join(name);
                        if p.is_file() {
                            if let Err(e) = check_key_permissions(&p) {
                                return KeySecurityStatus {
                                    has_public_key,
                                    public_key_path,
                                    private_key_ok: false,
                                    private_key_path: Some(p),
                                    private_key_error: Some(e),
                                    notes,
                                };
                            }
                            found = Some(p);
                            break;
                        }
                    }
                }
                if found.is_some() {
                    notes.push("Using default key from ~/.ssh (no path set).".into());
                    (true, found, None)
                } else {
                    notes.push("Set a private key path or generate one with ssh-keygen.".into());
                    (false, None, Some("No default private key found in ~/.ssh".into()))
                }
            }
            Err(e) => (false, None, Some(e)),
        };
    KeySecurityStatus {
        has_public_key,
        public_key_path,
        private_key_ok,
        private_key_path,
        private_key_error,
        notes,
    }
}

/// Ensure project `.gitignore` ignores publish config (host/user metadata).
pub fn ensure_publish_gitignore(project_root: &Path) -> Result<(), String> {
    let gi = project_root.join(".gitignore");
    let entry = "mor-publish.toml";
    if gi.is_file() {
        let raw = std::fs::read_to_string(&gi).map_err(|e| e.to_string())?;
        if raw.lines().any(|l| l.trim() == entry || l.trim() == "/mor-publish.toml") {
            return Ok(());
        }
        let mut out = raw;
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("\n# SSH publish target (host/user — do not commit)\n");
        out.push_str(entry);
        out.push('\n');
        std::fs::write(&gi, out).map_err(|e| e.to_string())?;
    } else {
        std::fs::write(
            &gi,
            format!("# SSH publish target (host/user — do not commit)\n{entry}\n"),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn config_path(project_root: &Path) -> PathBuf {
    project_root.join(CONFIG_FILE)
}

/// Load config from the project folder, or defaults if missing/invalid.
pub fn load_config(project_root: &Path) -> PublishConfig {
    let path = config_path(project_root);
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return PublishConfig::default();
    };
    // Hard reject any attempt to keep passwords in the project file.
    for line in raw.lines() {
        let t = line.trim().to_ascii_lowercase();
        if t.starts_with('#') {
            continue;
        }
        if t.starts_with("password") || t.contains("password =") || t.contains("passphrase") {
            log::warn!("Ignoring mor-publish.toml: password/passphrase fields are not allowed");
            return PublishConfig::default();
        }
    }
    toml::from_str(&raw).unwrap_or_else(|_| PublishConfig::default())
}

/// Write a clean TOML (with comments) so users can also edit it by hand.
pub fn save_config(project_root: &Path, cfg: &PublishConfig) -> Result<PathBuf, String> {
    cfg.validate()?;
    let _ = ensure_publish_gitignore(project_root);
    let path = config_path(project_root);
    let identity_line = if cfg.identity_file.trim().is_empty() {
        "# identity_file = \"~/.ssh/id_ed25519\"\n".to_string()
    } else {
        format!(
            "identity_file = \"{}\"\n",
            escape_toml_str(cfg.identity_file.trim())
        )
    };
    let body = format!(
        r#"# mor-publish — SSH publish target (keys only, no passwords).
# Map from Hostinger hPanel → Advanced → SSH Access:
#   host  = IP
#   port  = Port (often 65002)
#   user  = Username
# SECURITY: never put Password / passphrase here. Auth is publickey only.
# Add the matching .pub key in hPanel → SSH keys → Add SSH key.
host = "{host}"
port = {port}
user = "{user}"
remote_dir = "{remote}"
{identity}
# Extra excludes (rsync patterns). Always excluded automatically:
#   .git, node_modules, mor-publish.toml, .env
#   wiki/, wordpress/, mediawiki/, wp/, blog/  (subdomain / CMS installs)
#   any top-level folder with LocalSettings.php or wp-config.php
#   paths listed in .morignore
{exclude}
# DANGEROUS: also upload well-known protect dirs (wiki/, wordpress/, …).
# Marker-based installs and .morignore still stay excluded.
sync_protected = {sync_protected}

# Delete remote files missing locally. Off by default. Excluded/protect dirs
# are never deleted on the remote (rsync leaves them alone).
delete = {delete}
"#,
        host = escape_toml_str(&cfg.host),
        port = cfg.port,
        user = escape_toml_str(&cfg.user),
        remote = escape_toml_str(&cfg.remote_dir),
        identity = identity_line,
        exclude = if cfg.exclude.is_empty() {
            "# exclude = [\"drafts/\"]\n".to_string()
        } else {
            format!(
                "exclude = [{}]\n",
                cfg.exclude
                    .iter()
                    .map(|e| format!("\"{}\"", escape_toml_str(e)))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
        sync_protected = cfg.sync_protected,
        delete = cfg.delete,
    );
    std::fs::write(&path, body).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    Ok(path)
}

fn escape_toml_str(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Parse Hostinger-style CLI lines into connection fields.
///
/// Accepts forms like:
/// - `ssh -p 65002 u123@46.20.1.2`
/// - `ssh u123@host -p 65002`
/// - `u123@46.20.1.2` (port left default)
///
/// Does **not** accept or store passwords. Returns (host, user, port).
pub fn parse_ssh_command(input: &str) -> Result<(String, String, u16), String> {
    let s = input.trim();
    if s.is_empty() {
        return Err("Paste the Hostinger SSH command, e.g. ssh -p 65002 u123@1.2.3.4".into());
    }
    // Reject if it looks like a password or private key blob.
    if s.starts_with("-----BEGIN") || s.to_ascii_lowercase().contains("password") {
        return Err("That looks like a password or private key — paste only the ssh command.".into());
    }

    let tokens: Vec<&str> = s.split_whitespace().collect();
    let mut port: Option<u16> = None;
    let mut target: Option<&str> = None;

    let mut i = 0;
    while i < tokens.len() {
        let t = tokens[i];
        if t == "ssh" {
            i += 1;
            continue;
        }
        if t == "-p" || t == "-P" {
            let Some(p) = tokens.get(i + 1) else {
                return Err("Missing port after -p".into());
            };
            port = Some(
                p.parse()
                    .map_err(|_| format!("Invalid port: {p}"))?,
            );
            i += 2;
            continue;
        }
        if let Some(rest) = t.strip_prefix("-p") {
            if !rest.is_empty() {
                port = Some(
                    rest.parse()
                        .map_err(|_| format!("Invalid port: {rest}"))?,
                );
                i += 1;
                continue;
            }
        }
        // Skip other ssh flags (-i, -o, …) and their args when obvious.
        if t.starts_with('-') {
            // -o Option=value or -o Option value
            if t == "-o" || t == "-i" || t == "-F" || t == "-l" {
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if t.contains('@') {
            target = Some(t);
            i += 1;
            continue;
        }
        i += 1;
    }

    let Some(target) = target else {
        return Err(
            "Could not find user@host in that text. Expected e.g. ssh -p 65002 u123@46.20.1.2"
                .into(),
        );
    };
    let (user, host) = target
        .split_once('@')
        .ok_or_else(|| "Target must look like username@ip".to_string())?;
    if user.is_empty() || host.is_empty() {
        return Err("Username and host must both be non-empty".into());
    }
    // Strip trailing junk Hostinger sometimes copies (quotes, punctuation).
    let host = host.trim_matches(|c: char| c == '"' || c == '\'' || c == ';' || c == ',');
    let user = user.trim_matches(|c: char| c == '"' || c == '\'');
    Ok((
        host.to_string(),
        user.to_string(),
        port.unwrap_or(65002),
    ))
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    if path == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home);
        }
    }
    PathBuf::from(path)
}

/// `ssh …` argv fragment used by rsync `-e` (one string).
/// Always keys-only: no password prompts, no keyboard-interactive auth.
pub fn ssh_remote_shell(cfg: &PublishConfig) -> String {
    let mut parts = vec![format!("ssh -p {}", cfg.port)];
    for opt in SECURE_SSH_OPTS {
        parts.push(format!("-o {opt}"));
    }
    if !cfg.identity_file.trim().is_empty() {
        let id = expand_tilde(cfg.identity_file.trim());
        parts.push(format!("-i {}", shell_quote(&id.display().to_string())));
        parts.push("-o IdentitiesOnly=yes".into());
    }
    parts.join(" ")
}

fn apply_secure_ssh_args(cmd: &mut Command, cfg: &PublishConfig) {
    cmd.arg("-p").arg(cfg.port.to_string());
    for opt in SECURE_SSH_OPTS {
        cmd.arg("-o").arg(*opt);
    }
    // Accept new host keys on first connect (TOFU); still refuse MITM on change.
    cmd.arg("-o").arg("StrictHostKeyChecking=accept-new");
    if !cfg.identity_file.trim().is_empty() {
        let id = expand_tilde(cfg.identity_file.trim());
        cmd.arg("-i").arg(id).arg("-o").arg("IdentitiesOnly=yes");
    }
}

fn shell_quote(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || "/._-@:~".contains(c))
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// Build rsync argv (program first). Pure — unit-tested.
pub fn rsync_args(cfg: &PublishConfig, project: &Path, dry_run: bool) -> Vec<String> {
    let mut args = vec![
        "rsync".into(),
        "-rltzv".into(),
        "--progress".into(),
        "-e".into(),
        ssh_remote_shell(cfg),
    ];
    if dry_run {
        args.push("--dry-run".into());
    }
    if cfg.delete {
        args.push("--delete".into());
    }
    let excludes = publish_excludes(project, &cfg.exclude, cfg.sync_protected);
    for pat in excludes {
        args.push(format!("--exclude={pat}"));
    }
    args.push(format!("{}/", project.display()));
    args.push(format!(
        "{}@{}:{}/",
        cfg.user,
        cfg.host,
        cfg.remote_dir.trim_end_matches('/')
    ));
    args
}

/// Probe SSH with keys only (never prompts for password).
pub fn test_connection(cfg: &PublishConfig) -> Result<String, String> {
    cfg.validate()?;
    let mut cmd = Command::new("ssh");
    apply_secure_ssh_args(&mut cmd, cfg);
    cmd.arg(format!("{}@{}", cfg.user.trim(), cfg.host.trim()))
        .arg("echo mor-publish-ok && pwd");
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to run ssh: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() && stdout.contains("mor-publish-ok") {
        Ok(format!(
            "✓ Secure SSH OK (publickey only) — {}@{}:{}\n{}",
            cfg.user,
            cfg.host,
            cfg.port,
            stdout.trim()
        ))
    } else {
        Err(format!(
            "SSH failed in secure mode (password login is disabled on purpose).\n\
             1. Generate a key if needed: ssh-keygen -t ed25519 -a 100\n\
             2. Copy the .pub line into hPanel → SSH Access → SSH keys → Add SSH key\n\
             3. Set private key PATH in this dialog (e.g. ~/.ssh/id_ed25519)\n\
             4. Test again\n\n{stderr}{stdout}"
        ))
    }
}

/// Default public keys the user can paste into Hostinger.
pub fn local_public_keys() -> Vec<(PathBuf, String)> {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return Vec::new(),
    };
    let ssh = home.join(".ssh");
    let mut out = Vec::new();
    for name in ["id_ed25519.pub", "id_rsa.pub", "id_ecdsa.pub"] {
        let p = ssh.join(name);
        if let Ok(s) = std::fs::read_to_string(&p) {
            let t = s.trim().to_string();
            if !t.is_empty() {
                out.push((p, t));
            }
        }
    }
    // Also list other *.pub keys (project-specific) — first 8.
    if let Ok(rd) = std::fs::read_dir(&ssh) {
        for ent in rd.flatten() {
            let p = ent.path();
            if p.extension().and_then(|e| e.to_str()) != Some("pub") {
                continue;
            }
            if out.iter().any(|(q, _)| q == &p) {
                continue;
            }
            if let Ok(s) = std::fs::read_to_string(&p) {
                let t = s.trim().to_string();
                if !t.is_empty() {
                    out.push((p, t));
                }
            }
            if out.len() >= 8 {
                break;
            }
        }
    }
    out
}

/// Run rsync (optionally dry-run). Returns combined stdout/stderr text.
pub fn run_publish(project: &Path, cfg: &PublishConfig, dry_run: bool) -> Result<String, String> {
    cfg.validate()?;
    if !project.is_dir() {
        return Err(format!("Project folder not found: {}", project.display()));
    }

    // Prefer the companion CLI when available (same export + rsync pipeline).
    if which_mor_publish().is_some() {
        return run_mor_publish_cli(project, dry_run);
    }

    let summary = protect_summary(project, cfg.sync_protected);
    let args = rsync_args(cfg, project, dry_run);
    let mut log = format!("{summary}\n→ {}\n\n", args.join(" "));

    let output = Command::new(&args[0])
        .args(&args[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| {
            format!(
                "Failed to run rsync ({e}). Install it with: pacman -S rsync / apt install rsync"
            )
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stdout.is_empty() {
        log.push_str(&stdout);
        if !log.ends_with('\n') {
            log.push('\n');
        }
    }
    if !stderr.is_empty() {
        log.push_str(&stderr);
        if !log.ends_with('\n') {
            log.push('\n');
        }
    }

    if output.status.success() {
        if dry_run {
            log.push_str("\n✓ Dry run complete — nothing was uploaded.\n");
        } else {
            log.push_str(&format!(
                "\n✓ Published to {}@{}:{}\n",
                cfg.user, cfg.host, cfg.remote_dir
            ));
        }
        Ok(log)
    } else {
        log.push_str(&format!("\n✗ rsync exited with {}\n", output.status));
        Err(log)
    }
}

fn which_mor_publish() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        let cargo = PathBuf::from(home).join(".cargo/bin/mor-publish");
        if cargo.is_file() {
            return Some(cargo);
        }
    }
    let output = Command::new("sh")
        .args(["-c", "command -v mor-publish"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

fn run_mor_publish_cli(project: &Path, dry_run: bool) -> Result<String, String> {
    let bin = which_mor_publish().ok_or_else(|| "mor-publish not found".to_string())?;
    let mut cmd = Command::new(&bin);
    cmd.arg("--project").arg(project);
    if dry_run {
        cmd.arg("--dry-run");
    }
    let mut log = format!(
        "→ {} --project {}{}\n\n",
        bin.display(),
        project.display(),
        if dry_run { " --dry-run" } else { "" }
    );
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to run mor-publish: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    log.push_str(&stdout);
    if !stderr.is_empty() {
        if !log.ends_with('\n') {
            log.push('\n');
        }
        log.push_str(&stderr);
    }
    if output.status.success() {
        Ok(log)
    } else {
        log.push_str(&format!("\n✗ mor-publish exited with {}\n", output.status));
        Err(log)
    }
}

/// Open the user's system terminal (not an in-app shell), optionally running
/// an `ssh user@host -p port` command for Hostinger-style access.
///
/// Tries common Linux terminals; never embeds a PTY in the editor.
pub fn open_system_terminal(ssh_cmd: Option<&str>) -> Result<(), String> {
    let ssh = ssh_cmd.map(str::trim).filter(|s| !s.is_empty());
    // Prefer desktop defaults, then fall back through common emulators.
    let candidates: &[(&str, &[&str])] = &[
        ("x-terminal-emulator", &["-e"]),
        ("gnome-terminal", &["--"]),
        ("kgx", &["-e"]),
        ("konsole", &["-e"]),
        ("xfce4-terminal", &["-e"]),
        ("mate-terminal", &["-e"]),
        ("xterm", &["-e"]),
    ];
    for (bin, prefix) in candidates {
        if which_bin(bin).is_none() {
            continue;
        }
        let mut cmd = Command::new(bin);
        if let Some(ssh_line) = ssh {
            for p in *prefix {
                cmd.arg(p);
            }
            // Run via sh so a single string with spaces works for all terminals.
            cmd.arg("sh").arg("-c").arg(format!("{ssh_line}; exec bash"));
        }
        match cmd.spawn() {
            Ok(_) => return Ok(()),
            Err(e) => log::warn!("Could not spawn {bin}: {e}"),
        }
    }
    Err(
        "No system terminal found (tried x-terminal-emulator, gnome-terminal, konsole, xterm…)."
            .into(),
    )
}

/// Build `ssh -p PORT -i KEY user@host` for the Open Terminal button.
pub fn interactive_ssh_command(cfg: &PublishConfig) -> Result<String, String> {
    cfg.validate()?;
    let mut parts = vec!["ssh".to_string()];
    parts.push("-p".into());
    parts.push(cfg.port.to_string());
    if let Some(key) = validate_identity_path(cfg.identity_file.trim())? {
        parts.push("-i".into());
        parts.push(key.display().to_string());
    }
    parts.push(format!("{}@{}", cfg.user.trim(), cfg.host.trim()));
    Ok(parts.join(" "))
}

fn which_bin(name: &str) -> Option<PathBuf> {
    if let Ok(p) = std::env::var("PATH") {
        for dir in p.split(':') {
            let cand = Path::new(dir).join(name);
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rsync_args_protect_wiki_and_identity() {
        let dir = std::env::temp_dir().join(format!("mor_ssh_pub_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let cfg = PublishConfig {
            host: "1.2.3.4".into(),
            user: "u1234".into(),
            port: 65002,
            remote_dir: "domains/x.com/public_html/".into(),
            exclude: vec!["drafts/".into()],
            delete: false,
            sync_protected: false,
            identity_file: String::new(),
        };
        let args = rsync_args(&cfg, &dir, true);
        assert!(args.contains(&"--exclude=wiki/".to_string()), "{args:?}");
        let e = args.iter().find(|a| a.starts_with("ssh ")).unwrap();
        assert!(e.contains("BatchMode=yes"), "{e}");
        assert!(e.contains("PasswordAuthentication=no"), "{e}");
        assert!(args.iter().any(|a| a.contains("u1234@1.2.3.4:")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_rejects_empty_user() {
        let mut cfg = PublishConfig::default();
        cfg.host = "1.2.3.4".into();
        assert!(cfg.validate().is_err());
        cfg.user = "u1".into();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn validate_rejects_password_paste_as_key_path() {
        assert!(validate_identity_path("D85Z8W)XmS~secret").is_err());
        assert!(validate_identity_path("ssh-ed25519 AAAA stuff").is_err());
    }

    #[test]
    fn parse_hostinger_ssh_command() {
        let (h, u, p) = parse_ssh_command("ssh -p 65002 u67example@46.20.198.68").unwrap();
        assert_eq!(h, "46.20.198.68");
        assert_eq!(u, "u67example");
        assert_eq!(p, 65002);

        let (h2, u2, p2) = parse_ssh_command("ssh u99@host.example -p 22").unwrap();
        assert_eq!((h2, u2, p2), ("host.example".into(), "u99".into(), 22));
    }
}
