//! Verification that indexed video extensions use host-side transcode and (when ffmpeg + audio-engine
//! are available) that `waveform_preview` returns peaks for synthetically muxed samples.
//!
//! **Huge MP4:** the UI always calls `waveform_preview` for video (no file-size skip); host transcode
//! bounds work (~300 s). The WebView still skips the expensive `<video>` frame-sampling fallback above
//! ~96 MiB (`VIDEO_VISUAL_WAVEFORM_MAX_FILE_BYTES` in `video.js`).
//!
//! **HTML5 `<video>` playback** is WebView- and codec-dependent; this suite only checks container
//! muxing (`ffprobe`) + the Rust transcode + JUCE decode path. Run the ignored test with:
//! `cargo test -p audio-haxor --test video_waveform_pipeline_verify video_waveform_pipeline_all_indexed_extensions -- --ignored --nocapture --test-threads=1`

use app_lib::audio_engine::{self, spawn_audio_engine_request};
use app_lib::video_scanner::VIDEO_EXTENSIONS;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn every_indexed_video_extension_triggers_host_transcode() {
    for ext in VIDEO_EXTENSIONS {
        let path = PathBuf::from(format!("/fake/dir/sample{ext}"));
        assert!(
            app_lib::path_needs_video_waveform_transcode(&path),
            "extension {ext} must be routed through host transcode for waveform IPC"
        );
    }
}

fn ffmpeg_ok() -> bool {
    Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "quiet", "-version"])
        .status()
        .is_ok_and(|s| s.success())
}

/// Returns `true` when the file has at least one video and one audio stream.
fn ffprobe_has_video_and_audio(path: &Path) -> bool {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .output();
    let Ok(o) = out else {
        return false;
    };
    if !o.status.success() {
        return false;
    }
    let s = String::from_utf8_lossy(&o.stdout);
    let mut has_v = false;
    let mut has_a = false;
    for line in s.lines() {
        for part in line.split(',') {
            match part.trim() {
                "video" => has_v = true,
                "audio" => has_a = true,
                _ => {}
            }
        }
    }
    has_v && has_a
}

fn mux_test_clip(path: &Path, ext_trim: &str) -> bool {
    let base = [
        "-hide_banner",
        "-loglevel",
        "error",
        "-nostdin",
        "-f",
        "lavfi",
        "-i",
        "testsrc=size=320x240:rate=1",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:sample_rate=44100",
        "-t",
        "2",
        "-shortest",
    ];
    let mut c = Command::new("ffmpeg");
    c.args(base);
    match ext_trim {
        "mp4" | "m4v" => {
            c.args([
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-movflags",
                "+faststart",
            ]);
        }
        "mov" => {
            c.args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac"]);
        }
        "mkv" => {
            c.args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac"]);
        }
        "webm" => {
            c.args([
                "-c:v",
                "libvpx",
                "-deadline",
                "good",
                "-cpu-used",
                "8",
                "-b:v",
                "256k",
                "-c:a",
                "libopus",
                "-b:a",
                "96k",
            ]);
        }
        "avi" => {
            c.args(["-c:v", "mpeg4", "-c:a", "libmp3lame", "-q:a", "6"]);
        }
        "mpg" | "mpeg" => {
            c.args(["-f", "mpeg", "-c:v", "mpeg2video", "-c:a", "mp2"]);
        }
        "wmv" => {
            c.args(["-c:v", "wmv2", "-c:a", "wmav2"]);
        }
        "flv" => {
            c.args(["-c:v", "flv", "-c:a", "libmp3lame", "-q:a", "6"]);
        }
        "ogv" => {
            c.args(["-c:v", "libtheora", "-c:a", "libvorbis"]);
        }
        "3gp" => {
            c.args([
                "-c:v", "h263", "-s", "176x144", "-c:a", "aac", "-ar", "44100", "-ac", "1",
            ]);
        }
        "mts" | "m2ts" => {
            c.args(["-f", "mpegts", "-c:v", "mpeg2video", "-c:a", "aac"]);
        }
        _ => return false,
    }
    c.args(["-y"]).arg(path);
    c.status().is_ok_and(|s| s.success())
        && path.is_file()
        && path.metadata().is_ok_and(|m| m.len() > 0)
}

