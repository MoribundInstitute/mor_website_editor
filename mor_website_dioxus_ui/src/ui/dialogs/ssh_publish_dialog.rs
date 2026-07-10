//! SSH Publish dialog — Hostinger-oriented setup + rsync publish.
//! Maps hPanel → Advanced → SSH Access (IP / Port / Username) and never
//! stores the panel Password — use SSH keys instead.

use crate::app::services::ssh_publish::{self, PublishConfig};
use crate::app::state::{ThemeState, WebsiteState};
use crate::ui::components::form::{MorCheckbox, MorTextInput};
use crate::ui::dialogs::modal::Modal;
use crate::utils::clipboard::copy_to_clipboard;
use dioxus::prelude::*;

fn cfg_from_fields(
    host: Signal<String>,
    user: Signal<String>,
    port: Signal<String>,
    remote_dir: Signal<String>,
    identity_file: Signal<String>,
    delete_remote: Signal<bool>,
    sync_protected: Signal<bool>,
) -> Result<PublishConfig, String> {
    let port_num: u16 = port()
        .trim()
        .parse()
        .map_err(|_| "Port must be a number (Hostinger often uses 65002).".to_string())?;
    Ok(PublishConfig {
        host: host().trim().to_string(),
        user: user().trim().to_string(),
        port: port_num,
        remote_dir: remote_dir().trim().to_string(),
        exclude: Vec::new(),
        delete: delete_remote(),
        sync_protected: sync_protected(),
        identity_file: identity_file().trim().to_string(),
    })
}

fn append_log(mut log: Signal<String>, chunk: impl AsRef<str>) {
    let mut l = log.write();
    l.push_str(chunk.as_ref());
}

fn with_disk_excludes(root: &std::path::Path, mut cfg: PublishConfig) -> PublishConfig {
    cfg.exclude = ssh_publish::load_config(root).exclude;
    cfg
}

fn kick_publish(
    dry_run: bool,
    website: WebsiteState,
    theme: ThemeState,
    host: Signal<String>,
    user: Signal<String>,
    port: Signal<String>,
    remote_dir: Signal<String>,
    identity_file: Signal<String>,
    delete_remote: Signal<bool>,
    sync_protected: Signal<bool>,
    skip_export: Signal<bool>,
    log: Signal<String>,
    mut busy: Signal<bool>,
    mut status: Signal<String>,
) {
    if busy() {
        return;
    }
    if !website.project.peek().is_open() {
        status.set("Open a website folder first (File → Open Website Folder…).".into());
        return;
    }
    let mut cfg = match cfg_from_fields(
        host,
        user,
        port,
        remote_dir,
        identity_file,
        delete_remote,
        sync_protected,
    ) {
        Ok(c) => c,
        Err(e) => {
            status.set(e);
            return;
        }
    };
    let root = website.project.peek().root.clone();
    cfg = with_disk_excludes(&root, cfg);
    if let Err(e) = ssh_publish::save_config(&root, &cfg) {
        status.set(e);
        return;
    }

    let do_export = !skip_export() && !dry_run;
    busy.set(true);
    status.set(if dry_run {
        "Running dry-run…".into()
    } else {
        "Publishing…".into()
    });
    append_log(
        log,
        if dry_run {
            "\n── Dry run ──\n"
        } else {
            "\n── Publish ──\n"
        },
    );

    spawn(async move {
        if do_export {
            let project = website.project.peek().clone();
            let config = theme.signals.to_config();
            match tokio::task::spawn_blocking(move || {
                if !project.is_open() {
                    return Err("no project open".to_string());
                }
                mor_website_core::website::export_theme_css(&project, &config)
                    .map(|p| p.display().to_string())
                    .map_err(|e| e.to_string())
            })
            .await
            {
                Ok(Ok(path)) => append_log(log, format!("Exported {path}\n")),
                Ok(Err(e)) => append_log(log, format!("Note: skipped theme export ({e})\n")),
                Err(e) => append_log(log, format!("Note: export task failed ({e})\n")),
            }
        }

        let result =
            tokio::task::spawn_blocking(move || ssh_publish::run_publish(&root, &cfg, dry_run))
                .await;

        match result {
            Ok(Ok(out)) => {
                let mut chunk = out;
                if !chunk.ends_with('\n') {
                    chunk.push('\n');
                }
                append_log(log, chunk);
                status.set(if dry_run {
                    "Dry run finished.".into()
                } else {
                    "Published successfully.".into()
                });
            }
            Ok(Err(out)) => {
                let mut chunk = out;
                if !chunk.ends_with('\n') {
                    chunk.push('\n');
                }
                append_log(log, chunk);
                status.set("Publish failed — see log.".into());
            }
            Err(e) => status.set(format!("Task failed: {e}")),
        }
        busy.set(false);
    });
}

