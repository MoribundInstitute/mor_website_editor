//! Helpers for Google Sites–style rich editing in the preview iframe.

use dioxus::prelude::*;
use std::path::{Path, PathBuf};

/// Run a document.execCommand-style action in the preview iframe's active
/// contentEditable session.
pub fn iframe_rich_cmd(cmd: &str, val: Option<&str>) {
    let cmd = cmd.to_string();
    let val = val.map(|s| s.to_string());
    spawn(async move {
        let js = r#"
            const m = await dioxus.recv();
            const frm = document.getElementById('mor-preview-frame')
                || document.querySelector('iframe.preview-iframe')
                || document.querySelector('iframe');
            const w = frm && frm.contentWindow;
            if (!w) return false;
            if (m.html) {
                if (w.__morRichInsert && w.__morRichInsert(m.html)) return true;
                // Click-insert without an active caret: drop into page body + save.
                if (w.__morRichDropHtml) return !!w.__morRichDropHtml(m.html, null, null);
                return false;
            }
            if (m.dropHtml && w.__morRichDropHtml) { return !!w.__morRichDropHtml(m.dropHtml, m.x, m.y); }
            if (w.__morRichCmd) return !!w.__morRichCmd(m.cmd, m.val || null);
            return false;
        "#;
        let eval = dioxus::document::eval(js);
        let payload = if let Some(v) = val {
            if cmd == "html" || cmd.starts_with("html:") {
                serde_json::json!({ "html": v })
            } else {
                serde_json::json!({ "cmd": cmd, "val": v })
            }
        } else {
            serde_json::json!({ "cmd": cmd })
        };
        let _ = eval.send(payload);
    });
}

pub fn iframe_rich_insert_html(html: &str) {
    iframe_rich_cmd("html", Some(html));
}

/// MIME type used when dragging Insert-dock blocks onto the preview.
pub const MOR_BLOCK_MIME: &str = "application/x-mor-insert-html";

/// Snippets for the Insert dock (Google Sites–like content blocks).
#[derive(Clone, Copy)]
pub struct InsertBlock {
    pub id: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub html: &'static str,
}

pub const TEXT_BLOCKS: &[InsertBlock] = &[
    InsertBlock {
        id: "heading",
        label: "Heading",
        hint: "H2 section title — drag onto the page",
        html: "<h2>New heading</h2>",
    },
    InsertBlock {
        id: "paragraph",
        label: "Paragraph",
        hint: "Body text — drag onto the page",
        html: "<p>Start writing…</p>",
    },
    InsertBlock {
        id: "quote",
        label: "Quote",
        hint: "Blockquote",
        html: "<blockquote>A short quote.</blockquote>",
    },
    InsertBlock {
        id: "list",
        label: "Bulleted list",
        hint: "Unordered list",
        html: "<ul><li>Item one</li><li>Item two</li></ul>",
    },
    InsertBlock {
        id: "olist",
        label: "Numbered list",
        hint: "Ordered list",
        html: "<ol><li>First</li><li>Second</li></ol>",
    },
    InsertBlock {
        id: "two_col",
        label: "Two columns",
        hint: "Simple 50/50 layout row",
        html: concat!(
            "<div class=\"mor-insert-row\" style=\"display:grid;grid-template-columns:1fr 1fr;gap:1rem\">",
            "<div class=\"mor-insert-col\"><p>Column one</p></div>",
            "<div class=\"mor-insert-col\"><p>Column two</p></div>",
            "</div>"
        ),
    },
    InsertBlock {
        id: "three_col",
        label: "Three columns",
        hint: "Simple three-column row",
        html: concat!(
            "<div class=\"mor-insert-row\" style=\"display:grid;grid-template-columns:1fr 1fr 1fr;gap:1rem\">",
            "<div class=\"mor-insert-col\"><p>A</p></div>",
            "<div class=\"mor-insert-col\"><p>B</p></div>",
            "<div class=\"mor-insert-col\"><p>C</p></div>",
            "</div>"
        ),
    },
];

pub const MEDIA_BLOCKS: &[InsertBlock] = &[
    InsertBlock {
        id: "button",
        label: "Button",
        hint: "Primary link button",
        html: " <a class=\"btn-primary\" href=\"#\">Button</a> ",
    },
    InsertBlock {
        id: "link",
        label: "Text link",
        hint: "Inline link (or use toolbar)",
        html: " <a href=\"https://\">link text</a> ",
    },
    InsertBlock {
        id: "divider",
        label: "Divider",
        hint: "Horizontal rule",
        html: "<hr />",
    },
    InsertBlock {
        id: "spacer",
        label: "Spacer",
        hint: "Vertical space",
        html: "<div style=\"height:2rem\" aria-hidden=\"true\"></div>",
    },
    InsertBlock {
        id: "card",
        label: "Card",
        hint: "Bordered content card",
        html: concat!(
            "<div class=\"mor-insert-card\" style=\"border:1px solid var(--border,#333);",
            "border-radius:8px;padding:1rem;background:var(--bg-panel,#151d29)\">",
            "<h3>Card title</h3><p>Card body text.</p></div>"
        ),
    },
];

