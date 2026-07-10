//! Static map of CSS / JS / PHP includes reachable from one page.
//!
//! This is intentionally a lightweight text scan — enough to show a mindmap of
//! "what does this page pull in?" without executing PHP. It understands:
//!   * `require` / `include` (+ `_once`) with string paths and `__DIR__ . '…'`
//!   * `<link rel="stylesheet" href="…">` and bare `.css` link hrefs
//!   * `<script src="…">`
//!   * `$extraCss = […string…];` / `$extraScripts = […];` array literals
//!
//! External (http/https) assets are kept as separate nodes so the map still
//! shows Google Fonts etc., without pretending they live in the project.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetKind {
    Page,
    Include,
    Css,
    Js,
    ExternalCss,
    ExternalJs,
}

impl AssetKind {
    pub fn label(self) -> &'static str {
        match self {
            AssetKind::Page => "page",
            AssetKind::Include => "include",
            AssetKind::Css => "css",
            AssetKind::Js => "js",
            AssetKind::ExternalCss => "css (external)",
            AssetKind::ExternalJs => "js (external)",
        }
    }

    pub fn is_style(self) -> bool {
        matches!(self, AssetKind::Css | AssetKind::ExternalCss)
    }

    pub fn is_script(self) -> bool {
        matches!(self, AssetKind::Js | AssetKind::ExternalJs)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetNode {
    pub id: String,
    pub kind: AssetKind,
    /// Project-relative path (`css/looks.css`) or absolute URL for externals.
    pub path: String,
    /// Short label for the mindmap (filename for locals, host/path for URLs).
    pub label: String,
    /// For local assets: whether the file currently exists under the project root.
    pub exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetEdge {
    pub from: String,
    pub to: String,
    /// Human reason: "require", "stylesheet", "script", "extraCss", …
    pub via: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageAssetMap {
    pub root_page: String,
    pub nodes: Vec<AssetNode>,
    pub edges: Vec<AssetEdge>,
}

impl PageAssetMap {
    pub fn css_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|n| n.kind.is_style())
            .count()
    }

    pub fn js_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|n| n.kind.is_script())
            .count()
    }

    pub fn include_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|n| n.kind == AssetKind::Include)
            .count()
    }
}

/// Build the asset mindmap for `page_rel` under `project_root`.
pub fn map_page_assets(project_root: &Path, page_rel: &str) -> PageAssetMap {
    let page_rel = normalize_rel(page_rel);
    let mut map = PageAssetMap {
        root_page: page_rel.clone(),
        nodes: Vec::new(),
        edges: Vec::new(),
    };

    let page_path = project_root.join(&page_rel);
    let page_id = node_id(AssetKind::Page, &page_rel);
    map.nodes.push(AssetNode {
        id: page_id.clone(),
        kind: AssetKind::Page,
        path: page_rel.clone(),
        label: file_label(&page_rel),
        exists: page_path.is_file(),
    });

    if !page_path.is_file() {
        return map;
    }

    let mut seen_php: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, String)> = VecDeque::new(); // (rel, parent_id)
    queue.push_back((page_rel.clone(), page_id));
    seen_php.insert(page_rel);

    // Cap breadth so a runaway include tree (or MediaWiki slip-in) can't hang.
    let mut steps = 0usize;
    const MAX_STEPS: usize = 80;

    while let Some((rel, parent_id)) = queue.pop_front() {
        steps += 1;
        if steps > MAX_STEPS {
            break;
        }
        let abs = project_root.join(&rel);
        let Ok(contents) = std::fs::read_to_string(&abs) else {
            continue;
        };
        let dir = Path::new(&rel)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(""));

        for inc in extract_php_includes(&contents) {
            let Some(resolved) = resolve_php_path(project_root, &dir, &inc) else {
                continue;
            };
            let child_id = node_id(AssetKind::Include, &resolved);
            ensure_node(
                &mut map,
                AssetNode {
                    id: child_id.clone(),
                    kind: AssetKind::Include,
                    path: resolved.clone(),
                    label: file_label(&resolved),
                    exists: project_root.join(&resolved).is_file(),
                },
            );
            push_edge(&mut map, &parent_id, &child_id, "require");
            if seen_php.insert(resolved.clone()) {
                queue.push_back((resolved, child_id));
            }
        }

        for href in extract_stylesheet_hrefs(&contents) {
            add_asset_link(
                &mut map,
                project_root,
                &dir,
                &parent_id,
                &href,
                true,
                "stylesheet",
            );
        }
        for src in extract_script_srcs(&contents) {
            add_asset_link(
                &mut map,
                project_root,
                &dir,
                &parent_id,
                &src,
                false,
                "script",
            );
        }
        for href in extract_php_string_array(&contents, "extraCss") {
            add_asset_link(
                &mut map,
                project_root,
                &dir,
                &parent_id,
                &href,
                true,
                "extraCss",
            );
        }
        for src in extract_php_string_array(&contents, "extraScripts") {
            add_asset_link(
                &mut map,
                project_root,
                &dir,
                &parent_id,
                &src,
                false,
                "extraScripts",
            );
        }
    }

    map
}

