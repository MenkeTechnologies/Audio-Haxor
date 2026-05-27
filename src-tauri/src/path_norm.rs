//! Filesystem path strings as stored in SQLite: shorter keys on macOS.

/// Strips the synthetic `/System/Volumes/Data` prefix when present so stored paths
/// match user-visible paths and use fewer bytes in row data and FTS5 indexes.
/// Matches traversal dedup in `audio_scanner`, `unified_walker`, etc.
pub fn normalize_path_for_db(s: &str) -> String {
    #[cfg(target_os = "macos")]
    {
        if s.starts_with("/System/Volumes/Data/") {
            return s["/System/Volumes/Data".len()..].to_string();
        }
    }
    s.to_string()
}

/// JSON array of path strings, each passed through [`normalize_path_for_db`].
pub fn path_strings_json_normalized(paths: &[String]) -> String {
    let v: Vec<String> = paths.iter().map(|p| normalize_path_for_db(p)).collect();
    serde_json::to_string(&v).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_path_for_db_noop_without_prefix() {
        assert_eq!(normalize_path_for_db("/Users/x/a.wav"), "/Users/x/a.wav");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn normalize_path_for_db_strips_data_volume() {
        assert_eq!(
            normalize_path_for_db("/System/Volumes/Data/Users/x/a.wav"),
            "/Users/x/a.wav"
        );
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn normalize_path_for_db_keeps_data_volume_on_non_macos() {
        assert_eq!(
            normalize_path_for_db("/System/Volumes/Data/Users/x/a.wav"),
            "/System/Volumes/Data/Users/x/a.wav"
        );
    }

    #[test]
    fn path_strings_json_normalized_empty_array() {
        let empty: Vec<String> = vec![];
        assert_eq!(path_strings_json_normalized(&empty), "[]");
    }

    #[test]
    fn path_strings_json_normalized_preserves_order_and_valid_json() {
        let paths = vec!["/first/a.wav".into(), "/second/b.wav".into()];
        let j = path_strings_json_normalized(&paths);
        let parsed: Vec<String> = serde_json::from_str(&j).expect("valid JSON array");
        assert_eq!(parsed, paths);
    }

    #[test]
    fn path_strings_json_normalized_escapes_special_chars() {
        let paths = vec!["/q/\"quote\".wav".into(), "/u/ünïcöde.flac".into()];
        let j = path_strings_json_normalized(&paths);
        let parsed: Vec<String> = serde_json::from_str(&j).expect("round-trip");
        assert_eq!(parsed, paths);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn path_strings_json_normalized_applies_per_element_on_macos() {
        let paths = vec![
            "/System/Volumes/Data/Users/x/a.wav".into(),
            "/Users/y/b.wav".into(),
        ];
        let j = path_strings_json_normalized(&paths);
        let parsed: Vec<String> = serde_json::from_str(&j).unwrap();
        assert_eq!(parsed, vec!["/Users/x/a.wav", "/Users/y/b.wav"]);
    }
}
