//! Video containers (MP4/MOV/MKV/…) are not readable by JUCE `AudioFormatManager` for
//! `waveform_preview` / `spectrogram_preview`. Before forwarding those IPC requests to the
//! AudioEngine, we extract the **first audio stream** (up to 300 s): **ffmpeg → mono PCM WAV**
//! first (faster than LAME MP3 + smaller JUCE decode cost), then **ffmpeg → MP3** (LAME), else
//! **Symphonia → mono PCM WAV** (ISO MP4 / Matroska / …).
//!
//! **Multi‑gigabyte files:** `ffmpeg` uses `-t 300`, so it does not decode the whole movie; JUCE only
//! reads the small temp WAV/MP3. Transcodes use **22.05 kHz** mono — enough for on-screen peaks and
//! ~half the samples vs 44.1 kHz. Symphonia caps decoded samples the same way.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::video_scanner::VIDEO_EXTENSIONS;

/// Matches JUCE `VisualPreview.cpp` `kMaxDurationSec` for `waveform_preview`.
const MAX_EXTRACT_SEC: u32 = 300;

/// Preview transcode sample rate — lower than 44.1 kHz cuts ffmpeg + JUCE work (~50%) with minimal
/// visual loss for bar waveforms.
const PREVIEW_EXTRACT_HZ: &str = "22050";

/// Temp file for a transcode; deleted on drop (best effort).
pub(crate) struct TempTranscoded {
    path: PathBuf,
}

impl Drop for TempTranscoded {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[inline]
fn ext_matches_video_list(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let lower = e.to_ascii_lowercase();
            let dot = format!(".{lower}");
            VIDEO_EXTENSIONS.iter().any(|vx| *vx == dot.as_str())
        })
        .unwrap_or(false)
}

/// True when this path should be transcoded before JUCE visual preview.
pub(crate) fn path_needs_container_extract(path: &Path) -> bool {
    ext_matches_video_list(path)
}

fn temp_transcode_path(suffix: &str) -> PathBuf {
    let id: u64 = rand::random();
    std::env::temp_dir().join(format!(
        "audio_haxor_wf_{}_{}{suffix}",
        std::process::id(),
        id
    ))
}

fn ffmpeg_preview_log_snippet(stderr: &[u8], code: Option<i32>) -> String {
    let err = String::from_utf8_lossy(stderr).trim().to_string();
    if err.is_empty() {
        return format!("exit code {:?}", code);
    }
    let collapsed: String = err
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    let collapsed = collapsed.split_whitespace().collect::<Vec<_>>().join(" ");
    const MAX: usize = 900;
    if collapsed.len() > MAX {
        format!("{}…", &collapsed[..MAX])
    } else {
        collapsed
    }
}

fn log_ffmpeg_preview_failure(stage: &str, src: &Path, stderr: &[u8], code: Option<i32>) {
    let detail = ffmpeg_preview_log_snippet(stderr, code);
    crate::write_app_log(format!(
        "WAVEFORM PREVIEW — ffmpeg {stage} extract failed | {} | {detail}",
        src.display()
    ));
}

/// Try `ffmpeg`: first audio stream → mono MP3 (≤ [`MAX_EXTRACT_SEC`]).
fn try_ffmpeg_extract_mp3(src: &Path) -> Option<TempTranscoded> {
    if !src.is_file() {
        return None;
    }
    let out = temp_transcode_path(".mp3");
    let out_s = out.to_string_lossy();
    let src_s = src.to_string_lossy();
    let t = MAX_EXTRACT_SEC.to_string();
    let child_out = match Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-y",
            "-threads",
            "0",
            "-i",
            src_s.as_ref(),
            "-t",
            t.as_str(),
            "-vn",
            "-ac",
            "1",
            "-ar",
            PREVIEW_EXTRACT_HZ,
            "-codec:a",
            "libmp3lame",
            "-q:a",
            "5",
            out_s.as_ref(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            let _ = fs::remove_file(&out);
            crate::write_app_log(format!(
                "WAVEFORM PREVIEW — ffmpeg mp3 could not run | {} | {e}",
                src.display()
            ));
            return None;
        }
    };
    if !child_out.status.success() {
        let _ = fs::remove_file(&out);
        log_ffmpeg_preview_failure("mp3", src, &child_out.stderr, child_out.status.code());
        return None;
    }
    let len = fs::metadata(&out).ok()?.len();
    if len == 0 {
        let _ = fs::remove_file(&out);
        return None;
    }
    Some(TempTranscoded { path: out })
}

