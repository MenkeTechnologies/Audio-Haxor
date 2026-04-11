use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::path::Path;

use app_lib::audio_scanner::{format_size as audio_format_size, get_audio_metadata};
use app_lib::daw_scanner::{
    daw_name_for_format, ext_matches, format_size as daw_format_size, is_package_ext,
};
use app_lib::history::{gen_id, radix_string};
use app_lib::kvr::{compare_versions, extract_download_url, extract_version, parse_version};
use app_lib::scanner::{format_size as scanner_format_size, get_plugin_type};

// ── Version parsing & comparison ──

fn bench_parse_version(c: &mut Criterion) {
    let mut g = c.benchmark_group("parse_version");
    g.bench_function("simple", |b| b.iter(|| parse_version(black_box("1.2.3"))));
    g.bench_function("long", |b| {
        b.iter(|| parse_version(black_box("10.24.3.1.0")))
    });
    g.bench_function("empty", |b| b.iter(|| parse_version(black_box(""))));
    g.bench_function("unknown", |b| {
        b.iter(|| parse_version(black_box("Unknown")))
    });
    g.finish();
}

fn bench_compare_versions(c: &mut Criterion) {
    let mut g = c.benchmark_group("compare_versions");
    g.bench_function("equal", |b| {
        b.iter(|| compare_versions(black_box("1.2.3"), black_box("1.2.3")))
    });
    g.bench_function("different", |b| {
        b.iter(|| compare_versions(black_box("1.2.3"), black_box("2.0.0")))
    });
    g.bench_function("uneven_lengths", |b| {
        b.iter(|| compare_versions(black_box("1.2"), black_box("1.2.3.4")))
    });
    g.bench_function("long_equal", |b| {
        b.iter(|| compare_versions(black_box("10.24.3.1.0"), black_box("10.24.3.1.0")))
    });
    g.finish();
}

// ── HTML extraction (regex-heavy) ──

fn bench_extract_version(c: &mut Criterion) {
    let mut g = c.benchmark_group("extract_version");

    let html_version_tag = r#"<div class="product-version">Version 3.5.1</div>"#;
    let html_v_prefix = r#"<span>v2.1.0 released today</span>"#;
    let html_no_version = r#"<div class="product-info">Some plugin description without any version info at all</div>"#;
    let html_large = format!(
        r#"<html><body>{}<div class="footer">Version 1.0.2</div></body></html>"#,
        "<p>Lorem ipsum dolor sit amet. </p>".repeat(200)
    );

    g.bench_function("version_tag", |b| {
        b.iter(|| extract_version(black_box(html_version_tag)))
    });
    g.bench_function("v_prefix", |b| {
        b.iter(|| extract_version(black_box(html_v_prefix)))
    });
    g.bench_function("not_found", |b| {
        b.iter(|| extract_version(black_box(html_no_version)))
    });
    g.bench_function("large_html", |b| {
        b.iter(|| extract_version(black_box(&html_large)))
    });
    g.finish();
}

fn bench_extract_download_url(c: &mut Criterion) {
    let mut g = c.benchmark_group("extract_download_url");

    let html_with_download =
        r#"<a href="https://example.com/download/plugin-v1.2.3-mac.dmg">Download for Mac</a>"#;
    let html_no_download = r#"<div class="content">No links here</div>"#;
    let html_multiple = r#"
        <a href="https://example.com/download/plugin-win.exe">Windows</a>
        <a href="https://example.com/download/plugin-mac.dmg">Mac</a>
        <a href="https://example.com/download/plugin-linux.tar.gz">Linux</a>
    "#;

    g.bench_function("found", |b| {
        b.iter(|| extract_download_url(black_box(html_with_download)))
    });
    g.bench_function("not_found", |b| {
        b.iter(|| extract_download_url(black_box(html_no_download)))
    });
    g.bench_function("multiple_links", |b| {
        b.iter(|| extract_download_url(black_box(html_multiple)))
    });
    g.finish();
}

// ── Format size (across modules) ──

fn bench_format_size(c: &mut Criterion) {
    let mut g = c.benchmark_group("format_size");

    g.bench_function("audio/zero", |b| b.iter(|| audio_format_size(black_box(0))));
    g.bench_function("audio/bytes", |b| {
        b.iter(|| audio_format_size(black_box(500)))
    });
    g.bench_function("audio/kb", |b| {
        b.iter(|| audio_format_size(black_box(1024)))
    });
    g.bench_function("audio/mb", |b| {
        b.iter(|| audio_format_size(black_box(1_048_576)))
    });
    g.bench_function("audio/gb", |b| {
        b.iter(|| audio_format_size(black_box(1_073_741_824)))
    });
    g.bench_function("scanner/large", |b| {
        b.iter(|| scanner_format_size(black_box(3_500_000_000)))
    });
    g.bench_function("daw/large", |b| {
        b.iter(|| daw_format_size(black_box(3_500_000_000)))
    });
    g.finish();
}

// ── DAW scanner utilities ──

fn bench_daw_ext_matches(c: &mut Criterion) {
    let mut g = c.benchmark_group("daw_ext_matches");

    g.bench_function("als_hit", |b| {
        b.iter(|| ext_matches(black_box(Path::new("song.als"))))
    });
    g.bench_function("logicx_hit", |b| {
        b.iter(|| ext_matches(black_box(Path::new("project.logicx"))))
    });
    g.bench_function("dawproject_hit", |b| {
        b.iter(|| ext_matches(black_box(Path::new("my.dawproject"))))
    });
    g.bench_function("miss", |b| {
        b.iter(|| ext_matches(black_box(Path::new("readme.txt"))))
    });
    g.bench_function("no_ext", |b| {
        b.iter(|| ext_matches(black_box(Path::new("Makefile"))))
    });
    g.finish();
}

