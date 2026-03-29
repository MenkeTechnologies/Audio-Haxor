use criterion::{black_box, criterion_group, criterion_main, Criterion};

use app_lib::kvr::{compare_versions, extract_version, parse_version};

fn bench_parse_version(c: &mut Criterion) {
    c.bench_function("parse_version simple", |b| {
        b.iter(|| parse_version(black_box("1.2.3")))
    });
    c.bench_function("parse_version long", |b| {
        b.iter(|| parse_version(black_box("10.24.3.1.0")))
    });
    c.bench_function("parse_version empty", |b| {
        b.iter(|| parse_version(black_box("")))
    });
}

fn bench_compare_versions(c: &mut Criterion) {
    c.bench_function("compare_versions equal", |b| {
        b.iter(|| compare_versions(black_box("1.2.3"), black_box("1.2.3")))
    });
    c.bench_function("compare_versions different", |b| {
        b.iter(|| compare_versions(black_box("1.2.3"), black_box("2.0.0")))
    });
    c.bench_function("compare_versions uneven", |b| {
        b.iter(|| compare_versions(black_box("1.2"), black_box("1.2.3.4")))
    });
}

fn bench_extract_version(c: &mut Criterion) {
    let html_with_version = r#"<div class="product-version">Version 3.5.1</div>"#;
    let html_no_version = r#"<div class="product-info">Some plugin description</div>"#;

    c.bench_function("extract_version found", |b| {
        b.iter(|| extract_version(black_box(html_with_version)))
    });
    c.bench_function("extract_version not found", |b| {
        b.iter(|| extract_version(black_box(html_no_version)))
    });
}

criterion_group!(
    benches,
    bench_parse_version,
    bench_compare_versions,
    bench_extract_version,
);
criterion_main!(benches);
