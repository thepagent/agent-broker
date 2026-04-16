use crate::acp::ContentBlock;
use crate::config::{OutboundConfig, SttConfig};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use image::ImageReader;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::LazyLock;
use tracing::{debug, error, info, warn};

/// Reusable HTTP client for downloading attachments (shared across adapters).
pub static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("static HTTP client must build")
});

/// Maximum dimension (width or height) for resized images.
const IMAGE_MAX_DIMENSION_PX: u32 = 1200;

/// JPEG quality for compressed output.
const IMAGE_JPEG_QUALITY: u8 = 75;

/// Download an image from a URL, resize/compress it, and return as a ContentBlock.
/// Pass `auth_token` for platforms that require authentication (e.g. Slack private files).
pub async fn download_and_encode_image(
    url: &str,
    mime_hint: Option<&str>,
    filename: &str,
    size: u64,
    auth_token: Option<&str>,
) -> Option<ContentBlock> {
    const MAX_SIZE: u64 = 10 * 1024 * 1024; // 10 MB

    if url.is_empty() {
        return None;
    }

    let mime = mime_hint.or_else(|| {
        filename
            .rsplit('.')
            .next()
            .and_then(|ext| match ext.to_lowercase().as_str() {
                "png" => Some("image/png"),
                "jpg" | "jpeg" => Some("image/jpeg"),
                "gif" => Some("image/gif"),
                "webp" => Some("image/webp"),
                _ => None,
            })
    });

    let Some(mime) = mime else {
        debug!(filename, "skipping non-image attachment");
        return None;
    };
    let mime = mime.split(';').next().unwrap_or(mime).trim();
    if !mime.starts_with("image/") {
        debug!(filename, mime, "skipping non-image attachment");
        return None;
    }

    if size > MAX_SIZE {
        error!(filename, size, "image exceeds 10MB limit");
        return None;
    }

    let mut req = HTTP_CLIENT.get(url);
    if let Some(token) = auth_token {
        req = req.header("Authorization", format!("Bearer {token}"));
    }

    let response = match req.send().await {
        Ok(resp) => resp,
        Err(e) => { error!(url, error = %e, "download failed"); return None; }
    };
    if !response.status().is_success() {
        error!(url, status = %response.status(), "HTTP error downloading image");
        return None;
    }
    let bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => { error!(url, error = %e, "read failed"); return None; }
    };

    if bytes.len() as u64 > MAX_SIZE {
        error!(filename, size = bytes.len(), "downloaded image exceeds limit");
        return None;
    }

    let (output_bytes, output_mime) = match resize_and_compress(&bytes) {
        Ok(result) => result,
        Err(e) => {
            if bytes.len() > 1024 * 1024 {
                error!(filename, error = %e, size = bytes.len(), "resize failed and original too large, skipping");
                return None;
            }
            debug!(filename, error = %e, "resize failed, using original");
            (bytes.to_vec(), mime.to_string())
        }
    };

    debug!(
        filename,
        original_size = bytes.len(),
        compressed_size = output_bytes.len(),
        "image processed"
    );

    let encoded = BASE64.encode(&output_bytes);
    Some(ContentBlock::Image {
        media_type: output_mime,
        data: encoded,
    })
}

/// Download an audio file and transcribe it via the configured STT provider.
/// Pass `auth_token` for platforms that require authentication.
pub async fn download_and_transcribe(
    url: &str,
    filename: &str,
    mime_type: &str,
    size: u64,
    stt_config: &SttConfig,
    auth_token: Option<&str>,
) -> Option<String> {
    const MAX_SIZE: u64 = 25 * 1024 * 1024; // 25 MB (Whisper API limit)

    if size > MAX_SIZE {
        error!(filename, size, "audio exceeds 25MB limit");
        return None;
    }

    let mut req = HTTP_CLIENT.get(url);
    if let Some(token) = auth_token {
        req = req.header("Authorization", format!("Bearer {token}"));
    }

    let resp = req.send().await.ok()?;
    if !resp.status().is_success() {
        error!(url, status = %resp.status(), "audio download failed");
        return None;
    }
    let bytes = resp.bytes().await.ok()?.to_vec();

    crate::stt::transcribe(&HTTP_CLIENT, stt_config, bytes, filename.to_string(), mime_type).await
}

