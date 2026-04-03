//! Database integration smoke tests — assert real query APIs succeed after init.

use std::sync::Once;

static INIT_DB: Once = Once::new();

fn ensure_db() {
    INIT_DB.call_once(|| {
        app_lib::db::init_global().expect("init_global");
    });
}

#[test]
fn test_db_list_and_cache_queries_ok() {
    ensure_db();
    let db = app_lib::db::global();
    assert!(db.list_scans().is_ok(), "list_scans");
    assert!(db.latest_scan_id().is_ok(), "latest_scan_id");
    assert!(db.get_plugin_scans().is_ok(), "get_plugin_scans");
    assert!(db.get_daw_scans().is_ok(), "get_daw_scans");
    assert!(db.get_preset_scans().is_ok(), "get_preset_scans");
    assert!(db.get_audio_scans_list().is_ok(), "get_audio_scans_list");
    assert!(db.load_kvr_cache().is_ok(), "load_kvr_cache");
    assert!(db.unanalyzed_paths(10).is_ok(), "unanalyzed_paths");
}

#[test]
fn test_db_delete_scan_missing_id_ok() {
    ensure_db();
    let db = app_lib::db::global();
    assert!(
        db.delete_scan("scan-id-that-does-not-exist-0000").is_ok(),
        "delete_scan should not error on missing id"
    );
}
