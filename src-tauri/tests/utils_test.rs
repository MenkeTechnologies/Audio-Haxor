use app_lib::format_size;
use app_lib::path_norm::{normalize_path_for_db, path_strings_json_normalized};

#[test]
fn test_format_size_edge_cases() {
    assert_eq!(format_size(0), "0 B");
    assert_eq!(format_size(1), "1.0 B");
    assert_eq!(format_size(1023), "1023.0 B");
    assert_eq!(format_size(1024), "1.0 KB");
    assert_eq!(format_size(1024 * 1024), "1.0 MB");
    assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
    assert_eq!(format_size(1024 * 1024 * 1024 * 1024), "1.0 TB");
    // Ensure it doesn't crash on very large values
    assert!(format_size(u64::MAX).ends_with(" TB"));
}

#[test]
fn test_path_normalization_basic() {
    // Should be no-op for normal paths
    assert_eq!(normalize_path_for_db("/Users/test/file.wav"), "/Users/test/file.wav");
    assert_eq!(normalize_path_for_db("C:\\Users\\test\\file.wav"), "C:\\Users\\test\\file.wav");
}

#[test]
#[cfg(target_os = "macos")]
fn test_path_normalization_macos_data_prefix() {
    assert_eq!(
        normalize_path_for_db("/System/Volumes/Data/Users/test/file.wav"),
        "/Users/test/file.wav"
    );
    // Should NOT strip if it doesn't match the exact prefix
    assert_eq!(
        normalize_path_for_db("/Volumes/Data/Users/test/file.wav"),
        "/Volumes/Data/Users/test/file.wav"
    );
}

#[test]
fn test_path_strings_json_normalized() {
    let paths = vec![
        "/a/b.wav".to_string(),
        "/c/d.wav".to_string(),
    ];
    let json = path_strings_json_normalized(&paths);
    let parsed: Vec<String> = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0], "/a/b.wav");
    assert_eq!(parsed[1], "/c/d.wav");
}

#[test]
fn test_path_strings_json_normalized_empty() {
    let empty: Vec<String> = vec![];
    assert_eq!(path_strings_json_normalized(&empty), "[]");
}