/// Resize image so longest side <= IMAGE_MAX_DIMENSION_PX, then encode as JPEG.
/// GIFs are passed through unchanged to preserve animation.
pub fn resize_and_compress(raw: &[u8]) -> Result<(Vec<u8>, String), image::ImageError> {
    let reader = ImageReader::new(Cursor::new(raw))
        .with_guessed_format()?;

    let format = reader.format();

    if format == Some(image::ImageFormat::Gif) {
        return Ok((raw.to_vec(), "image/gif".to_string()));
    }

    let img = reader.decode()?;
    let (w, h) = (img.width(), img.height());

    let img = if w > IMAGE_MAX_DIMENSION_PX || h > IMAGE_MAX_DIMENSION_PX {
        let max_side = std::cmp::max(w, h);
        let ratio = f64::from(IMAGE_MAX_DIMENSION_PX) / f64::from(max_side);
        let new_w = (f64::from(w) * ratio) as u32;
        let new_h = (f64::from(h) * ratio) as u32;
        img.resize(new_w, new_h, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    let mut buf = Cursor::new(Vec::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, IMAGE_JPEG_QUALITY);
    img.write_with_encoder(encoder)?;

    Ok((buf.into_inner(), "image/jpeg".to_string()))
}

/// Check if a MIME type is audio.
pub fn is_audio_mime(mime: &str) -> bool {
    mime.starts_with("audio/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_png(width: u32, height: u32) -> Vec<u8> {
        let img = image::RgbImage::new(width, height);
        let mut buf = Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        buf.into_inner()
    }

    #[test]
    fn large_image_resized_to_max_dimension() {
        let png = make_png(3000, 2000);
        let (compressed, mime) = resize_and_compress(&png).unwrap();

        assert_eq!(mime, "image/jpeg");
        let result = image::load_from_memory(&compressed).unwrap();
        assert!(result.width() <= IMAGE_MAX_DIMENSION_PX);
        assert!(result.height() <= IMAGE_MAX_DIMENSION_PX);
    }

    #[test]
    fn small_image_keeps_original_dimensions() {
        let png = make_png(800, 600);
        let (compressed, mime) = resize_and_compress(&png).unwrap();

        assert_eq!(mime, "image/jpeg");
        let result = image::load_from_memory(&compressed).unwrap();
        assert_eq!(result.width(), 800);
        assert_eq!(result.height(), 600);
    }

    #[test]
    fn landscape_image_respects_aspect_ratio() {
        let png = make_png(4000, 2000);
        let (compressed, _) = resize_and_compress(&png).unwrap();

        let result = image::load_from_memory(&compressed).unwrap();
        assert_eq!(result.width(), 1200);
        assert_eq!(result.height(), 600);
    }

    #[test]
    fn portrait_image_respects_aspect_ratio() {
        let png = make_png(2000, 4000);
        let (compressed, _) = resize_and_compress(&png).unwrap();

        let result = image::load_from_memory(&compressed).unwrap();
        assert_eq!(result.width(), 600);
        assert_eq!(result.height(), 1200);
    }

    #[test]
    fn compressed_output_is_smaller_than_original() {
        let png = make_png(3000, 2000);
        let (compressed, _) = resize_and_compress(&png).unwrap();

        assert!(compressed.len() < png.len(), "compressed {} should be < original {}", compressed.len(), png.len());
    }

    #[test]
    fn gif_passes_through_unchanged() {
        let gif: Vec<u8> = vec![
            0x47, 0x49, 0x46, 0x38, 0x39, 0x61,
            0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
            0x2C, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00,
            0x02, 0x02, 0x44, 0x01, 0x00,
            0x3B,
        ];
        let (output, mime) = resize_and_compress(&gif).unwrap();

        assert_eq!(mime, "image/gif");
        assert_eq!(output, gif);
    }

    #[test]
    fn invalid_data_returns_error() {
        let garbage = vec![0x00, 0x01, 0x02, 0x03];
        assert!(resize_and_compress(&garbage).is_err());
    }
}

// --- Outbound attachments ---
//
// Implements the agent → chat file upload pathway requested in
// openabdev/openab#298 and the security hardening in openabdev/openab#355.
// Agents write `![alt](/path)` markdown in their response; this module
// extracts, validates, and surfaces paths to the adapter layer (which
// uploads each file as a native chat attachment).

/// Regex for outbound attachment markers: `![alt](/path/to/file)`.
static OUTBOUND_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"!\[[^\]]*\]\((/[^\)]+)\)").unwrap()
});

/// Canonicalize the configured allowlist once at first use. On macOS
/// `/tmp` is a symlink to `/private/tmp`; canonicalizing means later
/// `Path::starts_with` checks compare canonical ↔ canonical.
///
/// Returned as `Vec<PathBuf>`. Entries that fail to canonicalize
/// (missing directory on this host) are dropped silently.
fn canonicalize_allowlist(dirs: &[String]) -> Vec<PathBuf> {
    dirs.iter()
        .filter_map(|p| match std::fs::canonicalize(p) {
            Ok(canon) => Some(canon),
            Err(e) => {
                warn!(dir = p, error = %e, "outbound: allowed dir not canonicalizable, dropping");
                None
            }
        })
        .collect()
}