/// `ffmpeg` without LAME: mono PCM in WAV (≤ [`MAX_EXTRACT_SEC`]).
fn try_ffmpeg_extract_wav_pcm(src: &Path) -> Option<TempTranscoded> {
    if !src.is_file() {
        return None;
    }
    let out = temp_transcode_path(".wav");
    let out_s = out.to_string_lossy();
    let src_s = src.to_string_lossy();
    let t = MAX_EXTRACT_SEC.to_string();
    let child_out = match Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-y",
            "-threads",
            "0",
            "-i",
            src_s.as_ref(),
            "-t",
            t.as_str(),
            "-vn",
            "-ac",
            "1",
            "-ar",
            PREVIEW_EXTRACT_HZ,
            "-f",
            "wav",
            out_s.as_ref(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            let _ = fs::remove_file(&out);
            crate::write_app_log(format!(
                "WAVEFORM PREVIEW — ffmpeg wav could not run | {} | {e}",
                src.display()
            ));
            return None;
        }
    };
    if !child_out.status.success() {
        let _ = fs::remove_file(&out);
        log_ffmpeg_preview_failure("wav", src, &child_out.stderr, child_out.status.code());
        return None;
    }
    let len = fs::metadata(&out).ok()?.len();
    if len == 0 {
        let _ = fs::remove_file(&out);
        return None;
    }
    Some(TempTranscoded { path: out })
}

fn write_wav_mono_i16(path: &Path, samples: &[i16], sample_rate: u32) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = fs::File::create(path)?;
    let data_len = (samples.len() * 2) as u32;
    let riff_payload = 36u32 + data_len;
    f.write_all(b"RIFF")?;
    f.write_all(&riff_payload.to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?; // PCM chunk size
    f.write_all(&1u16.to_le_bytes())?; // format: PCM
    f.write_all(&1u16.to_le_bytes())?; // channels
    f.write_all(&sample_rate.to_le_bytes())?;
    let byte_rate = sample_rate * 2;
    f.write_all(&byte_rate.to_le_bytes())?;
    f.write_all(&2u16.to_le_bytes())?; // block align
    f.write_all(&16u16.to_le_bytes())?; // bits per sample
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;
    for s in samples {
        f.write_all(&s.to_le_bytes())?;
    }
    Ok(())
}

/// Symphonia decode: first default audio track, mono f32, at most `max_seconds`.
fn decode_audio_track_mono_symphonia(path: &Path, max_seconds: u32) -> Option<(Vec<f32>, u32)> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = std::fs::File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;

    let mut format = probed.format;
    let track = format.default_track()?;
    let sample_rate = track.codec_params.sample_rate?;
    let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(1);
    let track_id = track.id;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .ok()?;

    let max_samples = sample_rate as usize * max_seconds as usize;
    let mut all_samples: Vec<f32> = Vec::new();

    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let spec = *decoded.spec();
        let duration = decoded.capacity();
        let mut sample_buf = SampleBuffer::<f32>::new(duration as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);

        let buf = sample_buf.samples();
        if channels > 1 {
            for chunk in buf.chunks_exact(channels) {
                let mono: f32 = chunk.iter().sum::<f32>() / channels as f32;
                all_samples.push(mono);
            }
        } else {
            all_samples.extend_from_slice(buf);
        }

        if all_samples.len() >= max_samples {
            all_samples.truncate(max_samples);
            break;
        }
    }

    if all_samples.is_empty() {
        return None;
    }

    Some((all_samples, sample_rate))
}