fn images_dir(project_root: &Path) -> PathBuf {
    project_root.join("images")
}

fn unique_image_dest(images: &Path, preferred_name: &str) -> PathBuf {
    let safe: String = preferred_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe = if safe.is_empty() {
        "image.png".into()
    } else {
        safe
    };
    let dest = images.join(&safe);
    if !dest.exists() {
        return dest;
    }
    let stem = dest.file_stem().and_then(|s| s.to_str()).unwrap_or("img");
    let ext = dest.extension().and_then(|s| s.to_str()).unwrap_or("png");
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() % 100_000)
        .unwrap_or(0);
    images.join(format!("{stem}_{n}.{ext}"))
}

/// Copy a local image into the project `images/` folder and return a site-relative path.
pub fn import_image_to_project(project_root: &Path, source: &Path) -> Result<String, String> {
    if !project_root.is_dir() {
        return Err("No website folder open".into());
    }
    let name = source
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "Invalid image file name".to_string())?;
    let images = images_dir(project_root);
    std::fs::create_dir_all(&images).map_err(|e| format!("Create images/: {e}"))?;
    let dest = unique_image_dest(&images, name);
    std::fs::copy(source, &dest).map_err(|e| format!("Copy image: {e}"))?;
    let file = dest
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(name);
    Ok(format!("/images/{file}"))
}

/// Guess extension from Content-Type or URL path.
fn ext_from_url_or_ctype(url: &str, ctype: Option<&str>) -> &'static str {
    if let Some(ct) = ctype {
        let ct = ct.to_ascii_lowercase();
        if ct.contains("png") {
            return "png";
        }
        if ct.contains("jpeg") || ct.contains("jpg") {
            return "jpg";
        }
        if ct.contains("gif") {
            return "gif";
        }
        if ct.contains("webp") {
            return "webp";
        }
        if ct.contains("svg") {
            return "svg";
        }
    }
    let path = url.split('?').next().unwrap_or(url);
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".png") {
        "png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "jpg"
    } else if lower.ends_with(".gif") {
        "gif"
    } else if lower.ends_with(".webp") {
        "webp"
    } else if lower.ends_with(".svg") {
        "svg"
    } else {
        "png"
    }
}

/// Download a remote image into `images/` and return a site-relative `/images/…` path.
///
/// Only `http`/`https` URLs are allowed. On failure, returns an error (caller may
/// fall back to hotlinking the URL).
pub async fn download_image_to_project(
    project_root: PathBuf,
    url: String,
) -> Result<String, String> {
    let url = url.trim().to_string();
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("URL must start with http:// or https://".into());
    }
    if !project_root.is_dir() {
        return Err("No website folder open".into());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("MorWebsite-Editor/0.1 (image import)")
        .build()
        .map_err(|e| format!("HTTP client: {e}"))?;

    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("Download HTTP {}", res.status()));
    }
    let ctype = res
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    // Soft check: warn if clearly not an image, still allow if extension says so.
    if let Some(ref ct) = ctype {
        let ctl = ct.to_ascii_lowercase();
        if !ctl.starts_with("image/") && !ctl.contains("svg") && !ctl.contains("octet-stream") {
            return Err(format!("URL does not look like an image (Content-Type: {ct})"));
        }
    }
    let bytes = res
        .bytes()
        .await
        .map_err(|e| format!("Read body: {e}"))?;
    if bytes.len() > 12 * 1024 * 1024 {
        return Err("Image larger than 12 MB — refuse to import".into());
    }
    if bytes.is_empty() {
        return Err("Empty image body".into());
    }

    let ext = ext_from_url_or_ctype(&url, ctype.as_deref());
    let stem = url
        .rsplit('/')
        .next()
        .unwrap_or("image")
        .split('?')
        .next()
        .unwrap_or("image");
    let stem: String = stem
        .chars()
        .take(40)
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stem = if stem.is_empty() || stem == format!(".{ext}") {
        "image".into()
    } else {
        stem.trim_end_matches(&format!(".{ext}")).to_string()
    };
    let preferred = format!("{stem}.{ext}");

    let images = images_dir(&project_root);
    std::fs::create_dir_all(&images).map_err(|e| format!("Create images/: {e}"))?;
    let dest = unique_image_dest(&images, &preferred);
    std::fs::write(&dest, &bytes).map_err(|e| format!("Write image: {e}"))?;
    let file = dest
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&preferred);
    Ok(format!("/images/{file}"))
}

/// Build an `<img>` tag for a site-relative or absolute URL.
pub fn img_tag(src: &str, alt: &str) -> String {
    let alt = alt.replace('"', "&quot;");
    format!(
        r#"<img src="{src}" alt="{alt}" style="max-width:100%;height:auto" />"#
    )
}