fn add_asset_link(
    map: &mut PageAssetMap,
    project_root: &Path,
    current_dir: &Path,
    parent_id: &str,
    raw: &str,
    is_css: bool,
    via: &str,
) {
    let raw = raw.trim();
    if raw.is_empty() {
        return;
    }
    if is_external(raw) {
        let kind = if is_css {
            AssetKind::ExternalCss
        } else {
            AssetKind::ExternalJs
        };
        let id = node_id(kind, raw);
        ensure_node(
            map,
            AssetNode {
                id: id.clone(),
                kind,
                path: raw.to_string(),
                label: external_label(raw),
                exists: true,
            },
        );
        push_edge(map, parent_id, &id, via);
        return;
    }
    let Some(resolved) = resolve_web_path(project_root, current_dir, raw) else {
        return;
    };
    let kind = if is_css { AssetKind::Css } else { AssetKind::Js };
    let id = node_id(kind, &resolved);
    ensure_node(
        map,
        AssetNode {
            id: id.clone(),
            kind,
            path: resolved.clone(),
            label: file_label(&resolved),
            exists: project_root.join(&resolved).is_file(),
        },
    );
    push_edge(map, parent_id, &id, via);
}

fn ensure_node(map: &mut PageAssetMap, node: AssetNode) {
    if !map.nodes.iter().any(|n| n.id == node.id) {
        map.nodes.push(node);
    }
}

fn push_edge(map: &mut PageAssetMap, from: &str, to: &str, via: &str) {
    if map
        .edges
        .iter()
        .any(|e| e.from == from && e.to == to && e.via == via)
    {
        return;
    }
    map.edges.push(AssetEdge {
        from: from.to_string(),
        to: to.to_string(),
        via: via.to_string(),
    });
}

fn node_id(kind: AssetKind, path: &str) -> String {
    format!("{}:{}", kind.label(), path)
}

fn normalize_rel(s: &str) -> String {
    s.trim()
        .trim_start_matches("./")
        .trim_start_matches('/')
        .replace('\\', "/")
}