fn resolve_audio_engine_for_test() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("AUDIO_HAXOR_AUDIO_ENGINE") {
        let pb = PathBuf::from(p.trim());
        if pb.is_file() {
            return Some(pb);
        }
    }
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri has parent workspace root")
        .to_path_buf();
    for rel in [
        "audio-engine-artifacts/release/audio-engine",
        "audio-engine-artifacts/debug/audio-engine",
        "target/release/audio-engine",
        "target/debug/audio-engine",
    ] {
        let c = repo.join(rel);
        if c.is_file() {
            return Some(c);
        }
    }
    None
}

#[test]
#[ignore = "requires ffmpeg + audio-engine; run with: cargo test -p audio-haxor --test video_waveform_pipeline_verify video_waveform_pipeline_all_indexed_extensions -- --ignored --nocapture --test-threads=1"]
fn video_waveform_pipeline_all_indexed_extensions() {
    assert!(
        ffmpeg_ok(),
        "ffmpeg not found on PATH (needed to synthesize test clips)"
    );
    let engine_bin = resolve_audio_engine_for_test().unwrap_or_else(|| {
        panic!(
            "audio-engine binary not found; build it or set AUDIO_HAXOR_AUDIO_ENGINE to its absolute path"
        )
    });

    // SAFETY: `set_var` is only safe here because this test is run with `--test-threads=1` (see ignore note).
    unsafe {
        std::env::set_var(
            "AUDIO_HAXOR_AUDIO_ENGINE",
            engine_bin.to_string_lossy().as_ref(),
        );
    }
    let _ = audio_engine::restart_audio_engine_child();

    let ping = spawn_audio_engine_request(&json!({ "cmd": "ping" }));
    assert!(
        ping.as_ref()
            .is_ok_and(|v| v.get("ok") == Some(&json!(true))),
        "audio-engine ping failed: {ping:?}"
    );

    let dir = std::env::temp_dir().join(format!(
        "ah_vwave_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).expect("temp clip dir");
    let mut mux_failed = Vec::new();
    let mut probe_failed = Vec::new();
    let mut waveform_failed = Vec::new();
    let mut skipped_optional = Vec::new();

    for ext in VIDEO_EXTENSIONS {
        let ext_trim = ext.trim_start_matches('.');
        let path = dir.join(format!("clip.{ext_trim}"));
        if !mux_test_clip(&path, ext_trim) {
            if *ext == ".ogv" {
                eprintln!(
                    "note: skipping {ext} — ffmpeg lacks libtheora in this build (OGV optional)"
                );
                skipped_optional.push(*ext);
                continue;
            }
            mux_failed.push(*ext);
            continue;
        }
        if !ffprobe_has_video_and_audio(&path) {
            probe_failed.push(*ext);
            continue;
        }
        let req = json!({
            "cmd": "waveform_preview",
            "path": path.to_string_lossy(),
            "width_px": 64,
        });
        let res = spawn_audio_engine_request(&req);
        let ok = res
            .as_ref()
            .is_ok_and(|v| v.get("ok") == Some(&json!(true)));
        let peaks = res
            .as_ref()
            .ok()
            .and_then(|v| v.get("peaks"))
            .and_then(|p| p.as_array());
        let peaks_ok = peaks.is_some_and(|a| !a.is_empty());
        if !ok || !peaks_ok {
            waveform_failed.push(format!(
                "{ext} -> {:?}",
                res.as_ref().ok().map(|v| v.get("error").cloned())
            ));
        }
    }

    let _ = audio_engine::shutdown_audio_engine_child();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        mux_failed.is_empty(),
        "ffmpeg could not mux test clip for: {mux_failed:?} (install encoders or adjust recipes)"
    );
    assert!(
        probe_failed.is_empty(),
        "ffprobe missing video/audio for: {probe_failed:?}"
    );
    assert!(
        waveform_failed.is_empty(),
        "waveform_preview failed for: {waveform_failed:?}"
    );
    if !skipped_optional.is_empty() {
        eprintln!("optional formats skipped (missing encoder): {skipped_optional:?}");
    }
}
