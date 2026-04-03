//! Integration checks for `scanner::discover_plugins` (single-level scan rules) and
//! `xref` helpers that are cheap to assert without fixtures.

use std::sync::atomic::{AtomicU64, Ordering};

static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

fn unique_temp(prefix: &str) -> std::path::PathBuf {
    let n = TMP_SEQ.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("{prefix}_{n}"))
}

#[test]
fn discover_plugins_nonexistent_directory_yields_empty() {
    let dirs = vec!["/nonexistent/audio_haxor_discover_plugins_path".to_string()];
    let found = app_lib::scanner::discover_plugins(&dirs);
    assert!(found.is_empty());
}

#[test]
fn discover_plugins_only_includes_vst_vst3_component_dll() {
    let base = unique_temp("ah_discover_mix");
    std::fs::create_dir_all(&base).expect("mkdir");
    let _ = std::fs::create_dir_all(base.join("Plugin.vst3"));
    let _ = std::fs::create_dir_all(base.join("Legacy.vst"));
    let _ = std::fs::write(base.join("noise.clap"), b"");
    let _ = std::fs::write(base.join("readme.txt"), b"");
    let _ = std::fs::create_dir_all(base.join("Other.component"));

    let dirs = vec![base.to_string_lossy().to_string()];
    let mut paths: Vec<String> = app_lib::scanner::discover_plugins(&dirs)
        .into_iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    paths.sort_unstable();

    assert_eq!(
        paths,
        vec![
            "Legacy.vst".to_string(),
            "Other.component".to_string(),
            "Plugin.vst3".to_string(),
        ],
        "only .vst / .vst3 / .component / .dll entries; .clap and .txt ignored"
    );

    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn xref_extract_plugins_unknown_extension_returns_empty() {
    let f = unique_temp("ah_xref_bad_ext.txt");
    std::fs::write(&f, b"not a project").expect("write");
    let plugins = app_lib::xref::extract_plugins(f.to_str().expect("utf8 path"));
    assert!(
        plugins.is_empty(),
        "unknown extension should short-circuit to empty, got {}",
        plugins.len()
    );
    let _ = std::fs::remove_file(&f);
}

#[test]
fn xref_normalize_plugin_name_strips_nested_suffixes() {
    assert_eq!(
        app_lib::xref::normalize_plugin_name("Serum (x64) (VST3)"),
        "serum"
    );
    assert_eq!(
        app_lib::xref::normalize_plugin_name("   spaced   name   "),
        "spaced name"
    );
}