fn kick_test(
    website: WebsiteState,
    host: Signal<String>,
    user: Signal<String>,
    port: Signal<String>,
    remote_dir: Signal<String>,
    identity_file: Signal<String>,
    delete_remote: Signal<bool>,
    sync_protected: Signal<bool>,
    log: Signal<String>,
    mut busy: Signal<bool>,
    mut status: Signal<String>,
) {
    if busy() {
        return;
    }
    let cfg = match cfg_from_fields(
        host,
        user,
        port,
        remote_dir,
        identity_file,
        delete_remote,
        sync_protected,
    ) {
        Ok(c) => c,
        Err(e) => {
            status.set(e);
            return;
        }
    };
    // Soft-validate remote_dir for test (optional) — only need host/user/port/key.
    if cfg.host.is_empty() || cfg.user.is_empty() {
        status.set("Fill IP and Username from hPanel first.".into());
        return;
    }
    busy.set(true);
    status.set("Testing SSH…".into());
    append_log(log, "\n── Test SSH ──\n");
    spawn(async move {
        let result = tokio::task::spawn_blocking(move || ssh_publish::test_connection(&cfg)).await;
        match result {
            Ok(Ok(out)) => {
                append_log(log, format!("{out}\n"));
                status.set("SSH connection OK.".into());
            }
            Ok(Err(out)) => {
                append_log(log, format!("{out}\n"));
                status.set("SSH failed — add your public key in hPanel.".into());
            }
            Err(e) => status.set(format!("Task failed: {e}")),
        }
        let _ = website;
        busy.set(false);
    });
}

