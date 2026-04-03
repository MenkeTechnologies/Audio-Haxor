use std::path::PathBuf;

pub fn get_temp_dir() -> PathBuf {
    std::env::temp_dir().join("audio_haxor_tests")
}

pub fn setup_temp_plugin_file() -> PathBuf {
    let temp = get_temp_dir();
    let file = temp.join("test_vst.plugin");
    let _ = std::fs::write(&file, b"fake plugin content");
    file
}

pub fn setup_temp_audio_file() -> PathBuf {
    let temp = get_temp_dir();
    let file = temp.join("test_audio.wav");
    // Create a minimal valid WAV header (44 bytes)
    let wav_header = vec![
        b'R', b'I', b'F', b'F', 36, 0, 0, 0, b'W', b'A', b've', 0, 0, 0, 0,
        1, 0, 1, 0, 44, 100, 16, 16, 1, 0, 1, 0, 4, 0, 80, 0, 77, 65, 44,
        0, 0, 0, 1, 0, 1, 0, 96, 0, 0, 0, b'i', b'n', b't', b'h', 0, 0, 0, 0,
        b'A', b'T', b'T', b'X', 0, 0, 2002, 0, 102, 10, 4, 5, 5, 46, 0, 0,
        0, 64, 0, 0, 0, 0, 0, 0, 0, 0, 44, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let _ = std::fs::write(&file, &wav_header);
    file
}

pub fn setup_temp_midi_file() -> PathBuf {
    let temp = get_temp_dir();
    let file = temp.join("test.mid");
    let midi_header = vec![
        0x4D, 0x54, 0x68, 0x64, // 'MThd'
        6, 0, 0, 0, // 6 bytes header
        0, 0, 0, 0, // 0 divisions (bpm)
        70, 0, 0, 0, // 70 default ticks per quarter note
        0, 0, 0, 0, // 1 track
    ];
    let _ = std::fs::write(&file, &midi_header);
    file
}

pub fn setup_temp_project_file() -> PathBuf {
    let temp = get_temp_dir();
    let file = temp.join("test.project");
    let _ = std::fs::write(&file, b"fake project content");
    file
}

pub fn setup_temp_preset_file() -> PathBuf {
    let temp = get_temp_dir();
    let file = temp.join("test.presets");
    let preset_content = br#"<?xml version="1.0"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Manufacturer</key>
    <string>Fake Manufacturer</string>
    <key>Name</key>
    <string>Fake Preset</string>
    <key>Type</key>
    <string>AU</string>
</dict>
</plist>"#;
    let _ = std::fs::write(&file, preset_content);
    file
}

pub fn cleanup_temp_dir() {
    let temp = get_temp_dir();
    let _ = std::fs::remove_dir_all(&temp);
}
