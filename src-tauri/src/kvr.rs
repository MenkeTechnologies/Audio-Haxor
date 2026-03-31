//! KVR Audio scraper and plugin version checker.
//!
//! Queries [KVR Audio](https://www.kvraudio.com) for plugin product pages,
//! extracts version numbers and download URLs. Falls back to DuckDuckGo
//! site-restricted search when direct URL construction fails. Rate-limited
//! to avoid overloading KVR's servers.

use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

// Pre-compiled regexes for hot paths
static DOWNLOAD_LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"href="(https?://[^"]*(?:download|get|buy|release)[^"]*)""#).unwrap()
});
static PRODUCT_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"href="(/product/[^"]+)""#).unwrap());
static PLUGINS_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"href="(/plugins/[^"]+)""#).unwrap());
static KVR_DDG_LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"href="[^"]*?(https?://(?:www\.)?kvraudio\.com/product/[^"&]+)"#).unwrap()
});
static HTML_TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
static DATE_FILTER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^20[0-2]\d\.|^\d{4}\.").unwrap());
static VERSION_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"(?i)Version\s*[:]\s*(\d+\.\d+(?:\.\d+)?(?:\.\d+)?)",
        r"(?i)(?:Latest\s+)?Version</(?:dt|th|span|div|label)>\s*<(?:dd|td|span|div)[^>]*>\s*(\d+\.\d+(?:\.\d+)?(?:\.\d+)?)",
        r#"(?i)softwareVersion["\s:>]+(\d+\.\d+(?:\.\d+)?(?:\.\d+)?)"#,
        r"(?i)(?:current|latest|release|version)[^<]{0,40}?v?(\d+\.\d+(?:\.\d+)?(?:\.\d+)?)",
        r"(?i)Version\s*(?:<[^>]*>\s*)*(\d+\.\d+(?:\.\d+)?(?:\.\d+)?)",
    ]
    .iter()
    .map(|p| Regex::new(p).unwrap())
    .collect()
});
pub static URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"https?://[^\s)"',]+"#).unwrap());

const KVR_INVALID_PAGES: &[&str] = &["/plugins/the-newest-plugins", "/plugins/newest", "/plugins"];

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvrResult {
    #[serde(rename = "productUrl")]
    pub product_url: String,
    #[serde(rename = "downloadUrl")]
    pub download_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResult {
    #[serde(rename = "latestVersion")]
    pub latest_version: String,
    #[serde(rename = "hasUpdate")]
    pub has_update: bool,
    pub source: String,
    #[serde(rename = "updateUrl")]
    pub update_url: Option<String>,
    #[serde(rename = "kvrUrl")]
    pub kvr_url: Option<String>,
    #[serde(rename = "hasPlatformDownload")]
    pub has_platform_download: bool,
}

fn platform_keywords() -> Vec<&'static str> {
    if cfg!(target_os = "macos") {
        vec!["mac", "macos", "osx", "os x", "apple"]
    } else if cfg!(target_os = "windows") {
        vec!["win", "windows", "pc"]
    } else {
        vec!["linux", "ubuntu", "debian"]
    }
}

fn build_client() -> Client {
    Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .unwrap_or_default()
}

async fn fetch_with_validation(client: &Client, url: &str) -> Option<(String, String, bool)> {
    let resp = client.get(url).send().await.ok()?;
    let final_url = resp.url().to_string();
    let final_path = resp
        .url()
        .path()
        .split('?')
        .next()
        .unwrap_or("")
        .split('#')
        .next()
        .unwrap_or("");
    let is_invalid = KVR_INVALID_PAGES.iter().any(|p| final_path.starts_with(p));
    let status = resp.status();
    let html = resp.text().await.ok()?;
    Some((html, final_url, !is_invalid && status.is_success()))
}

async fn fetch_html(client: &Client, url: &str) -> Option<String> {
    let resp = client.get(url).send().await.ok()?;
    resp.text().await.ok()
}

pub fn extract_download_url(html: &str) -> Option<(String, bool)> {
    let all_links: Vec<String> = DOWNLOAD_LINK_RE
        .captures_iter(html)
        .map(|c| c[1].to_string())
        .collect();

    let keywords = platform_keywords();

    // Prefer platform-specific link
    for link in &all_links {
        let lower = link.to_lowercase();
        if keywords.iter().any(|kw| lower.contains(kw)) {
            return Some((link.clone(), true));
        }
    }

    // Check for platform text near download links
    for kw in &keywords {
        let pattern = format!(
            r#"(?i)(?:{})[^<]{{0,80}}?href="(https?://[^"]*(?:download|get)[^"]*)"|href="(https?://[^"]*(?:download|get)[^"]*)"[^<]{{0,80}}?(?:{})"#,
            regex::escape(kw),
            regex::escape(kw)
        );
        if let Ok(re) = Regex::new(&pattern) {
            if let Some(caps) = re.captures(html) {
                let url = caps
                    .get(1)
                    .or_else(|| caps.get(2))
                    .map(|m| m.as_str().to_string());
                if let Some(u) = url {
                    return Some((u, true));
                }
            }
        }
    }

    // Any download link
    all_links.first().map(|l| (l.clone(), false))
}