#[component]
pub fn SshPublishDialog(mut open: Signal<bool>) -> Element {
    let website = use_context::<WebsiteState>();
    let theme = use_context::<ThemeState>();

    let mut host = use_signal(String::new);
    let mut user = use_signal(String::new);
    let mut port = use_signal(|| "65002".to_string());
    let mut remote_dir = use_signal(String::new);
    let mut identity_file = use_signal(String::new);
    let mut paste_cmd = use_signal(String::new);
    let mut delete_remote = use_signal(|| false);
    let mut sync_protected = use_signal(|| false);
    let mut skip_export = use_signal(|| false);
    let mut log = use_signal(String::new);
    let busy = use_signal(|| false);
    let mut status = use_signal(String::new);
    let mut loaded_for = use_signal(String::new);

    let pub_keys = use_hook(ssh_publish::local_public_keys);

    // Load config when the dialog opens for a project.
    use_effect(move || {
        if !open() {
            return;
        }
        let project = website.project.peek().clone();
        if !project.is_open() {
            return;
        }
        let root = project.root.display().to_string();
        if loaded_for() == root {
            return;
        }
        let cfg = ssh_publish::load_config(&project.root);
        host.set(cfg.host);
        user.set(cfg.user);
        port.set(cfg.port.to_string());
        remote_dir.set(cfg.remote_dir);
        identity_file.set(cfg.identity_file);
        delete_remote.set(cfg.delete);
        sync_protected.set(cfg.sync_protected);
        loaded_for.set(root);
        if log().is_empty() {
            let shield = mor_website_core::website::publish_protect::protect_summary(
                &project.root,
                cfg.sync_protected,
            );
            log.set(format!(
                "Easiest setup:\n\
                 1. Copy Hostinger’s command:  ssh -p 65002 u…@…ip…\n\
                 2. Paste it into “Paste: ssh -p …” and click Fill from command\n\
                 3. Add your public key in hPanel → SSH keys (not the password)\n\
                 4. Set private key path → Test SSH → Dry run → Publish\n\n\
                 {shield}\n"
            ));
        }
    });

    let project_open = website.project.read().is_open();
    let project_label = if project_open {
        website.project.read().root.display().to_string()
    } else {
        "(no website folder open)".into()
    };

    let first_pub = pub_keys.first().cloned();

    rsx! {
        Modal {
            open: open,
            title: "SSH Publish".to_string(),
            style: "width: 600px; height: 700px; max-width: 760px;".to_string(),
            on_close: move |_| open.set(false),

            div {
                style: "display: flex; flex-direction: column; gap: 12px; height: 100%; min-height: 0;",

                div { class: "editor-note", style: "margin: 0;",
                    p { class: "editor-note-title", "Website folder" }
                    p { class: "editor-note-body", style: "font-family: var(--editor-mono, monospace); font-size: 0.78rem; word-break: break-all;",
                        "{project_label}"
                    }
                }

                if !project_open {
                    div { class: "editor-note", style: "border-color: var(--editor-warning); background: rgba(210,153,34,0.06);",
                        p { class: "editor-note-title", style: "color: var(--editor-warning);", "No folder open" }
                        p { class: "editor-note-body",
                            "Use File → Open Website Folder… then return here to publish."
                        }
                    }
                }

                // Easiest path: paste Hostinger's one-liner
                div { class: "editor-note", style: "margin: 0;",
                    p { class: "editor-note-title", "Easiest: paste Hostinger SSH command" }
                    p { class: "editor-note-body", style: "font-size: 0.78rem; line-height: 1.4; margin-bottom: 8px;",
                        "In hPanel, copy the command like "
                        code { "ssh -p 65002 u…@…ip…" }
                        " and paste it below — we fill IP, port, and username. "
                        strong { "Still use SSH keys for login" }
                        " (the password prompt Hostinger mentions is only for manual terminal use)."
                    }
                    MorTextInput {
                        label: "Paste: ssh -p 65002 u…@…".to_string(),
                        value: paste_cmd(),
                        onchange: move |v| paste_cmd.set(v),
                    }
                    button {
                        class: "mor-btn",
                        style: "margin-top: 6px;",
                        onclick: move |_| {
                            match ssh_publish::parse_ssh_command(&paste_cmd()) {
                                Ok((h, u, p)) => {
                                    host.set(h.clone());
                                    user.set(u.clone());
                                    port.set(p.to_string());
                                    status.set(format!(
                                        "Filled from command: {u}@{h} port {p}"
                                    ));
                                    append_log(
                                        log,
                                        format!(
                                            "\nParsed Hostinger command → {u}@{h}:{p}\n"
                                        ),
                                    );
                                    paste_cmd.set(String::new());
                                }
                                Err(e) => status.set(e),
                            }
                        },
                        "Fill from command"
                    }
                }

                // Match Hostinger "SSH details" labels (editable after parse)
                MorTextInput {
                    label: "IP (or hostname)".to_string(),
                    value: host(),
                    onchange: move |v| host.set(v),
                }
                div { style: "display: grid; grid-template-columns: 1fr 100px; gap: 10px;",
                    MorTextInput {
                        label: "Username".to_string(),
                        value: user(),
                        onchange: move |v| user.set(v),
                    }
                    MorTextInput {
                        label: "Port".to_string(),
                        value: port(),
                        onchange: move |v| port.set(v),
                    }
                }
                MorTextInput {
                    label: "Remote directory (e.g. domains/yoursite.com/public_html)".to_string(),
                    value: remote_dir(),
                    onchange: move |v| remote_dir.set(v),
                }
                MorTextInput {
                    label: "Private key PATH only (e.g. ~/.ssh/id_ed25519) — not password, not .pub text".to_string(),
                    value: identity_file(),
                    onchange: move |v| identity_file.set(v),
                }
                if !identity_file().is_empty()
                    && !identity_file().contains('/')
                    && !identity_file().starts_with('~')
                {
                    p {
                        style: "margin: 0; font-size: 0.75rem; color: var(--editor-warning, #d29922);",
                        "That doesn’t look like a file path. Clear the field or set ~/.ssh/id_ed25519"
                    }
                }

                // Security-first checklist (keys only)
                {
                    let sec = ssh_publish::key_security_status(&identity_file());
                    let ok_pub = sec.has_public_key;
                    let ok_priv = sec.private_key_ok;
                    rsx! {
                        div {
                            class: "editor-note",
                            style: "margin: 0; border-color: color-mix(in srgb, var(--editor-accent, #6d8fb8) 50%, transparent);",
                            p { class: "editor-note-title", "Secure setup (keys only)" }
                            p { class: "editor-note-body", style: "font-size: 0.78rem; line-height: 1.45; margin-bottom: 8px;",
                                "This app "
                                strong { "never stores passwords" }
                                " and "
                                strong { "disables password SSH login" }
                                " for publish (publickey only). Hostinger’s Password field is only for manual terminal login."
                            }
                            ul {
                                style: "margin: 0 0 8px 1.1rem; padding: 0; font-size: 0.78rem; line-height: 1.5; color: var(--fg-muted);",
                                li {
                                    if ok_pub { "✓ Local public key found" } else { "○ Generate a key: ssh-keygen -t ed25519 -a 100" }
                                }
                                li {
                                    "○ Add that .pub in hPanel → SSH keys → Add SSH key (not the private key)"
                                }
                                li {
                                    if ok_priv { "✓ Private key path OK (owner-only permissions)" }
                                    else if let Some(err) = sec.private_key_error.clone() {
                                        "✗ {err}"
                                    } else {
                                        "○ Set private key PATH below (e.g. ~/.ssh/id_ed25519)"
                                    }
                                }
                                li { "○ Test SSH, then Dry run, then Publish" }
                            }
                            if let Some((path, key)) = first_pub.clone() {
                                p { class: "editor-note-body", style: "font-family: var(--editor-mono, monospace); font-size: 0.68rem; word-break: break-all; opacity: 0.9; margin-bottom: 8px;",
                                    "{path.display()}"
                                }
                                div { style: "display: flex; flex-wrap: wrap; gap: 6px;",
                                    button {
                                        class: "mor-btn",
                                        title: "Copy public key only — safe to paste into Hostinger",
                                        onclick: move |_| {
                                            copy_to_clipboard(key.clone());
                                            status.set("Public key copied — paste into hPanel → SSH keys → Add SSH key.".into());
                                        },
                                        "Copy public key"
                                    }
                                    button {
                                        class: "mor-btn",
                                        title: "Set private key path (file path only, never paste key material)",
                                        onclick: move |_| {
                                            let s = path.display().to_string();
                                            let s = s
                                                .strip_suffix(".pub")
                                                .unwrap_or(s.as_str())
                                                .to_string();
                                            if let Ok(home) = std::env::var("HOME") {
                                                if let Some(rest) = s.strip_prefix(&format!("{home}/")) {
                                                    identity_file.set(format!("~/{rest}"));
                                                } else {
                                                    identity_file.set(s);
                                                }
                                            } else {
                                                identity_file.set(s);
                                            }
                                            status.set("Private key path set.".into());
                                        },
                                        "Use this key path"
                                    }
                                    button {
                                        class: "mor-btn",
                                        title: "Clear private key field",
                                        onclick: move |_| {
                                            identity_file.set(String::new());
                                            status.set("Private key field cleared.".into());
                                        },
                                        "Clear key path"
                                    }
                                }
                            } else {
                                p { class: "editor-note-body", style: "font-size: 0.78rem;",
                                    "No ~/.ssh/*.pub found. Generate one:  "
                                    code { "ssh-keygen -t ed25519 -a 100 -C \"you@example.com\"" }
                                }
                            }
                        }
                    }
                }

                div { style: "display: flex; flex-direction: column; gap: 6px;",
                    MorCheckbox {
                        label: "Delete remote files missing locally (protected CMS dirs stay)".to_string(),
                        checked: delete_remote(),
                        onchange: move |v| delete_remote.set(v),
                    }
                    MorCheckbox {
                        label: "Also upload wiki/wordpress/… protect dirs (dangerous)".to_string(),
                        checked: sync_protected(),
                        onchange: move |v| sync_protected.set(v),
                    }
                    MorCheckbox {
                        label: "Skip mor-theme.css re-export before upload".to_string(),
                        checked: skip_export(),
                        onchange: move |v| skip_export.set(v),
                    }
                }

                div {
                    style: "display: flex; flex-wrap: wrap; gap: 8px; align-items: center;",
                    button {
                        class: "mor-btn",
                        disabled: busy() || !project_open,
                        onclick: move |_| {
                            if !website.project.peek().is_open() {
                                status.set("Open a website folder first.".into());
                                return;
                            }
                            match cfg_from_fields(
                                host, user, port, remote_dir, identity_file,
                                delete_remote, sync_protected,
                            ) {
                                Ok(cfg) => {
                                    let root = website.project.peek().root.clone();
                                    let cfg = with_disk_excludes(&root, cfg);
                                    match ssh_publish::save_config(&root, &cfg) {
                                        Ok(path) => {
                                            status.set(format!("Saved {}", path.display()));
                                            append_log(log, format!("\nSaved {}\n", path.display()));
                                        }
                                        Err(e) => status.set(e),
                                    }
                                }
                                Err(e) => status.set(e),
                            }
                        },
                        "Save settings"
                    }
                    button {
                        class: "mor-btn",
                        disabled: busy(),
                        title: "ssh BatchMode test (keys only, no password prompt)",
                        onclick: move |_| {
                            kick_test(
                                website, host, user, port, remote_dir, identity_file,
                                delete_remote, sync_protected, log, busy, status,
                            );
                        },
                        "Test SSH"
                    }
                    button {
                        class: "mor-btn",
                        disabled: busy(),
                        title: "Open your system terminal with ssh user@host (not an in-app shell)",
                        onclick: move |_| {
                            match cfg_from_fields(
                                host, user, port, remote_dir, identity_file,
                                delete_remote, sync_protected,
                            ) {
                                Ok(cfg) => match ssh_publish::interactive_ssh_command(&cfg) {
                                    Ok(cmd) => match ssh_publish::open_system_terminal(Some(&cmd)) {
                                        Ok(()) => {
                                            status.set(format!("Opened system terminal: {cmd}"));
                                            append_log(log, format!("\nSystem terminal → {cmd}\n"));
                                        }
                                        Err(e) => status.set(e),
                                    },
                                    Err(e) => status.set(e),
                                },
                                Err(e) => status.set(e),
                            }
                        },
                        "Open system terminal"
                    }
                    button {
                        class: "mor-btn",
                        disabled: busy() || !project_open,
                        onclick: move |_| {
                            kick_publish(
                                true, website, theme, host, user, port, remote_dir,
                                identity_file, delete_remote, sync_protected, skip_export,
                                log, busy, status,
                            );
                        },
                        "Dry run"
                    }
                    button {
                        class: "mor-btn-primary",
                        disabled: busy() || !project_open,
                        onclick: move |_| {
                            kick_publish(
                                false, website, theme, host, user, port, remote_dir,
                                identity_file, delete_remote, sync_protected, skip_export,
                                log, busy, status,
                            );
                        },
                        if busy() { "Working…" } else { "Publish" }
                    }
                }

                if !status().is_empty() {
                    p {
                        style: "margin: 0; font-size: 0.82rem; color: var(--fg-muted, #999);",
                        "{status}"
                    }
                }

                div {
                    style: "flex: 1 1 auto; min-height: 120px; display: flex; flex-direction: column; gap: 4px;",
                    label { class: "editor-field-label", "Log" }
                    pre {
                        style: "flex: 1 1 auto; margin: 0; padding: 10px; overflow: auto; \
                                font-size: 0.72rem; line-height: 1.4; white-space: pre-wrap; word-break: break-word; \
                                background: var(--editor-bg-deep, #121212); color: var(--editor-text, #ddd); \
                                border: 1px solid var(--editor-border-soft, #333); border-radius: 6px; \
                                font-family: var(--editor-mono, ui-monospace, monospace);",
                        "{log}"
                    }
                }
            }
        }
    }
}