fn file_label(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

fn external_label(url: &str) -> String {
    // fonts.googleapis.com/… → fonts.googleapis.com
    let without = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let host = without.split('/').next().unwrap_or(without);
    if host.len() > 28 {
        format!("{}…", &host[..28])
    } else {
        host.to_string()
    }
}

fn is_external(s: &str) -> bool {
    let t = s.trim();
    t.starts_with("http://")
        || t.starts_with("https://")
        || t.starts_with("//")
        || t.starts_with("data:")
}

/// Resolve a PHP include path to a project-relative path.
fn resolve_php_path(project_root: &Path, current_dir: &Path, raw: &str) -> Option<String> {
    let raw = raw.trim().trim_matches(|c| c == '"' || c == '\'');
    if raw.is_empty() || raw.contains('$') {
        return None;
    }
    let candidate = if raw.starts_with('/') {
        // Absolute filesystem path that happens to sit under the project.
        let p = Path::new(raw);
        p.strip_prefix(project_root).ok().map(|r| r.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(raw.trim_start_matches('/')))
    } else {
        current_dir.join(raw)
    };
    Some(normalize_rel(&candidate.to_string_lossy()))
}

/// Resolve a web href/src (`/css/looks.css`, `../css/x.css`, `css/x.css`).
fn resolve_web_path(project_root: &Path, current_dir: &Path, raw: &str) -> Option<String> {
    let raw = raw.split(['?', '#']).next().unwrap_or(raw).trim();
    if raw.is_empty() {
        return None;
    }
    let candidate = if raw.starts_with('/') {
        PathBuf::from(raw.trim_start_matches('/'))
    } else {
        current_dir.join(raw)
    };
    // Collapse `a/../b` style segments without requiring the file to exist.
    let collapsed = collapse_dots(&candidate);
    let rel = normalize_rel(&collapsed.to_string_lossy());
    // Prefer existence check only for the `exists` flag; still map missing files.
    let _ = project_root;
    Some(rel)
}

fn collapse_dots(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// `require __DIR__ . '/includes/shell-start.php';` and friends.
fn extract_php_includes(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    // Strip block comments roughly so commented requires don't poison the map.
    let stripped = strip_php_comments(src);
    for line in stripped.lines() {
        let t = line.trim();
        if t.starts_with("//") || t.starts_with('#') {
            continue;
        }
        let lower = t.to_ascii_lowercase();
        if !(lower.contains("require") || lower.contains("include")) {
            continue;
        }
        // __DIR__ . 'path'  or  __DIR__."path"
        if let Some(rest) = find_after_ci(t, "__DIR__") {
            if let Some(path) = first_quoted(rest) {
                out.push(path);
                continue;
            }
        }
        // require 'path' / require("path") / require_once "path"
        if let Some(path) = first_quoted(t) {
            // Skip pure language constructs that quote something else by accident.
            if path.ends_with(".php")
                || path.ends_with(".inc")
                || path.ends_with(".html")
                || path.ends_with(".htm")
            {
                out.push(path);
            }
        }
    }
    out
}

fn extract_stylesheet_hrefs(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let stripped = strip_php_comments(src);
    // <link … href="…css…">
    for cap in find_attr_values(&stripped, "href") {
        let lower = cap.to_ascii_lowercase();
        if lower.contains(".css") || lower.contains("fonts.googleapis") || lower.contains("fonts.bunny")
        {
            // Prefer stylesheet links; still accept bare .css hrefs in link tags.
            out.push(cap);
        }
    }
    out
}

fn extract_script_srcs(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let stripped = strip_php_comments(src);
    // Only count src= that sit near a <script (same line or obvious script tag).
    for line in stripped.lines() {
        let lower = line.to_ascii_lowercase();
        if !lower.contains("<script") && !lower.contains("script ") {
            // Still allow plain src= on a line with .js
            if !(lower.contains("src=") && lower.contains(".js")) {
                continue;
            }
        }
        for cap in find_attr_values(line, "src") {
            let l = cap.to_ascii_lowercase();
            if l.contains(".js") || l.starts_with("http") || l.starts_with("//") {
                out.push(cap);
            }
        }
    }
    out
}

/// `$extraCss = ['/a.css', "/b.css"];`
fn extract_php_string_array(src: &str, var_name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let needle = format!("${var_name}");
    let stripped = strip_php_comments(src);
    let Some(idx) = stripped.find(&needle) else {
        return out;
    };
    let after = &stripped[idx + needle.len()..];
    let Some(bracket) = after.find('[') else {
        return out;
    };
    let rest = &after[bracket + 1..];
    let Some(end) = rest.find(']') else {
        return out;
    };
    let body = &rest[..end];
    // Pull every quoted string in the array body.
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' || c == '\'' {
            let quote = c;
            let mut s = String::new();
            while let Some(ch) = chars.next() {
                if ch == '\\' {
                    if let Some(n) = chars.next() {
                        s.push(n);
                    }
                    continue;
                }
                if ch == quote {
                    break;
                }
                s.push(ch);
            }
            if !s.is_empty() {
                out.push(s);
            }
        }
    }
    out
}

fn find_attr_values(src: &str, attr: &str) -> Vec<String> {
    let mut out = Vec::new();
    let lower_src = src.to_ascii_lowercase();
    let attr_l = attr.to_ascii_lowercase();
    let mut search_from = 0;
    while let Some(rel) = lower_src[search_from..].find(&attr_l) {
        let abs = search_from + rel;
        let after_attr = &src[abs + attr.len()..];
        let trimmed = after_attr.trim_start();
        if !trimmed.starts_with('=') {
            search_from = abs + attr.len();
            continue;
        }
        let after_eq = trimmed[1..].trim_start();
        if let Some(path) = first_quoted(after_eq) {
            out.push(path);
        }
        search_from = abs + attr.len();
    }
    out
}

fn first_quoted(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '"' || c == '\'' {
            let quote = c;
            i += 1;
            let start = i;
            while i < bytes.len() {
                let ch = bytes[i] as char;
                if ch == '\\' {
                    i += 2;
                    continue;
                }
                if ch == quote {
                    return Some(s[start..i].to_string());
                }
                i += 1;
            }
            return None;
        }
        i += 1;
    }
    None
}

