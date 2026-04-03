use std::collections::HashMap;

#[tokio::test]
async fn test_kvr_find_latest_network_or_none() {
    let result: Option<app_lib::kvr::UpdateResult> =
        app_lib::kvr::find_latest_version("Test Plugin", "Test Co", "1.0").await;
    // Offline CI: None is fine; online: Some is fine
    if let Some(u) = result {
        assert!(!u.source.is_empty(), "UpdateResult should set source");
    }
}

#[test]
fn test_kvr_compare_versions_orders_semver() {
    assert_eq!(
        app_lib::kvr::compare_versions("2.0", "1.0"),
        std::cmp::Ordering::Greater
    );
    assert_eq!(
        app_lib::kvr::compare_versions("1.0", "1.0"),
        std::cmp::Ordering::Equal
    );
    assert_eq!(
        app_lib::kvr::compare_versions("0.9", "1.0"),
        std::cmp::Ordering::Less
    );
}

#[test]
fn test_kvr_result_struct() {
    let result = app_lib::kvr::KvrResult {
        product_url: "https://example.com/product".to_string(),
        download_url: None,
    };

    assert!(!result.product_url.is_empty());
}

#[test]
fn test_kvr_cache_struct() {
    let entry = app_lib::history::KvrCacheEntry {
        kvr_url: None,
        update_url: None,
        latest_version: None,
        has_update: false,
        source: "not-found".to_string(),
        timestamp: "2024-01-01T00:00:00Z".to_string(),
    };

    assert_eq!(entry.source, "not-found");
}

#[test]
fn test_kvr_cache_operations() {
    let mut cache: HashMap<String, app_lib::history::KvrCacheEntry> = HashMap::new();

    let entry = app_lib::history::KvrCacheEntry {
        kvr_url: None,
        update_url: None,
        latest_version: None,
        has_update: false,
        source: "kvraudio".to_string(),
        timestamp: "2024-01-01T00:00:00Z".to_string(),
    };

    cache.insert("test_key".to_string(), entry);
    assert_eq!(cache.get("test_key").map(|e| e.source.as_str()), Some("kvraudio"));
}
