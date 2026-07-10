//! Local preview server for the opened website folder.
//!
//! One server at a time: opening a new folder replaces the previous one.
//! If `php` is on PATH the folder is served by `php -S` (so .php pages render);
//! otherwise a minimal std-only static file server takes over.

use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SiteServerInfo {
    pub port: u16,
    /// True when the folder is served by a `php -S` child (PHP pages execute).
    pub php: bool,
}

// The running php child, killed when a new folder opens.
// ponytail: one global server slot — the app previews one project at a time.
static PHP_CHILD: Mutex<Option<Child>> = Mutex::new(None);

/// Start serving `root` on a free localhost port.
pub fn start_server(root: &Path) -> io::Result<SiteServerInfo> {
    // Kill the previous php server (if any) before claiming a new port.
    if let Ok(mut slot) = PHP_CHILD.lock() {
        if let Some(mut child) = slot.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    // Bind :0 to let the OS pick a free port.
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();

    if php_available() {
        drop(listener); // free the port for php
        let child = Command::new("php")
            .arg("-S")
            .arg(format!("127.0.0.1:{port}"))
            .arg("-t")
            .arg(root)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        if let Ok(mut slot) = PHP_CHILD.lock() {
            *slot = Some(child);
        }
        return Ok(SiteServerInfo { port, php: true });
    }

    // Static fallback on a background thread.
    // ponytail: a replaced static server is leaked, idling on a port nothing
    // references anymore. Add a shutdown flag if orphan threads ever matter.
    let root = root.to_path_buf();
    std::thread::spawn(move || serve_static(listener, root));
    Ok(SiteServerInfo { port, php: false })
}

fn php_available() -> bool {
    Command::new("php")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Fetch a page body from the local server. Blocking — call off the UI thread.
pub fn fetch_page(port: u16, rel: &str) -> io::Result<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let rel = rel.trim_start_matches('/');
    write!(
        stream,
        "GET /{rel} HTTP/1.0\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    )?;
    let mut buf = Vec::new();
    // A timeout mid-read still yields whatever arrived, which beats erroring out.
    let _ = stream.read_to_end(&mut buf);
    let text = String::from_utf8_lossy(&buf).into_owned();
    Ok(match text.split_once("\r\n\r\n") {
        Some((_headers, body)) => body.to_string(),
        None => text,
    })
}

// ── static fallback ─────────────────────────────────────────────────────────

fn serve_static(listener: TcpListener, root: PathBuf) {
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let root = root.clone();
        // Thread-per-connection: the only client is the preview iframe.
        std::thread::spawn(move || {
            let _ = handle_conn(&mut stream, &root);
        });
    }
}

fn handle_conn(stream: &mut TcpStream, root: &Path) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[..n]).into_owned();
    // ponytail: "GET <path> ..." parse only; no keep-alive, no percent-decoding,
    // no chunked bodies. Enough for a localhost preview iframe.
    let path = req
        .strip_prefix("GET ")
        .and_then(|r| r.split_whitespace().next())
        .unwrap_or("/");
    let mut rel = path
        .split(['?', '#'])
        .next()
        .unwrap_or("/")
        .trim_start_matches('/')
        .to_string();
    if rel.is_empty() || rel.ends_with('/') {
        // ponytail: static fallback serves index.html only (php handles index.php).
        rel.push_str("index.html");
    }
    if rel.contains("..") {
        return respond(stream, "403 Forbidden", "text/plain", b"forbidden");
    }
    match std::fs::read(root.join(&rel)) {
        Ok(body) => respond(stream, "200 OK", mime_for(&rel), &body),
        Err(_) => respond(stream, "404 Not Found", "text/plain", b"not found"),
    }
}

fn respond(stream: &mut TcpStream, status: &str, mime: &str, body: &[u8]) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.0 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    stream.flush()
}

fn mime_for(rel: &str) -> &'static str {
    match rel.rsplit('.').next().unwrap_or("").to_ascii_lowercase().as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "json" => "application/json",
        "txt" => "text/plain; charset=utf-8",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        _ => "application/octet-stream", // ponytail: 404-worthy per spec, but octet-stream is friendlier
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_server_serves_and_guards_traversal() {
        let dir = std::env::temp_dir().join(format!("mor_srv_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.html"), "<html>hi</html>").unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let root = dir.clone();
        std::thread::spawn(move || serve_static(listener, root));

        let body = fetch_page(port, "index.html").unwrap();
        assert_eq!(body, "<html>hi</html>");
        let missing = fetch_page(port, "../etc/passwd").unwrap();
        assert!(missing.contains("forbidden") || missing.contains("not found"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
