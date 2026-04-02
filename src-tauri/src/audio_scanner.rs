//! Audio sample file scanner with metadata extraction.
//!
//! Discovers WAV, FLAC, AIFF, MP3, OGG, M4A, and AAC files across
//! the filesystem. Extracts audio metadata (sample rate, bit depth,
//! channels, duration) by reading file headers directly. Supports
//! symlink deduplication and parallel directory traversal via Rayon.

use crate::history::AudioSample;

/// Normalize macOS firmlink paths: /System/Volumes/Data/Users/... → /Users/...
/// On macOS, / and /System/Volumes/Data are the same volume linked via firmlinks.
/// canonicalize() doesn't resolve these, causing duplicate directory visits.
fn normalize_macos_path(p: std::path::PathBuf) -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    {
        let s = p.to_string_lossy();
        if s.starts_with("/System/Volumes/Data/") {
            return std::path::PathBuf::from(&s["/System/Volumes/Data".len()..]);
        }
    }
    p
}
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const AUDIO_EXTENSIONS: &[&str] = &[
    ".wav", ".mp3", ".aiff", ".aif", ".flac", ".ogg", ".m4a", ".wma", ".aac", ".opus", ".rex",
    ".rx2", ".sf2", ".sfz",
];

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".Trash",
    "$RECYCLE.BIN",
    "System Volume Information",
    ".cache",
    "__pycache__",
];

pub fn format_size(bytes: u64) -> String {
    crate::format_size(bytes)
}

