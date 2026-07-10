//! Structure-aware page text/HTML replacement for mixed PHP + HTML files.
//!
//! The preview browser often returns HTML that doesn't match the on-disk PHP
//! source byte-for-byte (quote style, void tags, whitespace). This module:
//!   1. Tries exact / quote-normalized HTML match
//!   2. Restricts search to HTML islands (outside `<?php … ?>`)
//!   3. Locates unique plain text and rewrites the innermost HTML element
//!      that contains it (PHP-aware text content edit)

/// Result of a successful page rewrite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageEditResult {
    pub updated: String,
    pub method: &'static str,
}

/// Strip script/handler injection from rich HTML before writing to disk.
///
/// Desktop editor is single-user; this is a safety net for paste/drop, not a
/// full HTML sanitizer. ponytail: string scan, not a browser parser.
pub fn sanitize_rich_html(html: &str) -> String {
    let mut s = strip_forbidden_elements(html);
    s = strip_event_handlers(&s);
    // Neutralize javascript: URLs anywhere (href/src/etc.).
    let mut out = String::with_capacity(s.len());
    let lower = s.to_ascii_lowercase();
    let mut i = 0;
    while i < s.len() {
        if lower[i..].starts_with("javascript:") {
            out.push_str("#");
            i += "javascript:".len();
            continue;
        }
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn strip_forbidden_elements(html: &str) -> String {
    let mut s = html.to_string();
    for tag in ["script", "iframe", "object", "embed"] {
        s = remove_paired_or_void_tag(&s, tag);
    }
    s
}

fn remove_paired_or_void_tag(html: &str, tag: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut out = String::with_capacity(html.len());
    let mut i = 0;
    while i < html.len() {
        if let Some(rel) = lower[i..].find(&open) {
            let at = i + rel;
            // Boundary: next char after tag name must not be alnum/-.
            let after = at + open.len();
            let ok_boundary = lower
                .as_bytes()
                .get(after)
                .map(|c| !c.is_ascii_alphanumeric() && *c != b'-')
                .unwrap_or(true);
            if !ok_boundary {
                out.push_str(&html[i..after]);
                i = after;
                continue;
            }
            out.push_str(&html[i..at]);
            let rest = &lower[at..];
            if let Some(c_rel) = rest.find(&close) {
                i = at + c_rel + close.len();
            } else if let Some(gt) = rest.find('>') {
                i = at + gt + 1;
            } else {
                break;
            }
        } else {
            out.push_str(&html[i..]);
            break;
        }
    }
    out
}

fn strip_event_handlers(html: &str) -> String {
    // Walk tags only; copy text nodes verbatim (UTF-8 safe).
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(lt) = rest.find('<') {
        out.push_str(&rest[..lt]);
        let tag_rest = &rest[lt..];
        let Some(gt_rel) = tag_rest.find('>') else {
            out.push_str(tag_rest);
            return out;
        };
        let tag = &tag_rest[..=gt_rel];
        out.push_str(&scrub_tag_attrs(tag));
        rest = &tag_rest[gt_rel + 1..];
    }
    out.push_str(rest);
    out
}

fn scrub_tag_attrs(tag: &str) -> String {
    // Fast path: no "on" prefix attrs.
    if !tag.to_ascii_lowercase().contains(" on") {
        return tag.to_string();
    }
    let bytes = tag.as_bytes();
    let mut out = String::with_capacity(tag.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            // on\w+\s*=
            if j + 2 < bytes.len()
                && bytes[j].eq_ignore_ascii_case(&b'o')
                && bytes[j + 1].eq_ignore_ascii_case(&b'n')
                && bytes[j + 2].is_ascii_alphabetic()
            {
                let mut k = j + 2;
                while k < bytes.len() && bytes[k].is_ascii_alphanumeric() {
                    k += 1;
                }
                while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                    k += 1;
                }
                if k < bytes.len() && bytes[k] == b'=' {
                    k += 1;
                    while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                        k += 1;
                    }
                    if k < bytes.len() && (bytes[k] == b'"' || bytes[k] == b'\'') {
                        let q = bytes[k];
                        k += 1;
                        while k < bytes.len() && bytes[k] != q {
                            k += 1;
                        }
                        if k < bytes.len() {
                            k += 1;
                        }
                    } else {
                        while k < bytes.len()
                            && !bytes[k].is_ascii_whitespace()
                            && bytes[k] != b'>'
                        {
                            k += 1;
                        }
                    }
                    i = k;
                    continue;
                }
            }
        }
        // ASCII-only tags in practice; copy one byte as char for attrs region.
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Apply a page edit. Returns `None` if no safe unique replacement is found.
pub fn apply_page_edit(src: &str, old: &str, new: &str) -> Option<PageEditResult> {
    if old.is_empty() || old == new {
        return None;
    }
    let rich = old.contains('<') || new.contains('<');
    let new_owned;
    let new = if rich {
        new_owned = sanitize_rich_html(new);
        if old == new_owned.as_str() {
            return None;
        }
        new_owned.as_str()
    } else {
        new
    };

    // 1) Exact
    if src.matches(old).count() == 1 {
        return Some(PageEditResult {
            updated: src.replacen(old, new, 1),
            method: "exact",
        });
    }

    // 2) Quote / void variants of rich HTML
    if rich {
        for cand in html_variants(old) {
            if src.matches(cand.as_str()).count() == 1 {
                return Some(PageEditResult {
                    updated: src.replacen(cand.as_str(), new, 1),
                    method: "html-variant",
                });
            }
        }
    }

    // 3) Search only in HTML islands (outside PHP)
    let islands = html_islands(src);
    if rich {
        for cand in html_variants(old) {
            let mut hits = 0usize;
            let mut hit_at: Option<usize> = None;
            for (start, end) in &islands {
                let slice = &src[*start..*end];
                let c = slice.matches(cand.as_str()).count();
                if c == 1 && hits == 0 {
                    // Absolute offset of match inside island
                    if let Some(rel) = slice.find(cand.as_str()) {
                        hit_at = Some(start + rel);
                    }
                }
                hits += c;
            }
            if hits == 1 {
                if let Some(at) = hit_at {
                    let mut out = String::with_capacity(src.len() - old.len() + new.len());
                    out.push_str(&src[..at]);
                    out.push_str(new);
                    out.push_str(&src[at + cand.len()..]);
                    return Some(PageEditResult {
                        updated: out,
                        method: "html-island",
                    });
                }
            }
        }
    }

    // 4) Unique plain text → rewrite innermost HTML element body
    let plain_old = strip_tags_collapsed(old);
    if plain_old.chars().count() >= 6 {
        if let Some(res) = replace_innermost_element_by_text(src, &plain_old, new, rich) {
            return Some(res);
        }
        // Restrict to HTML islands
        for (start, end) in &islands {
            let slice = &src[*start..*end];
            if let Some(res) = replace_innermost_element_by_text(slice, &plain_old, new, rich) {
                let mut out = String::new();
                out.push_str(&src[..*start]);
                out.push_str(&res.updated);
                out.push_str(&src[*end..]);
                return Some(PageEditResult {
                    updated: out,
                    method: "php-aware-text",
                });
            }
        }
    }

    // 5) Unique occurrence inside a PHP single- or double-quoted string
    if let Some(res) = replace_in_php_string(src, &plain_old, &strip_tags_collapsed(new)) {
        return Some(res);
    }

    None
}

fn html_variants(s: &str) -> Vec<String> {
    let mut v = vec![
        s.to_string(),
        s.replace('\"', "'"),
        s.replace('\'', "\""),
        s.replace(" />", ">").replace("/>", ">"),
    ];
    // Normalized whitespace form is used only for detection, not direct replace
    // unless it still appears uniquely in the source after light normalize.
    let n = normalize_html_ws(s);
    if !v.iter().any(|x| x == &n) {
        v.push(n);
    }
    v
}

fn normalize_html_ws(s: &str) -> String {
    let mut t = s.replace('\"', "'").replace("/>", ">");
    let mut out = String::with_capacity(t.len());
    let mut in_tag = false;
    let mut prev_space = false;
    for c in t.drain(..) {
        match c {
            '<' => {
                in_tag = true;
                prev_space = false;
                out.push(c);
            }
            '>' => {
                in_tag = false;
                prev_space = false;
                out.push(c);
            }
            c if c.is_whitespace() && !in_tag => {
                if !prev_space {
                    out.push(' ');
                    prev_space = true;
                }
            }
            c => {
                prev_space = false;
                out.push(c);
            }
        }
    }
    out.trim().to_string()
}

/// Byte ranges `[start, end)` of non-PHP content.
pub fn html_islands(src: &str) -> Vec<(usize, usize)> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut html_start = 0;
    while i < bytes.len() {
        // Open PHP: <?php or <?= or <?
        if bytes[i] == b'<' && i + 1 < bytes.len() && bytes[i + 1] == b'?' {
            if i > html_start {
                out.push((html_start, i));
            }
            // Skip to ?>
            i += 2;
            while i + 1 < bytes.len() {
                if bytes[i] == b'?' && bytes[i + 1] == b'>' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            html_start = i;
            continue;
        }
        i += 1;
    }
    if html_start < bytes.len() {
        out.push((html_start, bytes.len()));
    }
    // If no PHP at all, whole file is one island
    if out.is_empty() {
        out.push((0, src.len()));
    }
    out
}

fn strip_tags_collapsed(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Find unique plain text and replace the innermost element that fully contains it.
fn replace_innermost_element_by_text(
    src: &str,
    plain_old: &str,
    new: &str,
    new_is_html: bool,
) -> Option<PageEditResult> {
    // Count plain text occurrences in tag-stripped content is hard; use raw search
    // on collapsed whitespace version of HTML-stripped windows.
    let plain_hits: Vec<usize> = src.match_indices(plain_old).map(|(i, _)| i).collect();
    // Also try without requiring exact whitespace in source
    let plain_hits = if plain_hits.is_empty() {
        // Scan for collapsed match
        find_collapsed_text_offsets(src, plain_old)
    } else {
        plain_hits
    };
    if plain_hits.len() != 1 {
        return None;
    }
    let hit = plain_hits[0];

    // Walk outward from hit to find the innermost element whose open/close wrap the hit.
    let (el_start, inner_start, inner_end, el_end) = find_innermost_element(src, hit, hit + plain_old.len())?;

    let replacement_inner = if new_is_html {
        // If new is a full element matching the same tag, use its inner HTML only
        // when it wraps a single root; otherwise use as-is as inner content.
        extract_inner_if_single_root(new).unwrap_or_else(|| new.to_string())
    } else {
        html_escape_text(new)
    };

    let mut out = String::with_capacity(src.len());
    out.push_str(&src[..inner_start]);
    out.push_str(&replacement_inner);
    out.push_str(&src[inner_end..]);
    // Silence unused if el_start/el_end only for future
    let _ = (el_start, el_end);
    Some(PageEditResult {
        updated: out,
        method: "innermost-text",
    })
}

fn html_escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn extract_inner_if_single_root(html: &str) -> Option<String> {
    let t = html.trim();
    if !t.starts_with('<') {
        return None;
    }
    // Very light: <tag…>inner</tag>
    let open_end = t.find('>')?;
    let open = &t[1..open_end];
    let tag = open.split_whitespace().next()?.trim_end_matches('/');
    if tag.is_empty() || tag.starts_with('!') || tag.starts_with('?') {
        return None;
    }
    let close = format!("</{tag}>");
    if !t.to_ascii_lowercase().ends_with(&close.to_ascii_lowercase()) {
        return None;
    }
    let inner = &t[open_end + 1..t.len() - close.len()];
    Some(inner.to_string())
}

fn find_collapsed_text_offsets(src: &str, plain: &str) -> Vec<usize> {
    // Build mapping from collapsed index → original index for non-tag text.
    let mut map: Vec<usize> = Vec::new();
    let mut collapsed = String::new();
    let mut in_tag = false;
    let mut prev_space = false;
    for (i, c) in src.char_indices() {
        match c {
            '<' => {
                in_tag = true;
                prev_space = false;
            }
            '>' => {
                in_tag = false;
                prev_space = false;
            }
            c if !in_tag => {
                if c.is_whitespace() {
                    if !prev_space && !collapsed.is_empty() {
                        collapsed.push(' ');
                        map.push(i);
                        prev_space = true;
                    }
                } else {
                    collapsed.push(c);
                    map.push(i);
                    prev_space = false;
                }
            }
            _ => {}
        }
    }
    let plain = plain.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed
        .match_indices(&plain)
        .filter_map(|(ci, _)| map.get(ci).copied())
        .collect()
}

/// Returns (element_start, inner_start, inner_end, element_end).
fn find_innermost_element(
    src: &str,
    hit_start: usize,
    hit_end: usize,
) -> Option<(usize, usize, usize, usize)> {
    let mut stack: Vec<(usize, usize, String)> = Vec::new(); // el_start, inner_start, name
    let mut best: Option<(usize, usize, usize, usize)> = None; // smallest span
    let mut i = 0;
    let b = src.as_bytes();
    while i < b.len() {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        if i + 1 < b.len() && (b[i + 1] == b'!' || b[i + 1] == b'?') {
            if let Some(rel) = src[i..].find('>') {
                i += rel + 1;
            } else {
                break;
            }
            continue;
        }
        if i + 1 < b.len() && b[i + 1] == b'/' {
            let close_start = i;
            let ns = i + 2;
            let mut ne = ns;
            while ne < b.len() && (b[ne].is_ascii_alphanumeric() || b[ne] == b'-' || b[ne] == b':') {
                ne += 1;
            }
            let cname = src[ns..ne].to_ascii_lowercase();
            if let Some(rel) = src[ne..].find('>') {
                let close_end = ne + rel + 1;
                if let Some(pos) = stack.iter().rposition(|(_, _, n)| n == &cname) {
                    let (el_start, inner_start, _) = stack[pos].clone();
                    let inner_end = close_start;
                    let el_end = close_end;
                    if el_start <= hit_start && hit_end <= inner_end {
                        let span = el_end - el_start;
                        let replace = match best {
                            None => true,
                            Some((a, _, _, d)) => span < d - a,
                        };
                        if replace {
                            best = Some((el_start, inner_start, inner_end, el_end));
                        }
                    }
                    stack.truncate(pos);
                }
                i = close_end;
                continue;
            }
            break;
        }
        let tag_start = i;
        let ns = i + 1;
        let mut ne = ns;
        while ne < b.len() && (b[ne].is_ascii_alphanumeric() || b[ne] == b'-' || b[ne] == b':') {
            ne += 1;
        }
        if ne == ns {
            i += 1;
            continue;
        }
        let name = src[ns..ne].to_ascii_lowercase();
        if let Some(rel) = src[ne..].find('>') {
            let gt = ne + rel;
            let self_closing = gt > 0 && b[gt - 1] == b'/';
            let void = matches!(
                name.as_str(),
                "br" | "hr" | "img" | "input" | "meta" | "link" | "source" | "area" | "base" | "col" | "embed" | "wbr"
            );
            if !self_closing && !void {
                stack.push((tag_start, gt + 1, name));
            }
            i = gt + 1;
            continue;
        }
        break;
    }
    best
}

fn replace_in_php_string(src: &str, plain_old: &str, plain_new: &str) -> Option<PageEditResult> {
    if plain_old.len() < 6 {
        return None;
    }
    // Find unique plain_old inside "..." or '...' PHP strings (simple scan).
    let mut hits: Vec<(usize, usize, char)> = Vec::new(); // start, end of content, quote
    let b = src.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let q = b[i];
        if q != b'"' && q != b'\'' {
            i += 1;
            continue;
        }
        let quote = q as char;
        let content_start = i + 1;
        i += 1;
        while i < b.len() {
            if b[i] == b'\\' {
                i += 2;
                continue;
            }
            if b[i] == q {
                let content_end = i;
                let content = &src[content_start..content_end];
                if content.matches(plain_old).count() == 1 {
                    if let Some(rel) = content.find(plain_old) {
                        hits.push((content_start + rel, content_start + rel + plain_old.len(), quote));
                    }
                }
                i += 1;
                break;
            }
            i += 1;
        }
    }
    if hits.len() != 1 {
        return None;
    }
    let (a, b_end, _) = hits[0];
    let mut out = String::new();
    out.push_str(&src[..a]);
    out.push_str(plain_new);
    out.push_str(&src[b_end..]);
    Some(PageEditResult {
        updated: out,
        method: "php-string",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_replace() {
        let src = "<p>Hello world</p>";
        let r = apply_page_edit(src, "Hello world", "Hi there").unwrap();
        assert_eq!(r.updated, "<p>Hi there</p>");
        assert_eq!(r.method, "exact");
    }

    #[test]
    fn php_island_text_replace() {
        let src = r#"<?php echo $x; ?>
<p>Unique phrase here</p>
<?php /* more */ ?>"#;
        let r = apply_page_edit(src, "Unique phrase here", "Changed phrase").unwrap();
        assert!(r.updated.contains("Changed phrase"));
        assert!(!r.updated.contains("Unique phrase here"));
    }

    #[test]
    fn html_islands_skip_php() {
        let src = "<?php $a=1; ?><p>hi</p><?php $b=2; ?>";
        let is = html_islands(src);
        assert!(is.iter().any(|(s, e)| src[*s..*e].contains("<p>hi</p>")));
    }

    #[test]
    fn quote_variant_html() {
        let src = r#"<a href='/page.php'>Go</a>"#;
        let old = r#"<a href="/page.php">Go</a>"#;
        let r = apply_page_edit(src, old, r#"<a href="/page.php">Went</a>"#).unwrap();
        assert!(r.updated.contains("Went"));
    }

    #[test]
    fn sanitize_strips_script_and_handlers() {
        let dirty = r#"<p onclick="alert(1)">Hi</p><script>evil()</script><a href="javascript:alert(1)">x</a>"#;
        let clean = sanitize_rich_html(dirty);
        assert!(!clean.to_ascii_lowercase().contains("script"));
        assert!(!clean.contains("onclick"));
        assert!(!clean.to_ascii_lowercase().contains("javascript:"));
        assert!(clean.contains("Hi"));
    }

    #[test]
    fn apply_page_edit_sanitizes_rich_new() {
        let src = "<div><p>Unique block zz99</p></div>";
        let r = apply_page_edit(
            src,
            "<p>Unique block zz99</p>",
            r#"<p onclick="x">Safe</p><script>no</script>"#,
        )
        .unwrap();
        assert!(r.updated.contains("Safe"));
        assert!(!r.updated.contains("onclick"));
        assert!(!r.updated.to_ascii_lowercase().contains("script"));
    }
}