/// Scan agent response `text` for `![alt](/path)` markers, validate each
/// path against `config`, and return `(cleaned_text, list_of_paths)`.
///
/// Validation rules (all must pass):
/// 1. Canonicalization succeeds — resolves symlinks AND `..` components
///    (closes symlink escape and path traversal attacks).
/// 2. Canonical path lives under one of `config.allowed_dirs`
///    (compared component-wise with `Path::starts_with`, not string prefix).
/// 3. The resolved target is a regular file and ≤ `config.max_size_bytes`.
/// 4. Total attachments per call capped at `config.max_per_message`
///    (extras are dropped with a warning; the markers stay in the text so
///    users see what was attempted).
///
/// If `config.enabled` is false this function is a no-op — the input
/// text is returned untouched and no paths are extracted.
///
/// Markers for files that pass every check are stripped from the cleaned
/// text. Markers that fail (blocked, too large, missing) stay in the text
/// so the user sees what the agent tried to send.
pub fn extract_outbound_attachments(
    text: &str,
    config: &OutboundConfig,
) -> (String, Vec<PathBuf>) {
    if !config.enabled {
        return (text.to_string(), Vec::new());
    }
    let allowlist = canonicalize_allowlist(&config.allowed_dirs);
    if allowlist.is_empty() {
        warn!("outbound: enabled but every allowed_dir failed to canonicalize; refusing to send");
        return (text.to_string(), Vec::new());
    }

    let mut attachments = Vec::new();
    let mut paths_to_strip = Vec::new();
    let mut over_cap_dropped = 0usize;

    for cap in OUTBOUND_RE.captures_iter(text) {
        if attachments.len() >= config.max_per_message {
            over_cap_dropped += 1;
            continue;
        }

        let full_match = cap.get(0).unwrap().as_str();
        let path_str = &cap[1];
        let path = PathBuf::from(path_str);

        // Rule 1: canonicalize first. Resolves symlinks AND `..` components.
        let canonical = match std::fs::canonicalize(&path) {
            Ok(p) => p,
            Err(e) => {
                debug!(path = %path_str, error = %e, "outbound: cannot canonicalize");
                continue;
            }
        };

        // Rule 2: component-wise allowlist check on canonical path.
        let allowed = allowlist.iter().any(|prefix| canonical.starts_with(prefix));
        if !allowed {
            warn!(
                path = %path_str,
                canonical = %canonical.display(),
                "outbound: path not in allowed_dirs"
            );
            continue;
        }

        // Rule 3: regular file within size cap (metadata on canonical path
        // is symlink-safe at this point — the link was already resolved).
        let size_cap = config.max_size_bytes();
        match std::fs::metadata(&canonical) {
            Ok(meta) if meta.is_file() && meta.len() <= size_cap => {
                info!(path = %canonical.display(), size = meta.len(), "outbound: attachment accepted");
                attachments.push(canonical);
                paths_to_strip.push(full_match.to_string());
            }
            Ok(meta) if meta.len() > size_cap => {
                warn!(
                    path = %canonical.display(),
                    size = meta.len(),
                    limit_mb = config.max_file_size_mb,
                    "outbound: over size limit"
                );
            }
            Ok(_) => {
                warn!(path = %canonical.display(), "outbound: not a regular file");
            }
            Err(e) => {
                debug!(path = %canonical.display(), error = %e, "outbound: metadata error");
            }
        }
    }

    if over_cap_dropped > 0 {
        warn!(
            over_cap_dropped,
            cap = config.max_per_message,
            "outbound: per-message cap hit"
        );
    }

    let mut cleaned = text.to_string();
    for marker in &paths_to_strip {
        cleaned = cleaned.replace(marker, "");
    }
    while cleaned.contains("\n\n\n") {
        cleaned = cleaned.replace("\n\n\n", "\n\n");
    }
    (cleaned.trim().to_string(), attachments)
}

#[cfg(test)]
mod outbound_tests {
    use super::*;

    fn cfg_enabled() -> OutboundConfig {
        OutboundConfig {
            enabled: true,
            ..OutboundConfig::default()
        }
    }

    #[test]
    fn disabled_by_default_is_noop() {
        let cfg = OutboundConfig::default();
        assert!(!cfg.enabled);
        let text = "![foo](/tmp/does-not-matter.png)";
        let (cleaned, atts) = extract_outbound_attachments(text, &cfg);
        assert_eq!(cleaned, text);
        assert!(atts.is_empty());
    }