pub fn get_audio_roots() -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut roots = Vec::new();

    #[cfg(target_os = "macos")]
    {
        roots.push(home.clone());
        roots.push(PathBuf::from("/Library/Audio"));
        roots.push(PathBuf::from("/Applications"));
        if let Ok(vols) = fs::read_dir("/Volumes") {
            for entry in vols.flatten() {
                let path = entry.path();
                if path.is_dir() || path.is_symlink() {
                    roots.push(path);
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        roots.push(home.clone());
        roots.push(PathBuf::from(
            std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".into()),
        ));
        roots.push(PathBuf::from(
            std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| "C:\\Program Files (x86)".into()),
        ));
        for c in b'C'..=b'Z' {
            let drive = format!("{}:\\", c as char);
            if Path::new(&drive).exists() {
                roots.push(PathBuf::from(drive));
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        roots.push(home.clone());
        roots.push(PathBuf::from("/usr/share/sounds"));
        roots.push(PathBuf::from("/usr/local/share/sounds"));
    }

    roots.sort();
    roots.dedup();
    roots.into_iter().filter(|r| r.exists()).collect()
}

pub fn walk_for_audio(
    roots: &[PathBuf],
    on_batch: &mut dyn FnMut(&[AudioSample], usize),
    should_stop: &(dyn Fn() -> bool + Sync),
    exclude: Option<HashSet<String>>,
    active_dirs: Option<Arc<Mutex<Vec<String>>>>,
) {
    let batch_size = 100;
    let stop = Arc::new(AtomicBool::new(false));
    let found = Arc::new(AtomicUsize::new(0));
    let active = active_dirs.unwrap_or_else(|| Arc::new(Mutex::new(Vec::new())));
    let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<AudioSample>>(256);
    let visited = Arc::new(Mutex::new(HashSet::new()));
    let exclude = Arc::new(exclude.unwrap_or_default());

    // Dedicated pool — limit threads to avoid FD exhaustion with parallel scans
    let roots_owned: Vec<PathBuf> = roots.to_vec();
    let stop2 = stop.clone();
    let found2 = found.clone();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get().max(4))
        .build()
        .unwrap();
    std::thread::spawn(move || {
        pool.install(|| {
            roots_owned.par_iter().for_each(|root| {
                if stop2.load(Ordering::Relaxed) {
                    return;
                }
                walk_dir_parallel(
                    root, 0, &visited, &tx, &found2, batch_size, &stop2, &exclude, &active,
                );
            });
        });
        drop(pool); // Release thread pool resources immediately
    });

    // Stream results to callback as they arrive, checking stop frequently
    let mut total_found = 0usize;
    loop {
        if should_stop() {
            stop.store(true, Ordering::Relaxed);
            while rx.try_recv().is_ok() {}
            break;
        }
        match rx.recv_timeout(std::time::Duration::from_millis(10)) {
            Ok(samples) => {
                total_found += samples.len();
                on_batch(&samples, total_found);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn walk_dir_parallel(
    dir: &Path,
    depth: u32,
    visited: &Arc<Mutex<HashSet<PathBuf>>>,
    tx: &std::sync::mpsc::SyncSender<Vec<AudioSample>>,
    found: &Arc<AtomicUsize>,
    batch_size: usize,
    stop: &Arc<AtomicBool>,
    exclude: &Arc<HashSet<String>>,
    active_dirs: &Arc<Mutex<Vec<String>>>,
) {
    if depth > 30 || stop.load(Ordering::Relaxed) {
        return;
    }

    // Canonicalize for visited-set dedup (resolves symlinks), normalize for macOS firmlinks
    let real_dir = match fs::canonicalize(dir) {
        Ok(p) => normalize_macos_path(p),
        Err(_) => normalize_macos_path(dir.to_path_buf()),
    };
    {
        let mut vis = visited.lock().unwrap_or_else(|e| e.into_inner());
        if !vis.insert(real_dir) {
            return;
        }
    }

    // Track active directory (rolling window of last 30 visited)
    let dir_str = dir.to_string_lossy().to_string();
    {
        let mut ad = active_dirs.lock().unwrap_or_else(|e| e.into_inner());
        ad.push(dir_str.clone());
        if ad.len() > 30 { let excess = ad.len() - 30; ad.drain(..excess); }
    }

    let entries: Vec<_> = match fs::read_dir(dir) {
        Ok(e) => e.flatten().collect(),
        Err(_) => return,
    };

    let mut files = Vec::new();
    let mut subdirs = Vec::new();

    for entry in &entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') || SKIP_DIRS.contains(&name_str.as_ref()) {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            subdirs.push(path);
        } else if path.is_file() {
            files.push((path, dir.to_path_buf()));
        }
    }

    // Process files in this directory
    let mut batch = Vec::new();
    for (path, parent) in files {
        let ext = path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
            .unwrap_or_default();

        if AUDIO_EXTENSIONS.contains(&ext.as_str()) {
            let path_str = path.to_string_lossy().to_string();
            if exclude.contains(&path_str) {
                continue;
            }
            if let Ok(meta) = fs::metadata(&path) {
                // Skip empty or unreadable files
                if meta.len() == 0 {
                    continue;
                }
                // Skip files where we can't read timestamps (broken symlinks, unmounted volumes)
                if meta.modified().is_err() && meta.accessed().is_err() {
                    continue;
                }
                let sample_name = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                let modified = meta
                    .modified()
                    .ok()
                    .map(|t| {
                        let dt: chrono::DateTime<chrono::Utc> = t.into();
                        dt.format("%Y-%m-%d").to_string()
                    })
                    .unwrap_or_default();

                // Only read headers for formats with fast native parsers (WAV/AIFF/FLAC)
                // Skip symphonia probe for MP3/OGG/etc during scan — too slow for bulk
                let fast_fmt = matches!(ext.as_str(), ".wav" | ".aiff" | ".aif" | ".flac");
                let (dur, ch, sr, bps) = if fast_fmt {
                    let am = get_audio_metadata(path.to_str().unwrap_or(""));
                    (am.duration, am.channels, am.sample_rate, am.bits_per_sample)
                } else {
                    (None, None, None, None)
                };
                batch.push(AudioSample {
                    name: sample_name,
                    path: path.to_string_lossy().to_string(),
                    directory: parent.to_string_lossy().to_string(),
                    format: ext[1..].to_uppercase(),
                    size: meta.len(),
                    size_formatted: format_size(meta.len()),
                    modified,
                    duration: dur,
                    channels: ch,
                    sample_rate: sr,
                    bits_per_sample: bps,
                });
                found.fetch_add(1, Ordering::Relaxed);

                if batch.len() >= batch_size {
                    let _ = tx.send(batch);
                    batch = Vec::new();
                }
            }
        }
    }
    if !batch.is_empty() {
        let _ = tx.send(batch);
    }

    // Recurse into subdirectories in parallel
    subdirs.par_iter().for_each(|subdir| {
        walk_dir_parallel(
            subdir,
            depth + 1,
            visited,
            tx,
            found,
            batch_size,
            stop,
            exclude,
            active_dirs,
        );
    });

    // Remove dir from active list
}

// Audio metadata extraction
#[derive(Debug, Clone, serde::Serialize)]
pub struct AudioMetadata {
    #[serde(rename = "fullPath")]
    pub full_path: String,
    #[serde(rename = "fileName")]
    pub file_name: String,
    pub directory: String,
    pub format: String,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: u64,
    pub created: String,
    pub modified: String,
    pub accessed: String,
    pub permissions: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<u16>,
    #[serde(rename = "sampleRate", skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,
    #[serde(rename = "bitsPerSample", skip_serializing_if = "Option::is_none")]
    pub bits_per_sample: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn get_audio_metadata(file_path: &str) -> AudioMetadata {
    let path = Path::new(file_path);
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            return AudioMetadata {
                full_path: file_path.to_string(),
                file_name: String::new(),
                directory: String::new(),
                format: String::new(),
                size_bytes: 0,
                created: String::new(),
                modified: String::new(),
                accessed: String::new(),
                permissions: String::new(),
                channels: None,
                sample_rate: None,
                bits_per_sample: None,
                duration: None,
                error: Some(e.to_string()),
            };
        }
    };

    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
        .unwrap_or_default();

    let fmt_time = |t: std::io::Result<std::time::SystemTime>| -> String {
        t.ok()
            .map(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.to_rfc3339()
            })
            .unwrap_or_default()
    };

    #[cfg(unix)]
    let permissions = {
        use std::os::unix::fs::PermissionsExt;
        format!("0{:o}", meta.permissions().mode() & 0o777)
    };
    #[cfg(not(unix))]
    let permissions = String::new();

    let mut result = AudioMetadata {
        full_path: file_path.to_string(),
        file_name: path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default(),
        directory: path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        format: ext[1..].to_uppercase(),
        size_bytes: meta.len(),
        created: fmt_time(meta.created()),
        modified: fmt_time(meta.modified()),
        accessed: fmt_time(meta.accessed()),
        permissions,
        channels: None,
        sample_rate: None,
        bits_per_sample: None,
        duration: None,
        error: None,
    };

    // Parse audio headers
    match ext.as_str() {
        ".wav" => parse_wav(path, &mut result),
        ".aiff" | ".aif" => parse_aiff(path, &mut result),
        ".flac" => parse_flac(path, &mut result),
        ".mp3" | ".ogg" | ".m4a" | ".aac" | ".opus" | ".wma" => probe_with_symphonia(path, &mut result),
        _ => {}
    }

    result
}

/// Fast metadata probe using symphonia — reads codec params without decoding.
fn probe_with_symphonia(path: &Path, meta: &mut AudioMetadata) {
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = match symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
    {
        Ok(p) => p,
        Err(_) => return,
    };

    if let Some(track) = probed.format.default_track() {
        let params = &track.codec_params;
        if let Some(sr) = params.sample_rate {
            meta.sample_rate = Some(sr);
        }
        if let Some(ch) = params.channels {
            meta.channels = Some(ch.count() as u16);
        }
        if let Some(bps) = params.bits_per_sample {
            meta.bits_per_sample = Some(bps as u16);
        }
        // Duration from time base + n_frames
        if let (Some(tb), Some(n_frames)) = (params.time_base, params.n_frames) {
            let time = tb.calc_time(n_frames);
            meta.duration = Some(time.seconds as f64 + time.frac);
        }
    }
}

fn parse_wav(path: &Path, meta: &mut AudioMetadata) {
    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut header = [0u8; 44];
    if file.read_exact(&mut header).is_err() {
        return;
    }

    if &header[0..4] == b"RIFF" && &header[8..12] == b"WAVE" {
        meta.channels = Some(u16::from_le_bytes([header[22], header[23]]));
        meta.sample_rate = Some(u32::from_le_bytes([
            header[24], header[25], header[26], header[27],
        ]));
        let byte_rate = u32::from_le_bytes([header[28], header[29], header[30], header[31]]);
        meta.bits_per_sample = Some(u16::from_le_bytes([header[34], header[35]]));
        let data_size = u32::from_le_bytes([header[40], header[41], header[42], header[43]]);
        if byte_rate > 0 {
            meta.duration = Some(data_size as f64 / byte_rate as f64);
        }
    }
}

fn parse_aiff(path: &Path, meta: &mut AudioMetadata) {
    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut buf = [0u8; 512];
    let bytes_read = match file.read(&mut buf) {
        Ok(n) => n,
        Err(_) => return,
    };

    if bytes_read < 12 || &buf[0..4] != b"FORM" || &buf[8..12] != b"AIFF" {
        return;
    }

    let mut offset = 12usize;
    while offset + 8 < bytes_read {
        let chunk_id = &buf[offset..offset + 4];
        let chunk_size = u32::from_be_bytes([
            buf[offset + 4],
            buf[offset + 5],
            buf[offset + 6],
            buf[offset + 7],
        ]) as usize;

        if chunk_id == b"COMM" && offset + 18 < bytes_read {
            meta.channels = Some(u16::from_be_bytes([buf[offset + 8], buf[offset + 9]]));
            let num_frames = u32::from_be_bytes([
                buf[offset + 10],
                buf[offset + 11],
                buf[offset + 12],
                buf[offset + 13],
            ]);
            meta.bits_per_sample = Some(u16::from_be_bytes([buf[offset + 14], buf[offset + 15]]));

            // 80-bit extended float for sample rate
            let exponent = u16::from_be_bytes([buf[offset + 16], buf[offset + 17]]) as i32;
            let mantissa = u32::from_be_bytes([
                buf[offset + 18],
                buf[offset + 19],
                buf[offset + 20],
                buf[offset + 21],
            ]);
            let exp = exponent - 16383 - 31;
            let sample_rate = (mantissa as f64 * 2f64.powi(exp)).round() as u32;
            meta.sample_rate = Some(sample_rate);
            if sample_rate > 0 {
                meta.duration = Some(num_frames as f64 / sample_rate as f64);
            }
            break;
        }

        offset += 8 + chunk_size;
        if !chunk_size.is_multiple_of(2) {
            offset += 1;
        }
    }
}

fn parse_flac(path: &Path, meta: &mut AudioMetadata) {
    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut buf = [0u8; 42];
    if file.read_exact(&mut buf).is_err() {
        return;
    }

    if &buf[0..4] != b"fLaC" {
        return;
    }

    let sample_rate = ((buf[18] as u32) << 12) | ((buf[19] as u32) << 4) | ((buf[20] as u32) >> 4);
    let channels = ((buf[20] >> 1) & 0x07) + 1;
    let bits_per_sample = (((buf[20] & 1) as u16) << 4) | (((buf[21] >> 4) as u16) + 1);

    let total_samples = (((buf[21] & 0x0F) as u64) * (1u64 << 32))
        | ((buf[22] as u64) << 24)
        | ((buf[23] as u64) << 16)
        | ((buf[24] as u64) << 8)
        | (buf[25] as u64);

    meta.sample_rate = Some(sample_rate);
    meta.channels = Some(channels as u16);
    meta.bits_per_sample = Some(bits_per_sample);

    if sample_rate > 0 && total_samples > 0 {
        meta.duration = Some(total_samples as f64 / sample_rate as f64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(500), "500.0 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1_048_576), "1.0 MB");
        assert_eq!(format_size(1_073_741_824), "1.0 GB");
    }

    #[test]
    fn test_audio_extensions_complete() {
        for ext in &[".wav", ".mp3", ".flac", ".aiff", ".ogg", ".m4a"] {
            assert!(
                AUDIO_EXTENSIONS.contains(ext),
                "AUDIO_EXTENSIONS should contain {}",
                ext
            );
        }
    }

    #[test]
    fn test_skip_dirs_complete() {
        for dir in &["node_modules", ".git", ".Trash"] {
            assert!(SKIP_DIRS.contains(dir), "SKIP_DIRS should contain {}", dir);
        }
    }

    #[test]
    fn test_get_audio_roots_not_empty() {
        let roots = get_audio_roots();
        assert!(
            roots.iter().any(|r| r.exists()),
            "get_audio_roots should return at least one existing path"
        );
    }

    #[test]
    fn test_walk_for_audio_empty_dir() {
        let tmp = std::env::temp_dir().join("upum_test_walk_empty");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let mut total = 0usize;
        walk_for_audio(
            &[tmp.clone()],
            &mut |_batch, count| {
                total = count;
            },
            &|| false,
            None,
            None,
        );
        assert_eq!(total, 0);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_audio_finds_files() {
        let tmp = std::env::temp_dir().join("upum_test_walk_finds");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("test.wav"), b"fake wav data").unwrap();
        fs::write(tmp.join("test.txt"), b"not audio").unwrap();

        let mut found = Vec::new();
        walk_for_audio(
            &[tmp.clone()],
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| false,
            None,
            None,
        );
        assert_eq!(found.len(), 1);
        assert!(found[0].path.contains("test.wav"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_audio_stop() {
        let tmp = std::env::temp_dir().join("upum_test_walk_stop");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("test.wav"), b"fake wav data").unwrap();

        let mut found = Vec::new();
        walk_for_audio(
            &[tmp.clone()],
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| true,
            None,
            None,
        );
        assert_eq!(found.len(), 0);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_audio_skips_dotdirs() {
        let tmp = std::env::temp_dir().join("upum_test_walk_dotdirs");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join(".hidden")).unwrap();
        fs::create_dir_all(tmp.join("visible")).unwrap();
        fs::write(tmp.join(".hidden").join("test.wav"), b"hidden").unwrap();
        fs::write(tmp.join("visible").join("test.wav"), b"visible").unwrap();

        let mut found = Vec::new();
        walk_for_audio(
            &[tmp.clone()],
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| false,
            None,
            None,
        );
        assert_eq!(found.len(), 1);
        assert!(found[0].path.contains("visible"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_audio_skips_node_modules() {
        let tmp = std::env::temp_dir().join("upum_test_walk_nodemod");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("node_modules")).unwrap();
        fs::create_dir_all(tmp.join("music")).unwrap();
        fs::write(tmp.join("node_modules").join("test.wav"), b"nm").unwrap();
        fs::write(tmp.join("music").join("test.wav"), b"music").unwrap();

        let mut found = Vec::new();
        walk_for_audio(
            &[tmp.clone()],
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| false,
            None,
            None,
        );
        assert_eq!(found.len(), 1);
        assert!(found[0].path.contains("music"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_get_audio_metadata_nonexistent() {
        let meta = get_audio_metadata("/nonexistent/path/fake.wav");
        assert!(meta.error.is_some());
    }

    #[test]
    fn test_get_audio_metadata_wav() {
        let tmp = std::env::temp_dir().join("upum_test_meta_wav");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let wav_path = tmp.join("test.wav");

        let mut header = [0u8; 44];
        // RIFF header
        header[0..4].copy_from_slice(b"RIFF");
        let file_size: u32 = 44 - 8 + 1000;
        header[4..8].copy_from_slice(&file_size.to_le_bytes());
        header[8..12].copy_from_slice(b"WAVE");
        // fmt chunk
        header[12..16].copy_from_slice(b"fmt ");
        header[16..20].copy_from_slice(&16u32.to_le_bytes());
        header[20..22].copy_from_slice(&1u16.to_le_bytes()); // PCM
        header[22..24].copy_from_slice(&2u16.to_le_bytes()); // channels
        header[24..28].copy_from_slice(&44100u32.to_le_bytes()); // sample rate
        header[28..32].copy_from_slice(&176400u32.to_le_bytes()); // byte rate
        header[32..34].copy_from_slice(&4u16.to_le_bytes()); // block align
        header[34..36].copy_from_slice(&16u16.to_le_bytes()); // bits per sample
                                                              // data chunk
        header[36..40].copy_from_slice(b"data");
        header[40..44].copy_from_slice(&1000u32.to_le_bytes());

        let mut file = fs::File::create(&wav_path).unwrap();
        file.write_all(&header).unwrap();
        // Write some data bytes to match the data size
        file.write_all(&vec![0u8; 1000]).unwrap();

        let meta = get_audio_metadata(wav_path.to_str().unwrap());
        assert_eq!(meta.format, "WAV");
        assert_eq!(meta.channels, Some(2));
        assert_eq!(meta.sample_rate, Some(44100));
        assert_eq!(meta.bits_per_sample, Some(16));
        assert!(meta.error.is_none());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_get_audio_metadata_flac() {
        let tmp = std::env::temp_dir().join("upum_test_meta_flac");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let flac_path = tmp.join("test.flac");

        let mut buf = [0u8; 42];
        // fLaC magic
        buf[0..4].copy_from_slice(b"fLaC");
        // Metadata block header: last-block flag (0x80) + type 0 (STREAMINFO)
        buf[4] = 0x80;
        // Block size = 34 as 3-byte big-endian
        buf[5] = 0;
        buf[6] = 0;
        buf[7] = 34;
        // Min/max block size
        buf[8..10].copy_from_slice(&4096u16.to_be_bytes());
        buf[10..12].copy_from_slice(&4096u16.to_be_bytes());
        // Min/max frame size (3 bytes each, zeros)
        // bytes 12-17 are already 0
        // Sample rate (20 bits) + channels-1 (3 bits) + bps-1 high bit (1 bit)
        // 44100 Hz, 2 channels, 16 bits per sample
        // sample_rate = 44100 = 0xAC44
        // byte18 = (44100 >> 12) = 0x0A
        // byte19 = (44100 >> 4) & 0xFF = 0xC4 (44100 = 0xAC44, >> 4 = 0xAC4, & 0xFF = 0xC4)
        // byte20 = ((44100 & 0x0F) << 4) | ((2-1) << 1) | ((16-1) >> 4)
        //        = (0x04 << 4) | (1 << 1) | (15 >> 4)
        //        = 0x40 | 0x02 | 0x00 = 0x42
        buf[18] = 0x0A;
        buf[19] = 0xC4;
        buf[20] = 0x42;
        // byte21: bps-1 low 4 bits (15 & 0x0F = 0xF) << 4 | total_samples high 4 bits
        // total_samples = 44100 = 0x0000AC44
        // high 4 bits of 36-bit total = 0
        buf[21] = 0xF0;
        // bytes 22-25: total samples low 32 bits = 44100
        buf[22] = 0x00;
        buf[23] = 0x00;
        buf[24] = 0xAC;
        buf[25] = 0x44;
        // bytes 26-41: MD5 (zeros, already set)

        fs::write(&flac_path, &buf).unwrap();

        let meta = get_audio_metadata(flac_path.to_str().unwrap());
        assert_eq!(meta.format, "FLAC");
        assert_eq!(meta.sample_rate, Some(44100));
        assert_eq!(meta.channels, Some(2));
        // bits_per_sample parsing: (((buf[20] & 1) as u16) << 4) | ((buf[21] >> 4) as u16) + 1
        // = ((0x42 & 1) << 4) | (0xF0 >> 4) + 1 = (0 << 4) | 15 + 1 = 16
        assert_eq!(meta.bits_per_sample, Some(16));
        assert!(meta.error.is_none());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_audio_respects_depth_limit() {
        let tmp = std::env::temp_dir().join("upum_test_walk_depth");
        let _ = fs::remove_dir_all(&tmp);

        // Create a dir structure 32 levels deep (exceeds depth > 30 guard)
        let mut deep = tmp.clone();
        for i in 0..32 {
            deep = deep.join(format!("d{}", i));
        }
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("deep.wav"), b"deep wav").unwrap();

        let mut found = Vec::new();
        walk_for_audio(
            &[tmp.clone()],
            &mut |batch, _count| {
                found.extend_from_slice(batch);
            },
            &|| false,
            None,
            None,
        );
        assert!(
            !found.iter().any(|s| s.name == "deep"),
            "Should not find audio files deeper than 30 levels"
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_audio_batching() {
        let tmp = std::env::temp_dir().join("upum_test_walk_batching");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        for i in 0..120 {
            fs::write(tmp.join(format!("sample_{}.wav", i)), b"wav data").unwrap();
        }

        let mut batch_call_count = 0usize;
        walk_for_audio(
            &[tmp.clone()],
            &mut |_batch, _count| {
                batch_call_count += 1;
            },
            &|| false,
            None,
            None,
        );
        assert!(
            batch_call_count >= 2,
            "Expected on_batch called at least twice for 120 files with batch_size=100, got {}",
            batch_call_count
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_walk_for_audio_deduplicates_symlinks() {
        let tmp = std::env::temp_dir().join("upum_test_walk_symlinks");
        let _ = fs::remove_dir_all(&tmp);
        let subdir = tmp.join("originals");
        fs::create_dir_all(&subdir).unwrap();
        fs::write(subdir.join("test.wav"), b"wav data").unwrap();

        // Create a symlink to subdir
        #[cfg(unix)]
        {
            let link = tmp.join("linked");
            std::os::unix::fs::symlink(&subdir, &link).unwrap();

            let mut found = Vec::new();
            walk_for_audio(
                &[tmp.clone()],
                &mut |batch, _count| {
                    found.extend_from_slice(batch);
                },
                &|| false,
            None,
                None,
            );
            let wav_count = found.iter().filter(|s| s.name == "test").count();
            assert_eq!(
                wav_count, 1,
                "test.wav should be found exactly once despite symlink, found {}",
                wav_count
            );
        }
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_get_audio_metadata_aiff() {
        let tmp = std::env::temp_dir().join("upum_test_meta_aiff");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let aiff_path = tmp.join("test.aiff");

        // Build a minimal valid AIFF file
        let mut data = Vec::new();
        // FORM header
        data.extend_from_slice(b"FORM");
        // file_size - 8 placeholder (will fill after)
        let total_size: u32 = 4 + 8 + 18; // "AIFF" + COMM chunk header + COMM data
        data.extend_from_slice(&total_size.to_be_bytes());
        data.extend_from_slice(b"AIFF");
        // COMM chunk
        data.extend_from_slice(b"COMM");
        data.extend_from_slice(&18u32.to_be_bytes()); // chunk size
        data.extend_from_slice(&1u16.to_be_bytes()); // channels = 1
        data.extend_from_slice(&48000u32.to_be_bytes()); // num_frames = 48000
        data.extend_from_slice(&24u16.to_be_bytes()); // bits_per_sample = 24
                                                      // 80-bit extended float for sample rate 48000
                                                      // exponent = 16383 + 15 = 16398 = 0x400E
                                                      // mantissa = 48000 << 16 = 0xBB80_0000 (top 32 bits), lower 32 bits = 0
        data.extend_from_slice(&[0x40, 0x0E, 0xBB, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        fs::write(&aiff_path, &data).unwrap();

        let meta = get_audio_metadata(aiff_path.to_str().unwrap());
        assert_eq!(meta.format, "AIFF");
        assert_eq!(meta.channels, Some(1));
        assert_eq!(meta.sample_rate, Some(48000));
        assert_eq!(meta.bits_per_sample, Some(24));
        assert!(meta.error.is_none());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_format_size_boundary_values() {
        assert_eq!(format_size(1023), "1023.0 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1025), "1.0 KB");
        assert_eq!(format_size(1024 * 1024 - 1), "1024.0 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
    }

    #[test]
    fn test_format_size_zero() {
        assert_eq!(format_size(0), "0 B");
    }

    #[test]
    fn test_audio_extensions_includes_common() {
        for ext in &[".wav", ".mp3", ".flac"] {
            assert!(
                AUDIO_EXTENSIONS.contains(ext),
                "AUDIO_EXTENSIONS must include {}",
                ext
            );
        }
    }

    #[test]
    fn test_parse_wav_invalid() {
        let tmp = std::env::temp_dir().join("upum_test_parse_wav_invalid");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let path = tmp.join("garbage.wav");
        fs::write(
            &path,
            &[
                0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x11, 0x22, 0x33, 0xAA, 0xBB, 0xCC, 0xDD, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00,
            ],
        )
        .unwrap();

        let mut meta = AudioMetadata {
            full_path: path.to_string_lossy().to_string(),
            file_name: "garbage.wav".to_string(),
            directory: tmp.to_string_lossy().to_string(),
            format: "WAV".to_string(),
            size_bytes: 44,
            created: String::new(),
            modified: String::new(),
            accessed: String::new(),
            permissions: String::new(),
            channels: None,
            sample_rate: None,
            bits_per_sample: None,
            duration: None,
            error: None,
        };
        parse_wav(&path, &mut meta);
        // Should not crash; fields remain None since RIFF/WAVE magic doesn't match
        assert!(meta.channels.is_none());
        assert!(meta.sample_rate.is_none());
        let _ = fs::remove_dir_all(&tmp);
    }
}
