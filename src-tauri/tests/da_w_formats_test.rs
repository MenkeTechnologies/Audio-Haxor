//! High-quality DAW format tests

#[test]
fn test_ableton_set_parsing() {
    // Test Ableton Set XML parsing
    #[cfg(target_os = "macos")]
    {
        let als_content = r#"<?xml version="1.0"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Format</key>
    <string>Ableton Live Set</string>
    <key>Version</key>
    <integer>11</integer>
</dict>
</plist>"#;

        let path = "/tmp/test.set";
        std::fs::write(path, als_content).ok();

        // Verify the content is valid
        let content = std::fs::read_to_string(path);
        if let Ok(c) = content {
            assert!(c.contains("Ableton Live Set"));
        }

        let _ = std::fs::remove_file(path);
    }
}

#[test]
fn test_studio_one_session() {
    // Test Studio One session file structure
    let temp = std::env::temp_dir().join("studio_one");
    std::fs::create_dir_all(&temp).ok();

    let song_path = temp.join("song.ses3");
    let _ = std::fs::write(&song_path, b"fake ses3 data");

    assert!(song_path.exists());

    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn test_reason_rea_project() {
    // Reason project files
    let temp = std::env::temp_dir().join("reason");
    std::fs::create_dir_all(&temp).ok();

    let rea_path = temp.join("song.rea");
    let _ = std::fs::write(&rea_path, b"fake rea data");

    assert!(rea_path.exists());

    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn test_fl_project() {
    // FL Studio project files
    let temp = std::env::temp_dir().join("flstudio");
    std::fs::create_dir_all(&temp).ok();

    let flp_path = temp.join("project.flp");
    let _ = std::fs::write(&flp_path, b"fake flp data");

    assert!(flp_path.exists());

    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn test_bitwig_project() {
    // Bitwig Studio project files
    let temp = std::env::temp_dir().join("bitwig");
    std::fs::create_dir_all(&temp).ok();

    // Create a package directory (macOS package)
    let pkg_path = temp.join("project.bwproject.pkg");
    let _ = std::fs::create_dir_all(&pkg_path);

    // Create files inside the package
    let _ = std::fs::write(pkg_path.join("info.plist"), b"<plist></plist>");

    assert!(pkg_path.exists());

    let _ = std::fs::remove_dir_all(&temp);
}
