//! Optional header-only metadata lookup. It never creates a job or staging file.
use anyhow::{Result, ensure};
#[derive(Clone, Debug)]
pub struct Metadata {
    pub filename: String,
    pub host: String,
    pub size: Option<u64>,
}
pub fn identity(raw: &str) -> Result<Metadata> {
    let url = url::Url::parse(raw.trim())?;
    ensure!(
        matches!(url.scheme(), "http" | "https") && url.host_str().is_some(),
        "Enter an HTTP or HTTPS URL"
    );
    ensure!(
        url.username().is_empty() && url.password().is_none(),
        "Embedded credentials are not supported"
    );
    let name = url
        .path_segments()
        .and_then(|mut parts| parts.next_back())
        .filter(|name| !name.is_empty())
        .unwrap_or("download");
    let filename =
        crate::sanitize_filename(&percent_encoding::percent_decode_str(name).decode_utf8_lossy());
    Ok(Metadata {
        filename,
        host: url.host_str().unwrap().to_owned(),
        size: None,
    })
}
fn disposition(raw: &str) -> Option<String> {
    let mut ordinary = None;
    let mut quoted = false;
    let mut escaped = false;
    // A semicolon inside a quoted filename is part of the name, not a delimiter.
    let parameters = raw.split(|c| {
        if escaped {
            escaped = false;
        } else if quoted && c == '\\' {
            escaped = true;
        } else if c == '"' {
            quoted = !quoted;
        } else {
            return c == ';' && !quoted;
        }
        false
    });
    for part in parameters.skip(1) {
        let Some((key, value)) = part.trim().split_once('=') else {
            continue;
        };
        let value = value.trim();
        if key.eq_ignore_ascii_case("filename*") {
            let mut parts = value.splitn(3, '\'');
            if parts
                .next()
                .is_some_and(|charset| charset.eq_ignore_ascii_case("utf-8"))
                && parts.next().is_some()
                && let Some(encoded) = parts.next()
                && let Ok(decoded) = percent_encoding::percent_decode_str(encoded).decode_utf8()
            {
                return Some(crate::sanitize_filename(&decoded));
            }
        }
        if key.eq_ignore_ascii_case("filename") {
            let value = if let Some(inner) = value.strip_prefix('"') {
                let Some(inner) = inner.strip_suffix('"') else {
                    continue;
                };
                let mut chars = inner.chars();
                let mut decoded = String::new();
                while let Some(c) = chars.next() {
                    decoded.push(if c == '\\' { chars.next()? } else { c });
                }
                decoded
            } else {
                value.to_owned()
            };
            ordinary = Some(crate::sanitize_filename(&value));
        }
    }
    ordinary
}
pub async fn probe(raw: &str) -> Result<Metadata> {
    let mut metadata = identity(raw)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(15))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()?;
    let mut response = client
        .head(raw.trim())
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .send()
        .await
        .map_err(|error| anyhow::anyhow!(error.without_url()))?;
    if !response.status().is_success() {
        response = client
            .get(raw.trim())
            .header(reqwest::header::ACCEPT_ENCODING, "identity")
            .header(reqwest::header::RANGE, "bytes=0-0")
            .send()
            .await
            .map_err(|error| anyhow::anyhow!(error.without_url()))?;
    }
    ensure!(
        response.status().is_success(),
        "Metadata check returned HTTP {}",
        response.status()
    );
    metadata.size = if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
        response
            .headers()
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| crate::network::parse_range(v).ok())
            .map(|r| r.2)
    } else {
        // HEAD has an empty response body; its header describes the resource size.
        response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok())
    };
    if let Some(name) = response
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|value| value.to_str().ok())
        .and_then(disposition)
    {
        metadata.filename = name;
    }
    // Drop the response without streaming its body, even when Range is ignored.
    Ok(metadata)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metadata_preserves_quoted_filename_parameters() {
        assert_eq!(
            disposition("attachment; filename=\"report; final.pdf\"; size=123").unwrap(),
            "report; final.pdf"
        );
        assert_eq!(
            disposition(r#"attachment; filename="report\"; final.pdf""#).unwrap(),
            "report_; final.pdf"
        );
    }
    #[test]
    fn metadata_prefers_extended_filename_with_language_tag() {
        for header in [
            "attachment; filename=fallback.zip; filename*=UTF-8'en'r%C3%A4ksm%C3%B6rg%C3%A5s.zip",
            "attachment; filename*=UtF-8'sv'r%C3%A4ksm%C3%B6rg%C3%A5s.zip; filename=fallback.zip",
        ] {
            assert_eq!(disposition(header).unwrap(), "räksmörgås.zip");
        }
        assert_eq!(
            disposition("attachment; filename=fallback.zip; filename*=UTF-8''%FF").unwrap(),
            "fallback.zip"
        );
    }
    #[test]
    fn metadata_names_cannot_escape_destination() {
        assert_eq!(
            identity("https://example.com/a%20b.zip").unwrap().filename,
            "a b.zip"
        );
        assert!(identity("file:///private").is_err());
        assert!(identity("https://user:secret@example.com/a").is_err());
        assert_eq!(
            disposition("attachment; filename*=UTF-8''..%2Funsafe.zip").unwrap(),
            "_unsafe.zip"
        );
    }
}