fn try_symphonia_extract_wav(src: &Path) -> Option<TempTranscoded> {
    let (mono_f32, sr) = decode_audio_track_mono_symphonia(src, MAX_EXTRACT_SEC)?;
    let out = temp_transcode_path(".wav");
    let mut mono_i16: Vec<i16> = Vec::with_capacity(mono_f32.len());
    for s in mono_f32 {
        let x = (s.clamp(-1.0, 1.0) * 32767.0).round() as i32;
        mono_i16.push(x.clamp(-32768, 32767) as i16);
    }
    write_wav_mono_i16(&out, &mono_i16, sr).ok()?;
    Some(TempTranscoded { path: out })
}

/// If `req` is `waveform_preview` / `spectrogram_preview` on a video-container path, transcode
/// to a temp WAV/MP3 (ffmpeg) or WAV (Symphonia) and return a cloned request pointing at that file.
/// Otherwise returns `req` unchanged. On transcode failure, returns the original request (JUCE will
/// respond `unsupported` and the UI can fall back).
pub(crate) fn rewrite_visual_preview_for_juce(req: &Value) -> (Value, Option<TempTranscoded>) {
    let cmd = match req.get("cmd").and_then(|c| c.as_str()) {
        Some(c) => c,
        None => return (req.clone(), None),
    };
    if cmd != "waveform_preview" && cmd != "spectrogram_preview" {
        return (req.clone(), None);
    }
    let path_str = match req.get("path").and_then(|p| p.as_str()) {
        Some(p) if !p.is_empty() => p,
        _ => return (req.clone(), None),
    };
    let path = Path::new(path_str);
    if !path_needs_container_extract(path) {
        return (req.clone(), None);
    }

    // Missing path (unmounted volume, deleted/moved file): skip transcode; log once per request.
    if !path.is_file() {
        crate::write_app_log(format!(
            "WAVEFORM PREVIEW — input not accessible (no such file, unmounted volume, or not a regular file) | {path_str}"
        ));
        return (req.clone(), None);
    }

    if let Some(tmp) = try_ffmpeg_extract_wav_pcm(path) {
        let mut out = req.clone();
        if let Some(obj) = out.as_object_mut() {
            obj.insert(
                "path".to_string(),
                Value::String(tmp.path.to_string_lossy().into_owned()),
            );
        }
        return (out, Some(tmp));
    }

    if let Some(tmp) = try_ffmpeg_extract_mp3(path) {
        let mut out = req.clone();
        if let Some(obj) = out.as_object_mut() {
            obj.insert(
                "path".to_string(),
                Value::String(tmp.path.to_string_lossy().into_owned()),
            );
        }
        return (out, Some(tmp));
    }

    if let Some(tmp) = try_symphonia_extract_wav(path) {
        let mut out = req.clone();
        if let Some(obj) = out.as_object_mut() {
            obj.insert(
                "path".to_string(),
                Value::String(tmp.path.to_string_lossy().into_owned()),
            );
        }
        return (out, Some(tmp));
    }

    crate::write_app_log(format!(
        "WAVEFORM PREVIEW — container audio extract failed after ffmpeg (wav, mp3) and symphonia | {path_str}"
    ));
    (req.clone(), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mp4_needs_extract_mov_mkv_too() {
        assert!(path_needs_container_extract(Path::new("/x/foo.MP4")));
        assert!(path_needs_container_extract(Path::new(r"C:\v\clip.mov")));
        assert!(path_needs_container_extract(Path::new("/m/file.mkv")));
    }

    #[test]
    fn wav_and_plain_audio_do_not_need_extract() {
        assert!(!path_needs_container_extract(Path::new("/a/x.wav")));
        assert!(!path_needs_container_extract(Path::new("/a/x.mp3")));
    }
}
