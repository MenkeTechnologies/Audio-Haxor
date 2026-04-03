//! Integration: `gen_id`, `compute_plugin_diff` (real history logic, no I/O).

use app_lib::history::{build_plugin_snapshot, compute_plugin_diff};
use app_lib::scanner::PluginInfo;

fn plugin(name: &str, path: &str, version: &str) -> PluginInfo {
    PluginInfo {
        name: name.into(),
        path: path.into(),
        plugin_type: "VST3".into(),
        version: version.into(),
        manufacturer: "M".into(),
        manufacturer_url: None,
        size: "1 MB".into(),
        size_bytes: 1,
        modified: "2024-01-01".into(),
        architectures: vec![],
    }
}

#[test]
fn test_gen_id_many_unique() {
    use std::collections::HashSet;
    let ids: Vec<String> = (0..200).map(|_| app_lib::history::gen_id()).collect();
    let uniq: HashSet<_> = ids.iter().collect();
    assert_eq!(ids.len(), uniq.len());
    for id in &ids {
        assert!(!id.is_empty());
    }
}

#[test]
fn test_compute_plugin_diff_added_removed_and_version_change() {
    let old_plugins = vec![
        plugin("A", "/vst/a.vst3", "1.0.0"),
        plugin("B", "/vst/b.vst3", "1.0.0"),
    ];
    let new_plugins = vec![
        plugin("A", "/vst/a.vst3", "2.0.0"),
        plugin("C", "/vst/c.vst3", "1.0.0"),
    ];
    let old_snap = build_plugin_snapshot(&old_plugins, &[], &[]);
    let new_snap = build_plugin_snapshot(&new_plugins, &[], &[]);

    let diff = compute_plugin_diff(&old_snap, &new_snap);

    assert_eq!(diff.added.len(), 1);
    assert_eq!(diff.added[0].name, "C");
    assert_eq!(diff.removed.len(), 1);
    assert_eq!(diff.removed[0].name, "B");
    assert_eq!(diff.version_changed.len(), 1);
    assert_eq!(diff.version_changed[0].plugin.name, "A");
    assert_eq!(diff.version_changed[0].previous_version, "1.0.0");
}
