use std::sync::{Mutex, Once};

static INIT_DB: Once = Once::new();
/// Serialize prefs mutations — parallel tests can clobber the same on-disk prefs file.
static PREF_TEST_LOCK: Mutex<()> = Mutex::new(());

fn ensure_db() {
    INIT_DB.call_once(|| {
        app_lib::db::init_global().expect("init_global");
    });
}

#[test]
fn test_db_global_exists() {
    ensure_db();
    let _db = app_lib::db::global();
}

#[test]
fn test_db_preferences_path_nonempty() {
    let path = app_lib::history::get_preferences_path();
    assert!(!path.as_os_str().is_empty());
}

#[test]
fn test_db_data_dir() {
    let dir = app_lib::history::get_data_dir();
    assert!(dir.as_os_str().is_empty() || std::path::Path::new(&dir).exists());
}

#[test]
fn test_db_ensure_data_dir() {
    let dir = app_lib::history::ensure_data_dir();
    assert!(dir.as_os_str().is_empty() || std::path::Path::new(&dir).exists());
}

#[test]
fn test_db_load_preferences_serializable() {
    let prefs = app_lib::history::load_preferences();
    serde_json::to_string(&prefs).expect("prefs should serialize");
}

#[test]
fn test_db_preference_roundtrip() {
    let _lock = PREF_TEST_LOCK.lock().unwrap();
    let key = "audio_haxor_db_test_roundtrip";
    app_lib::history::set_preference(key, serde_json::json!("value"));
    assert_eq!(
        app_lib::history::get_preference(key),
        Some(serde_json::json!("value"))
    );
    app_lib::history::remove_preference(key);
    assert_eq!(app_lib::history::get_preference(key), None);
}

#[test]
fn test_db_save_load_preferences_stable() {
    let _lock = PREF_TEST_LOCK.lock().unwrap();
    let mut prefs = app_lib::history::load_preferences();
    prefs.insert(
        "audio_haxor_save_key".into(),
        serde_json::json!({"x": 1}),
    );
    app_lib::history::save_preferences(&prefs);
    let mut prefs2 = app_lib::history::load_preferences();
    assert_eq!(
        prefs2.get("audio_haxor_save_key"),
        Some(&serde_json::json!({"x": 1}))
    );
    prefs2.remove("audio_haxor_save_key");
    app_lib::history::save_preferences(&prefs2);
}