pub fn extract_version(html: &str) -> Option<String> {
    for re in VERSION_PATTERNS.iter() {
        if let Some(caps) = re.captures(html) {
            let ver = caps[1].to_string();
            if !DATE_FILTER_RE.is_match(&ver) {
                return Some(ver);
            }
        }
    }

    None
}

pub fn parse_version(ver: &str) -> Vec<i32> {
    if ver.is_empty() || ver == "Unknown" {
        return vec![0, 0, 0];
    }
    ver.split('.')
        .map(|n| n.parse::<i32>().unwrap_or(0))
        .collect()
}

pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let pa = parse_version(a);
    let pb = parse_version(b);
    let len = pa.len().max(pb.len());
    for i in 0..len {
        let va = pa.get(i).copied().unwrap_or(0);
        let vb = pb.get(i).copied().unwrap_or(0);
        match va.cmp(&vb) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

pub async fn resolve_kvr(direct_url: &str, plugin_name: &str) -> KvrResult {
    let client = build_client();

    // Try direct URL first
    if let Some((html, final_url, valid)) = fetch_with_validation(&client, direct_url).await {
        if valid {
            let download_url = extract_download_url(&html).map(|(u, _)| u);
            return KvrResult {
                product_url: final_url,
                download_url,
            };
        }
    }

    // Fallback: search KVR
    let search_url = format!(
        "https://www.kvraudio.com/plugins/search?q={}",
        urlencoding::encode(plugin_name)
    );
    if let Some(html) = fetch_html(&client, &search_url).await {
        let mut seen = std::collections::HashSet::new();
        let mut product_links = Vec::new();
        for caps in PRODUCT_LINK_RE.captures_iter(&html) {
            let href = caps[1].to_string();
            if seen.insert(href.clone()) {
                product_links.push(format!("https://www.kvraudio.com{}", href));
            }
        }

        let name_lower = plugin_name.to_lowercase();
        let name_slug = name_lower
            .replace(|c: char| !c.is_alphanumeric(), "-")
            .trim_matches('-')
            .to_string();
        let name_words: Vec<&str> = name_lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .collect();

        for found_url in product_links.iter().take(5) {
            let url_slug = found_url
                .split("/product/")
                .nth(1)
                .unwrap_or("")
                .to_string();
            let matching_words = name_words
                .iter()
                .filter(|w| w.len() > 1 && url_slug.contains(*w))
                .count();
            let threshold = (name_words.len() as f64 * 0.5).ceil() as usize;

            if url_slug.contains(&name_slug) || matching_words >= threshold {
                if let Some(page_html) = fetch_html(&client, found_url).await {
                    let download_url = extract_download_url(&page_html).map(|(u, _)| u);
                    return KvrResult {
                        product_url: found_url.clone(),
                        download_url,
                    };
                }
            }
        }

        if let Some(first_url) = product_links.first() {
            if let Some(page_html) = fetch_html(&client, first_url).await {
                let download_url = extract_download_url(&page_html).map(|(u, _)| u);
                return KvrResult {
                    product_url: first_url.clone(),
                    download_url,
                };
            }
        }
    }

    // Last resort
    KvrResult {
        product_url: format!(
            "https://www.kvraudio.com/plugins/search?q={}",
            urlencoding::encode(plugin_name)
        ),
        download_url: None,
    }
}

pub async fn find_latest_version(
    name: &str,
    manufacturer: &str,
    current_version: &str,
) -> Option<UpdateResult> {
    let client = build_client();
    let mfg = if manufacturer != "Unknown" {
        manufacturer
    } else {
        ""
    };

    // Try KVR search
    let query = format!("{} {}", mfg, name).trim().to_string();
    let search_url = format!(
        "https://www.kvraudio.com/plugins/search?q={}",
        urlencoding::encode(&query)
    );

    if let Some(html) = fetch_html(&client, &search_url).await {
        let mut product_links: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for caps in PRODUCT_LINK_RE.captures_iter(&html) {
            let href = caps[1].to_string();
            if seen.insert(href.clone()) {
                product_links.push(format!("https://www.kvraudio.com{}", href));
            }
        }

        // Also try /plugins/ style links
        for caps in PLUGINS_LINK_RE.captures_iter(&html) {
            let href = caps[1].to_string();
            if !href.contains("/search") && !href.contains("/category") && seen.insert(href.clone())
            {
                product_links.push(format!("https://www.kvraudio.com{}", href));
            }
        }

        for product_url in product_links.iter().take(2) {
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

            if let Some(page_html) = fetch_html(&client, product_url).await {
                let clean_name = name
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>()
                    .to_lowercase();
                let page_text = HTML_TAG_RE.replace_all(&page_html, "").to_lowercase();

                if !page_text.contains(&clean_name) && !page_text.contains(&name.to_lowercase()) {
                    continue;
                }

                if let Some(version) = extract_version(&page_html) {
                    let (download_url, has_platform) =
                        extract_download_url(&page_html).unwrap_or((product_url.clone(), false));
                    let has_update =
                        compare_versions(&version, current_version) == std::cmp::Ordering::Greater;
                    return Some(UpdateResult {
                        latest_version: version,
                        has_update,
                        source: "kvr".into(),
                        update_url: Some(download_url),
                        kvr_url: Some(product_url.clone()),
                        has_platform_download: has_platform,
                    });
                }
            }
        }
    }

    // Fallback: DuckDuckGo site-restricted search
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
    let ddg_query = format!("site:kvraudio.com {} {} VST version", mfg, name)
        .trim()
        .to_string();
    let ddg_url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(&ddg_query)
    );

    if let Some(ddg_html) = fetch_html(&client, &ddg_url).await {
        let mut kvr_links: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for caps in KVR_DDG_LINK_RE.captures_iter(&ddg_html) {
            let url = caps[1].to_string();
            if seen.insert(url.clone()) {
                kvr_links.push(url);
            }
        }

        for kvr_url in kvr_links.iter().take(2) {
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

            if let Some(page_html) = fetch_html(&client, kvr_url).await {
                if let Some(version) = extract_version(&page_html) {
                    let (download_url, has_platform) =
                        extract_download_url(&page_html).unwrap_or((kvr_url.clone(), false));
                    let has_update =
                        compare_versions(&version, current_version) == std::cmp::Ordering::Greater;
                    return Some(UpdateResult {
                        latest_version: version,
                        has_update,
                        source: "kvr-ddg".into(),
                        update_url: Some(download_url),
                        kvr_url: Some(kvr_url.clone()),
                        has_platform_download: has_platform,
                    });
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_basic() {
        assert_eq!(parse_version("1.2.3"), vec![1, 2, 3]);
        assert_eq!(parse_version("10.0"), vec![10, 0]);
        assert_eq!(parse_version("1.0.0.1"), vec![1, 0, 0, 1]);
    }

    #[test]
    fn test_parse_version_unknown() {
        assert_eq!(parse_version("Unknown"), vec![0, 0, 0]);
        assert_eq!(parse_version(""), vec![0, 0, 0]);
    }

    #[test]
    fn test_compare_versions_equal() {
        assert_eq!(
            compare_versions("1.0.0", "1.0.0"),
            std::cmp::Ordering::Equal
        );
        assert_eq!(compare_versions("2.1", "2.1.0"), std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_compare_versions_greater() {
        assert_eq!(
            compare_versions("2.0.0", "1.9.9"),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_versions("1.1", "1.0.9"),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_versions("1.0.1", "1.0.0"),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn test_compare_versions_less() {
        assert_eq!(compare_versions("1.0.0", "1.0.1"), std::cmp::Ordering::Less);
        assert_eq!(compare_versions("0.9", "1.0"), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_compare_versions_different_lengths() {
        assert_eq!(
            compare_versions("1.0", "1.0.0.0"),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            compare_versions("1.0.0.1", "1.0"),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn test_extract_version_basic() {
        let html = r#"<div>Version: 3.5.2</div>"#;
        assert_eq!(extract_version(html), Some("3.5.2".into()));
    }

    #[test]
    fn test_extract_version_latest() {
        let html = r#"<dt>Latest Version</dt><dd>2.1.0</dd>"#;
        assert_eq!(extract_version(html), Some("2.1.0".into()));
    }

    #[test]
    fn test_extract_version_software_version() {
        let html = r#"{"softwareVersion": "1.4.7"}"#;
        assert_eq!(extract_version(html), Some("1.4.7".into()));
    }

    #[test]
    fn test_extract_version_filters_dates() {
        let html = r#"<div>Version: 2024.01.15</div>"#;
        assert_eq!(extract_version(html), None);
    }

    #[test]
    fn test_extract_version_none() {
        let html = r#"<div>No version info here</div>"#;
        assert_eq!(extract_version(html), None);
    }

    #[test]
    fn test_extract_version_four_part() {
        let html = r#"<span>Version: 1.2.3.4</span>"#;
        assert_eq!(extract_version(html), Some("1.2.3.4".into()));
    }

    #[test]
    fn test_extract_download_url_basic() {
        let html = r#"<a href="https://example.com/download/plugin-v1.zip">Download</a>"#;
        let result = extract_download_url(html);
        assert!(result.is_some());
        let (url, _) = result.unwrap();
        assert_eq!(url, "https://example.com/download/plugin-v1.zip");
    }

    #[test]
    fn test_extract_download_url_none() {
        let html = r#"<a href="https://example.com/about">About</a>"#;
        assert!(extract_download_url(html).is_none());
    }

    #[test]
    fn test_platform_keywords_not_empty() {
        assert!(!platform_keywords().is_empty());
    }

    #[test]
    fn test_extract_version_with_v_prefix() {
        let html = "current version v2.3.1";
        assert_eq!(extract_version(html), Some("2.3.1".into()));
    }

    #[test]
    fn test_extract_version_release_context() {
        let html = "latest release 4.0.2 available";
        assert_eq!(extract_version(html), Some("4.0.2".into()));
    }

    #[test]
    fn test_extract_download_url_platform_specific() {
        let html = r#"
            <a href="https://example.com/download/plugin-win.zip">Windows</a>
            <a href="https://example.com/download/plugin-mac.dmg">Mac</a>
            <a href="https://example.com/download/plugin-linux.tar.gz">Linux</a>
        "#;
        let result = extract_download_url(html);
        assert!(result.is_some());
        let (url, is_platform) = result.unwrap();
        // On macOS, should prefer the mac link; on other platforms, the respective one
        if cfg!(target_os = "macos") {
            assert!(url.contains("mac"), "Expected mac URL, got: {}", url);
            assert!(is_platform);
        } else if cfg!(target_os = "windows") {
            assert!(url.contains("win"), "Expected windows URL, got: {}", url);
            assert!(is_platform);
        } else {
            assert!(url.contains("linux"), "Expected linux URL, got: {}", url);
            assert!(is_platform);
        }
    }

    #[test]
    fn test_compare_versions_single_component() {
        assert_eq!(compare_versions("3", "2"), std::cmp::Ordering::Greater);
    }

    #[test]
    fn test_parse_version_non_numeric() {
        assert_eq!(parse_version("abc"), vec![0]);
    }

    #[test]
    fn test_extract_version_multiple_versions_picks_first() {
        let html = r#"<div>Version: 1.0</div><div>Version: 2.0</div>"#;
        assert_eq!(extract_version(html), Some("1.0".into()));
    }

    #[test]
    fn test_extract_version_html_tags_between() {
        let html = r#"<dt>Version</dt><dd>3.2.1</dd>"#;
        assert_eq!(extract_version(html), Some("3.2.1".into()));
    }

    #[test]
    fn test_compare_versions_zero_vs_zero() {
        assert_eq!(
            compare_versions("0.0.0", "0.0.0"),
            std::cmp::Ordering::Equal
        );
    }

    #[test]
    fn test_compare_versions_leading_zeros() {
        // parse_version("1.02.3") -> [1, 2, 3] since i32 parse drops leading zeros
        let a = parse_version("1.02.3");
        let b = parse_version("1.2.3");
        assert_eq!(a, b);
        assert_eq!(
            compare_versions("1.02.3", "1.2.3"),
            std::cmp::Ordering::Equal
        );
    }

    #[test]
    fn test_extract_download_url_multiple_links() {
        let html = r#"
            <a href="https://example.com/download/a.zip">A</a>
            <a href="https://example.com/download/b.zip">B</a>
            <a href="https://example.com/download/c.zip">C</a>
        "#;
        let result = extract_download_url(html);
        assert!(result.is_some(), "Should find at least one download link");
    }

    #[test]
    fn test_extract_download_url_get_link() {
        let html = r#"<a href="https://example.com/get/plugin">Get Plugin</a>"#;
        let result = extract_download_url(html);
        assert!(result.is_some(), "Should find 'get' link");
        let (url, _) = result.unwrap();
        assert_eq!(url, "https://example.com/get/plugin");
    }

    #[test]
    fn test_extract_download_url_buy_link() {
        let html = r#"<a href="https://example.com/buy/plugin">Buy Plugin</a>"#;
        let result = extract_download_url(html);
        assert!(result.is_some(), "Should find 'buy' link");
        let (url, _) = result.unwrap();
        assert_eq!(url, "https://example.com/buy/plugin");
    }
}