    #[test]
    fn enabled_extracts_tmp_file() {
        let path = "/tmp/openab_pr300_happy.png";
        std::fs::write(path, b"png").unwrap();
        let text = "Here: ![screenshot](/tmp/openab_pr300_happy.png) done.";
        let (cleaned, atts) = extract_outbound_attachments(text, &cfg_enabled());
        assert_eq!(atts.len(), 1);
        let expected = std::fs::canonicalize(path).unwrap();
        assert_eq!(atts[0], expected);
        assert!(!cleaned.contains("openab_pr300_happy"));
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn blocks_non_allowlisted_path() {
        let text = "![secret](/etc/passwd)";
        let (cleaned, atts) = extract_outbound_attachments(text, &cfg_enabled());
        assert!(atts.is_empty());
        // Blocked markers stay in text so the user sees the attempt.
        assert!(cleaned.contains("/etc/passwd"));
    }

    #[test]
    fn blocks_symlink_escape() {
        let link = "/tmp/openab_pr300_symlink.png";
        let _ = std::fs::remove_file(link);
        std::os::unix::fs::symlink("/etc/hosts", link).unwrap();
        let text = format!("![esc]({link})");
        let (_, atts) = extract_outbound_attachments(&text, &cfg_enabled());
        assert!(
            atts.is_empty(),
            "symlink escaping /tmp must be blocked: {:?}",
            atts
        );
        std::fs::remove_file(link).ok();
    }

    #[test]
    fn blocks_path_traversal() {
        let (_, atts) =
            extract_outbound_attachments("![x](/tmp/../etc/hosts)", &cfg_enabled());
        assert!(atts.is_empty(), "/tmp/../ must be blocked");
    }

    #[test]
    fn respects_custom_allowed_dirs() {
        let dir = "/tmp/openab_pr300_custom";
        std::fs::create_dir_all(dir).unwrap();
        let path = format!("{dir}/file.png");
        std::fs::write(&path, b"x").unwrap();

        // Default allowed_dirs is /tmp and /var/folders — this path passes.
        let default_cfg = cfg_enabled();
        let (_, atts) = extract_outbound_attachments(
            &format!("![a]({path})"),
            &default_cfg,
        );
        assert_eq!(atts.len(), 1);

        // Narrow the allowed dirs: only accept something unrelated.
        let narrow_cfg = OutboundConfig {
            enabled: true,
            allowed_dirs: vec!["/var/empty/".into()],
            ..OutboundConfig::default()
        };
        let (_, atts_narrow) = extract_outbound_attachments(
            &format!("![a]({path})"),
            &narrow_cfg,
        );
        assert!(atts_narrow.is_empty(), "narrowed allowlist must exclude /tmp");

        std::fs::remove_file(&path).ok();
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn enforces_max_file_size_mb() {
        // 2 MB file, 1 MB limit → blocked.
        let path = "/tmp/openab_pr300_large.bin";
        std::fs::write(path, vec![0u8; 2 * 1024 * 1024]).unwrap();
        let cfg = OutboundConfig {
            enabled: true,
            max_file_size_mb: 1,
            ..OutboundConfig::default()
        };
        let (_, atts) = extract_outbound_attachments(
            &format!("![big]({path})"),
            &cfg,
        );
        assert!(atts.is_empty(), "file exceeding max_file_size_mb must be blocked");
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn enforces_max_per_message() {
        let mut text = String::new();
        let mut paths = Vec::new();
        for i in 0..5 {
            let p = format!("/tmp/openab_pr300_permsg_{i}.png");
            std::fs::write(&p, b"x").unwrap();
            text.push_str(&format!("![a{i}]({p})\n"));
            paths.push(p);
        }
        let cfg = OutboundConfig {
            enabled: true,
            max_per_message: 2,
            ..OutboundConfig::default()
        };
        let (_, atts) = extract_outbound_attachments(&text, &cfg);
        assert_eq!(atts.len(), 2, "must cap at max_per_message");
        for p in &paths {
            std::fs::remove_file(p).ok();
        }
    }

    #[test]
    fn strips_multiple_valid_markers() {
        let a = "/tmp/openab_pr300_multi_a.png";
        let b = "/tmp/openab_pr300_multi_b.png";
        std::fs::write(a, b"a").unwrap();
        std::fs::write(b, b"b").unwrap();
        let text = format!("one ![a]({a}) two ![b]({b}) three");
        let (cleaned, atts) = extract_outbound_attachments(&text, &cfg_enabled());
        assert_eq!(atts.len(), 2);
        assert!(!cleaned.contains("openab_pr300_multi"));
        std::fs::remove_file(a).ok();
        std::fs::remove_file(b).ok();
    }
}