fn find_after_ci<'a>(hay: &'a str, needle: &str) -> Option<&'a str> {
    let h = hay.to_ascii_lowercase();
    let n = needle.to_ascii_lowercase();
    h.find(&n).map(|i| &hay[i + needle.len()..])
}

/// Cheap strip of `//`, `#`, and `/* */` comments — good enough for scans.
fn strip_php_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut in_block = false;
    let mut in_str: Option<u8> = None;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = in_str {
            out.push(b as char);
            if b == b'\\' && i + 1 < bytes.len() {
                out.push(bytes[i + 1] as char);
                i += 2;
                continue;
            }
            if b == q {
                in_str = None;
            }
            i += 1;
            continue;
        }
        if in_block {
            if b == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                in_block = false;
                i += 2;
            } else {
                // preserve newlines so line-based filters still work
                if b == b'\n' {
                    out.push('\n');
                }
                i += 1;
            }
            continue;
        }
        if b == b'"' || b == b'\'' {
            in_str = Some(b);
            out.push(b as char);
            i += 1;
            continue;
        }
        if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if b == b'#' {
            // PHP #-comments, but not inside HTML colors — only at line-ish starts
            // we already skip full-line # above via trim; keep simple.
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            in_block = true;
            i += 2;
            continue;
        }
        out.push(b as char);
        i += 1;
    }
    out
}

/// Positions for a radial mindmap layout (editor units, ~960×640 viewBox).
#[derive(Debug, Clone, Copy)]
pub struct MapPoint {
    pub x: f64,
    pub y: f64,
}

/// Lay out nodes: page at center, includes on an inner ring, CSS left/outer,
/// JS right/outer — same spirit as the code-gui-manager dependency circle.
pub fn layout_mindmap(map: &PageAssetMap, cx: f64, cy: f64) -> HashMap<String, MapPoint> {
    let mut pos = HashMap::new();
    let page_id = node_id(AssetKind::Page, &map.root_page);
    pos.insert(page_id, MapPoint { x: cx, y: cy });

    let includes: Vec<_> = map
        .nodes
        .iter()
        .filter(|n| n.kind == AssetKind::Include)
        .collect();
    let css: Vec<_> = map
        .nodes
        .iter()
        .filter(|n| n.kind.is_style())
        .collect();
    let js: Vec<_> = map
        .nodes
        .iter()
        .filter(|n| n.kind.is_script())
        .collect();

    place_ring(&mut pos, &includes, cx, cy, 160.0, -std::f64::consts::FRAC_PI_2);
    // CSS on the left half-arc, JS on the right.
    place_arc(
        &mut pos,
        &css,
        cx,
        cy,
        300.0,
        std::f64::consts::FRAC_PI_2 + 0.25,
        std::f64::consts::PI + std::f64::consts::FRAC_PI_2 - 0.25,
    );
    place_arc(
        &mut pos,
        &js,
        cx,
        cy,
        300.0,
        -std::f64::consts::FRAC_PI_2 + 0.25,
        std::f64::consts::FRAC_PI_2 - 0.25,
    );

    // Any leftover node (shouldn't happen) — park below center.
    for n in &map.nodes {
        pos.entry(n.id.clone()).or_insert(MapPoint {
            x: cx,
            y: cy + 340.0,
        });
    }
    pos
}

