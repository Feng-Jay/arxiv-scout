use reqwest::Client;
use tracing::{info, warn};

/// Fetch clean paper text from arXiv HTML endpoint.
/// Returns at most `max_chars` characters, or `None` if unavailable.
/// Retries up to `max_attempts` times with exponential backoff.
pub async fn fetch_paper_text(
    client: &Client,
    paper_id: &str,
    max_chars: usize,
    max_attempts: u32,
) -> Option<String> {
    let url = format!("https://arxiv.org/html/{}", paper_id);
    info!("  Fetching HTML: {}", url);

    let html = {
        let mut last_err;
        let mut fetched: Option<String> = None;
        for attempt in 1..=max_attempts.max(1) {
            match client
                .get(&url)
                .header("User-Agent", "Mozilla/5.0 (compatible; paper-scout/0.1)")
                .send()
                .await
            {
                Err(e) => {
                    last_err = e.to_string();
                }
                Ok(resp) if !resp.status().is_success() => {
                    last_err = format!("HTTP {}", resp.status());
                }
                Ok(resp) => match resp.text().await {
                    Ok(t) => { fetched = Some(t); break; }
                    Err(e) => { last_err = e.to_string(); }
                },
            }
            if attempt < max_attempts {
                let wait = 2_u64.pow(attempt - 1);
                warn!("  HTML fetch failed for '{}' (attempt {}/{}): {}. Retrying in {}s ...",
                    paper_id, attempt, max_attempts, last_err, wait);
                tokio::time::sleep(tokio::time::Duration::from_secs(wait)).await;
            } else {
                warn!("  HTML fetch failed for '{}' (attempt {}/{}): {}. Giving up.",
                    paper_id, attempt, max_attempts, last_err);
            }
        }
        match fetched {
            Some(t) => t,
            None => return None,
        }
    };

    let text = html_to_text(&html);
    if text.len() < 200 {
        warn!("  HTML text too short for '{}', skipping", paper_id);
        return None;
    }

    Some(text.chars().take(max_chars).collect())
}

// ── HTML → plain text ─────────────────────────────────────────────────────────

/// Convert arXiv HTML to plain text.
///
/// - Skips content inside <script>, <style>, <head> entirely.
/// - For <math>, emits the `alttext` attribute value instead of MathML markup.
/// - Inserts newlines at block-level elements.
/// - Decodes common HTML entities.
fn html_to_text(html: &str) -> String {
    // Tags whose entire subtree should be dropped
    const SKIP: &[&str] = &["script", "style", "head", "nav", "footer"];
    // Block elements that get a newline
    const BLOCK: &[&str] = &[
        "p", "div", "br", "h1", "h2", "h3", "h4", "h5", "h6",
        "li", "tr", "td", "th", "section", "article", "blockquote",
    ];

    let mut out = String::with_capacity(html.len() / 3);
    let bytes = html.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut skip_depth: usize = 0;
    let mut math_depth: usize = 0;
    let mut math_text_emitted = false;

    while i < len {
        // ── Plain text character ──────────────────────────────────────────────
        if bytes[i] != b'<' {
            if skip_depth == 0 && !(math_depth > 0 && math_text_emitted) {
                if bytes[i] == b'&' {
                    let (entity, consumed) = decode_entity(&html[i..]);
                    out.push_str(entity);
                    i += consumed;
                } else {
                    // Emit valid UTF-8 char
                    let ch = html[i..].chars().next().unwrap_or('\0');
                    out.push(ch);
                    i += ch.len_utf8();
                }
            } else {
                i += 1;
            }
            continue;
        }

        // ── Tag ───────────────────────────────────────────────────────────────
        // Skip HTML comment <!-- ... -->
        if html[i..].starts_with("<!--") {
            let end = html[i..].find("-->").map(|p| i + p + 3).unwrap_or(len);
            i = end;
            continue;
        }

        // Find end of tag
        let tag_end = html[i..].find('>').map(|p| i + p + 1).unwrap_or(len);
        let raw_tag = &html[i..tag_end];

        let inner = raw_tag.trim_start_matches('<').trim_end_matches('>').trim();
        let is_closing = inner.starts_with('/');
        let name_str = inner.trim_start_matches('/');
        let tag_name = name_str
            .split(|c: char| c.is_ascii_whitespace() || c == '/')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();

        // ── <math> special handling ───────────────────────────────────────────
        if tag_name == "math" {
            if !is_closing {
                math_depth += 1;
                if math_depth == 1 {
                    math_text_emitted = false;
                    if let Some(alt) = attr_value(raw_tag, "alttext") {
                        if skip_depth == 0 {
                            out.push(' ');
                            out.push_str(&alt);
                            out.push(' ');
                            math_text_emitted = true;
                        }
                    }
                }
            } else if math_depth > 0 {
                math_depth -= 1;
                if math_depth == 0 {
                    math_text_emitted = false;
                }
            }
            i = tag_end;
            continue;
        }

        // Inside math subtree — skip all child tags
        if math_depth > 0 {
            i = tag_end;
            continue;
        }

        // ── Skip-zone tags ────────────────────────────────────────────────────
        if SKIP.contains(&tag_name.as_str()) {
            if !is_closing {
                skip_depth += 1;
            } else if skip_depth > 0 {
                skip_depth -= 1;
            }
            i = tag_end;
            continue;
        }

        // ── Block newlines ────────────────────────────────────────────────────
        if skip_depth == 0 && BLOCK.contains(&tag_name.as_str()) && !out.ends_with('\n') {
            out.push('\n');
        }

        i = tag_end;
    }

    // Collapse whitespace
    let mut result = String::with_capacity(out.len() / 2);
    let mut prev_blank = false;
    for line in out.lines() {
        let t = line.trim();
        if t.is_empty() {
            if !prev_blank {
                result.push('\n');
            }
            prev_blank = true;
        } else {
            result.push_str(t);
            result.push('\n');
            prev_blank = false;
        }
    }
    result
}

/// Extract the value of an HTML attribute from a raw tag string.
fn attr_value(tag: &str, attr: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    // Match attr="value" or attr='value'
    for quote in ['"', '\''] {
        let needle = format!("{}={}", attr, quote);
        if let Some(pos) = lower.find(&needle) {
            let start = pos + needle.len();
            if let Some(end) = tag[start..].find(quote) {
                return Some(tag[start..start + end].to_string());
            }
        }
    }
    None
}

/// Decode a single HTML entity at the start of `s`.
/// Returns (replacement_str, bytes_consumed).
fn decode_entity(s: &str) -> (&str, usize) {
    if s.starts_with("&amp;")   { return ("&",  5); }
    if s.starts_with("&lt;")    { return ("<",  4); }
    if s.starts_with("&gt;")    { return (">",  4); }
    if s.starts_with("&quot;")  { return ("\"", 6); }
    if s.starts_with("&nbsp;")  { return (" ",  6); }
    if s.starts_with("&apos;")  { return ("'",  6); }
    // Numeric entity: skip it
    if s.starts_with("&#") {
        let end = s.find(';').unwrap_or(1);
        return ("", end + 1);
    }
    // Unknown entity: emit as-is, consume just the '&'
    ("&", 1)
}
