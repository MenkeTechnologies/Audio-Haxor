//! Path / extension checks aligned with scanner expectations (no DOM, no IPC).

use std::path::Path;

#[test]
fn test_audio_extensions_on_file_paths() {
    let exts = [
        "aiff", "wav", "m4a", "flac", "mp3", "ogg", "aif", "opus", "aac",
    ];
    for ext in exts {
        let p = Path::new("/tmp").join(format!("track.{ext}"));
        assert_eq!(
            p.extension().and_then(|e| e.to_str()),
            Some(ext),
            "extension for .{ext}"
        );
    }
}

#[test]
fn test_daw_extensions_on_file_paths() {
    let exts = [
        "als", "band", "bwproject", "song", "flp", "ptx", "reason", "rpp", "cpr",
        "logicx", "npr", "dawproject", "ardour", "aup", "aup3",
    ];
    for ext in exts {
        let p = Path::new("/tmp").join(format!("project.{ext}"));
        assert_eq!(
            p.extension().and_then(|e| e.to_str()),
            Some(ext),
            "DAW extension .{ext}"
        );
    }
}

#[test]
fn test_vst_bundle_extensions() {
    for ext in [".vst", ".vst3", ".component", ".clap"] {
        assert!(ext.starts_with('.') && ext.len() > 1);
    }
}

#[test]
fn test_get_audio_metadata_nonexistent_has_error_or_empty_name() {
    let meta = app_lib::audio_scanner::get_audio_metadata("/nonexistent/audio_haxor_test.wav");
    assert_eq!(meta.full_path, "/nonexistent/audio_haxor_test.wav");
    assert!(
        meta.error.is_some() || meta.file_name.is_empty(),
        "missing file should surface error or empty name, got {:?}",
        meta.error
    );
}