fn place_ring(pos: &mut HashMap<String, MapPoint>, nodes: &[&AssetNode], cx: f64, cy: f64, r: f64, start: f64) {
    let n = nodes.len().max(1) as f64;
    for (i, node) in nodes.iter().enumerate() {
        let angle = start + (i as f64 / n) * std::f64::consts::TAU;
        pos.insert(
            node.id.clone(),
            MapPoint {
                x: cx + angle.cos() * r,
                y: cy + angle.sin() * r,
            },
        );
    }
}

fn place_arc(
    pos: &mut HashMap<String, MapPoint>,
    nodes: &[&AssetNode],
    cx: f64,
    cy: f64,
    r: f64,
    a0: f64,
    a1: f64,
) {
    let count = nodes.len();
    if count == 0 {
        return;
    }
    for (i, node) in nodes.iter().enumerate() {
        let t = if count == 1 {
            0.5
        } else {
            i as f64 / (count as f64 - 1.0)
        };
        let angle = a0 + (a1 - a0) * t;
        // Slight radius jitter so dense arcs don't stack labels.
        let rr = r + ((i % 3) as f64 - 1.0) * 28.0;
        pos.insert(
            node.id.clone(),
            MapPoint {
                x: cx + angle.cos() * rr,
                y: cy + angle.sin() * rr,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn maps_shell_style_page() {
        let dir = tempfile_dir();
        std::fs::create_dir_all(dir.join("includes")).unwrap();
        std::fs::create_dir_all(dir.join("css")).unwrap();
        std::fs::create_dir_all(dir.join("components")).unwrap();
        write(
            &dir.join("index.php"),
            "<?php\nrequire __DIR__ . '/includes/shell-start.php';\n?>\nhi\n<?php require __DIR__ . '/includes/shell-end.php'; ?>\n",
        );
        write(
            &dir.join("includes/shell-start.php"),
            r#"<link rel="stylesheet" href="/css/looks.css" />
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=X" />
<?php require __DIR__ . '/chrome-header.php'; ?>
"#,
        );
        write(
            &dir.join("includes/shell-end.php"),
            r#"<script defer src="/components/mor-theme.js"></script>
"#,
        );
        write(&dir.join("includes/chrome-header.php"), "<!-- header -->\n");
        write(&dir.join("css/looks.css"), "body{}\n");
        write(&dir.join("components/mor-theme.js"), "/* js */\n");

        let map = map_page_assets(&dir, "index.php");
        assert_eq!(map.root_page, "index.php");
        assert!(map.include_count() >= 2, "includes: {:?}", map.nodes);
        assert!(map.css_count() >= 2, "css: {:?}", map.nodes);
        assert!(map.js_count() >= 1, "js: {:?}", map.nodes);
        assert!(
            map.nodes.iter().any(|n| n.path == "css/looks.css" && n.exists),
            "local css missing: {:?}",
            map.nodes
        );
        assert!(
            map.nodes
                .iter()
                .any(|n| n.kind == AssetKind::ExternalCss),
            "external font missing"
        );
        let layout = layout_mindmap(&map, 480.0, 320.0);
        assert_eq!(layout.len(), map.nodes.len());
    }

    fn tempfile_dir() -> PathBuf {
        let mut d = std::env::temp_dir();
        d.push(format!("mor_page_assets_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn write(path: &Path, body: &str) {
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(body.as_bytes()).unwrap();
    }
}
