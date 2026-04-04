#[test]
fn test_lufs_temp_wav_written_and_measured() {
    let temp = std::env::temp_dir().join("audio_haxor_lufs_test");
    std::fs::create_dir_all(&temp).unwrap();
    let file = temp.join("file.wav");
    std::fs::write(&file, vec![0u8; 44]).unwrap();
    assert!(file.exists());
    // Too short for LUFS pipeline (< 1024 samples decoded)
    assert!(app_lib::lufs::measure_lufs(file.to_str().unwrap()).is_none());
    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn test_lufs_directory_path_returns_none() {
    let temp = std::env::temp_dir().join("audio_haxor_lufs_dironly");
    std::fs::create_dir_all(&temp).unwrap();
    assert!(
        app_lib::lufs::measure_lufs(&format!("{}", temp.display())).is_none(),
        "directory path should not yield LUFS"
    );
    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn test_lufs_nonexistent_file() {
    assert!(app_lib::lufs::measure_lufs("/nonexistent/audio_haxor/missing.wav").is_none());
}