fn bench_daw_name_for_format(c: &mut Criterion) {
    let mut g = c.benchmark_group("daw_name_for_format");

    g.bench_function("ALS", |b| b.iter(|| daw_name_for_format(black_box("ALS"))));
    g.bench_function("FLP", |b| b.iter(|| daw_name_for_format(black_box("FLP"))));
    g.bench_function("LOGICX", |b| {
        b.iter(|| daw_name_for_format(black_box("LOGICX")))
    });
    g.bench_function("unknown", |b| {
        b.iter(|| daw_name_for_format(black_box("NOPE")))
    });
    g.finish();
}

fn bench_is_package_ext(c: &mut Criterion) {
    let mut g = c.benchmark_group("is_package_ext");

    g.bench_function("logicx_true", |b| {
        b.iter(|| is_package_ext(black_box(Path::new("MySong.logicx"))))
    });
    g.bench_function("band_true", |b| {
        b.iter(|| is_package_ext(black_box(Path::new("MySong.band"))))
    });
    g.bench_function("als_false", |b| {
        b.iter(|| is_package_ext(black_box(Path::new("MySong.als"))))
    });
    g.finish();
}

// ── Plugin scanner utilities ──

fn bench_get_plugin_type(c: &mut Criterion) {
    let mut g = c.benchmark_group("get_plugin_type");

    g.bench_function("vst3", |b| b.iter(|| get_plugin_type(black_box(".vst3"))));
    g.bench_function("component", |b| {
        b.iter(|| get_plugin_type(black_box(".component")))
    });
    g.bench_function("unknown", |b| b.iter(|| get_plugin_type(black_box(".xyz"))));
    g.finish();
}

// ── History utilities ──

fn bench_radix_string(c: &mut Criterion) {
    let mut g = c.benchmark_group("radix_string");

    g.bench_function("base36_small", |b| {
        b.iter(|| radix_string(black_box(255), black_box(36)))
    });
    g.bench_function("base36_large", |b| {
        b.iter(|| radix_string(black_box(1_700_000_000_000), black_box(36)))
    });
    g.bench_function("base16", |b| {
        b.iter(|| radix_string(black_box(0xDEADBEEF), black_box(16)))
    });
    g.bench_function("base2", |b| {
        b.iter(|| radix_string(black_box(0xFF), black_box(2)))
    });
    g.bench_function("zero", |b| {
        b.iter(|| radix_string(black_box(0), black_box(36)))
    });
    g.finish();
}

fn bench_gen_id(c: &mut Criterion) {
    c.bench_function("gen_id", |b| b.iter(gen_id));
}

// ── Audio metadata (filesystem) ──

fn bench_get_audio_metadata(c: &mut Criterion) {
    let mut g = c.benchmark_group("get_audio_metadata");

    // Benchmark error path (nonexistent file) - measures overhead without I/O
    g.bench_function("nonexistent", |b| {
        b.iter(|| get_audio_metadata(black_box("/tmp/__nonexistent_bench_file__.wav")))
    });

    // Create a minimal valid WAV file for benchmarking the parse path
    let tmp = std::env::temp_dir().join("upum_bench_wav.wav");
    {
        use std::io::Write;
        let data_size: u32 = 1000;
        let file_size: u32 = 36 + data_size;
        let mut header = [0u8; 44];
        header[0..4].copy_from_slice(b"RIFF");
        header[4..8].copy_from_slice(&file_size.to_le_bytes());
        header[8..12].copy_from_slice(b"WAVE");
        header[12..16].copy_from_slice(b"fmt ");
        header[16..20].copy_from_slice(&16u32.to_le_bytes());
        header[20..22].copy_from_slice(&1u16.to_le_bytes());
        header[22..24].copy_from_slice(&2u16.to_le_bytes());
        header[24..28].copy_from_slice(&44100u32.to_le_bytes());
        header[28..32].copy_from_slice(&176400u32.to_le_bytes());
        header[32..34].copy_from_slice(&4u16.to_le_bytes());
        header[34..36].copy_from_slice(&16u16.to_le_bytes());
        header[36..40].copy_from_slice(b"data");
        header[40..44].copy_from_slice(&data_size.to_le_bytes());
        let mut f = std::fs::File::create(&tmp).unwrap();
        f.write_all(&header).unwrap();
        f.write_all(&vec![0u8; data_size as usize]).unwrap();
    }
    let wav_path = tmp.to_string_lossy().to_string();
    g.bench_function("valid_wav", |b| {
        b.iter(|| get_audio_metadata(black_box(&wav_path)))
    });

    g.finish();
    let _ = std::fs::remove_file(&tmp);
}

criterion_group!(
    benches,
    bench_parse_version,
    bench_compare_versions,
    bench_extract_version,
    bench_extract_download_url,
    bench_format_size,
    bench_daw_ext_matches,
    bench_daw_name_for_format,
    bench_is_package_ext,
    bench_get_plugin_type,
    bench_radix_string,
    bench_gen_id,
    bench_get_audio_metadata,
);
criterion_main!(benches);
