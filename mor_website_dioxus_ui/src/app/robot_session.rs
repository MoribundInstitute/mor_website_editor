//! Robot Assist session handoff + project hot-reload when agents write files.

use dioxus::prelude::*;
use futures_util::stream::StreamExt;
use notify::{recommended_watcher, Event, EventKind, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

use crate::app::state::{ThemeState, WebsiteState};
use mor_website_core::config::ThemeConfig;
use mor_website_core::utils::robot_assist::{
    self, load_policy, save_policy, write_session, LiveBuffer, RobotPolicy, RobotTier,
    SessionSnapshot,
};

/// Persist UI policy and push a session snapshot.
pub fn sync_policy_from_ui(
    enabled: bool,
    tier: RobotTier,
    allow_delete: bool,
    project_path: Option<String>,
) {
    let policy = RobotPolicy {
        enabled,
        tier: if enabled && tier == RobotTier::Off {
            RobotTier::Theme
        } else if !enabled {
            RobotTier::Off
        } else {
            tier
        },
        project_path,
        allow_delete,
        updated_unix: 0,
    };
    if let Err(e) = save_policy(policy) {
        log::warn!("Failed to save robot assist policy: {e}");
    }
    if !enabled {
        robot_assist::clear_session();
    }
}

/// Write live session JSON for MCP `get_session`.
pub fn publish_session(
    theme: ThemeState,
    website: WebsiteState,
    live_buffer: Option<LiveBuffer>,
) {
    let policy = load_policy();
    if policy.effective_tier() == RobotTier::Off {
        robot_assist::clear_session();
        return;
    }

    let project = website.project.peek().clone();
    let project_root = if project.is_open() {
        Some(project.root.display().to_string())
    } else {
        policy.project_path.clone()
    };

    let workspace_toml_path = project_root.as_ref().map(|r| {
        let p = PathBuf::from(r).join("workspace.toml");
        p.display().to_string()
    });

    let active_preset = (*theme.active_preset.peek()).map(|s| s.to_string());

    let snap = SessionSnapshot {
        project_root,
        active_page: (website.current_page)(),
        workspace_toml_path,
        active_preset,
        tier: policy.effective_tier().as_str().to_string(),
        enable_ai_bridge: *theme.enable_ai_bridge.peek(),
        timestamp: 0,
        live_buffer,
    };
    if let Err(e) = write_session(snap) {
        log::warn!("Failed to write robot session: {e}");
    }
}

/// Keep session file updated whenever project / page / assist flags change.
pub fn use_robot_session_bridge(theme: ThemeState, website: WebsiteState) {
    use_effect(move || {
        let _ = (website.project)();
        let _ = (website.current_page)();
        let _ = (theme.active_preset)();
        let _ = (theme.enable_ai_bridge)();
        let _ = (theme.robot_tier)();
        publish_session(theme, website, None);
    });
}

enum ProjectWatchMsg {
    WorkspaceToml(String),
    PageTouched,
}

static PROJECT_TX: OnceLock<UnboundedSender<ProjectWatchMsg>> = OnceLock::new();

fn send_project_msg(msg: ProjectWatchMsg) {
    if let Some(tx) = PROJECT_TX.get() {
        let _ = tx.unbounded_send(msg);
    }
}

/// Watch open project for agent writes (workspace.toml + pages) when assist is on.
pub fn use_project_hot_reload(theme: ThemeState, website: WebsiteState) {
    let signals = theme.signals;
    let mut website_for_rx = website;

    let reload = use_coroutine(move |mut rx: UnboundedReceiver<ProjectWatchMsg>| async move {
        while let Some(msg) = rx.next().await {
            match msg {
                ProjectWatchMsg::WorkspaceToml(raw) => {
                    if let Ok(cfg) = toml::from_str::<ThemeConfig>(&raw) {
                        signals.apply_config(&cfg);
                        log::info!("Robot Assist: reloaded workspace.toml from disk");
                    }
                }
                ProjectWatchMsg::PageTouched => {
                    website_for_rx.bump_preview();
                }
            }
        }
    });

    use_effect(move || {
        let _ = PROJECT_TX.set(reload.tx());
        let project = (website.project)();
        let tier = (theme.robot_tier)();
        let enabled = *theme.enable_ai_bridge.read() || tier != "off";
        if !enabled || !project.is_open() {
            return;
        }
        spawn_project_watcher(project.root.clone());
    });
}

fn spawn_project_watcher(root: PathBuf) {
    static LAST_ROOT: OnceLock<std::sync::Mutex<Option<PathBuf>>> = OnceLock::new();
    let lock = LAST_ROOT.get_or_init(|| std::sync::Mutex::new(None));
    {
        let mut guard = lock.lock().unwrap_or_else(|e| e.into_inner());
        if guard.as_ref() == Some(&root) {
            return; // already watching this project
        }
        *guard = Some(root.clone());
    }

    thread::spawn(move || {
        let Ok(mut watcher) = recommended_watcher(move |result: Result<Event, notify::Error>| {
            let Ok(event) = result else { return };
            if !matches!(
                event.kind,
                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
            ) {
                return;
            }
            thread::sleep(Duration::from_millis(80));
            for path in &event.paths {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name == "workspace.toml" || name == "theme_config.toml" {
                    if let Ok(raw) = std::fs::read_to_string(path) {
                        send_project_msg(ProjectWatchMsg::WorkspaceToml(raw));
                    }
                } else if name.ends_with(".html")
                    || name.ends_with(".htm")
                    || name.ends_with(".php")
                    || name == "mor-theme.css"
                    || name == "mor-theme.js"
                {
                    send_project_msg(ProjectWatchMsg::PageTouched);
                }
            }
        }) else {
            return;
        };
        let _ = watcher.watch(&root, RecursiveMode::Recursive);
        log::info!("Robot Assist: watching project {:?}", root);
        loop {
            thread::sleep(Duration::from_secs(3600));
        }
    });
}
