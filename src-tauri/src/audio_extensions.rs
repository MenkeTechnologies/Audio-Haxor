//! Canonical audio sample extensions for the library scanner, unified walker,
//! file watcher, and Settings → App Info. Keep in one place so UI labels match indexing.

/// Lowercase extensions with a leading dot (matches `audio_scanner` path handling).
pub const AUDIO_EXTENSIONS: &[&str] = &[
    ".wav", ".mp3", ".aiff", ".aif", ".flac", ".ogg", ".m4a", ".wma", ".aac", ".opus", ".rex",
    ".rx2", ".sf2", ".sfz",
];

/// `Path::extension()` lowercased, no dot — e.g. `"wav"`, `"m4a"`.
#[inline]
pub fn is_audio_extension_lowercase(ext_no_dot: &str) -> bool {
    AUDIO_EXTENSIONS
        .iter()
        .any(|e| e.strip_prefix('.') == Some(ext_no_dot))
}

/// Uppercase tags for Settings → App Info (no leading dot).
pub fn audio_format_tags_for_app_info() -> Vec<String> {
    AUDIO_EXTENSIONS
        .iter()
        .map(|e| e.strip_prefix('.').unwrap_or(e).to_ascii_uppercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_dotted_ext_maps_to_lowercase_predicate() {
        for d in AUDIO_EXTENSIONS {
            let plain = d.strip_prefix('.').expect("extension must start with '.'");
            assert!(
                is_audio_extension_lowercase(plain),
                "is_audio_extension_lowercase({plain})"
            );
        }
    }

    #[test]
    fn tags_align_with_extension_count() {
        assert_eq!(
            audio_format_tags_for_app_info().len(),
            AUDIO_EXTENSIONS.len()
        );
    }
}
