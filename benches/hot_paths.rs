//! Benchmarks for hot paths identified in research.md section 10.

use std::hint::black_box;

use {
    criterion::{Criterion, criterion_group, criterion_main},
    serde_json::{Value, from_str},
};

use qobuz_api::signing::{sign_request, sign_track_file_url};

/// Benchmarks request signature generation (MD5 hashing).
fn bench_sign_request(criterion: &mut Criterion) {
    let mut params: Vec<(String, String)> = vec![
        ("app_id".into(), "123456".into()),
        ("limit".into(), "50".into()),
        ("offset".into(), "0".into()),
        ("query".into(), "Dark Side of the Moon".into()),
        ("request_ts".into(), "1710000000".into()),
    ];

    criterion.bench_function("sign_request", |b| {
        b.iter(|| {
            black_box(sign_request(
                black_box("GET"),
                black_box("/album/search"),
                black_box(&mut params),
                black_box("secret_key_12345"),
            ));
        });
    });
}

/// Benchmarks track file URL signature generation.
fn bench_sign_track_file_url(criterion: &mut Criterion) {
    criterion.bench_function("sign_track_file_url", |b| {
        b.iter(|| {
            black_box(sign_track_file_url(
                black_box(6),
                black_box(12345),
                black_box("1710000000"),
                black_box("secret_key_12345"),
            ));
        });
    });
}

/// Benchmarks JSON search result deserialization.
fn bench_search_deserialization(criterion: &mut Criterion) {
    let json = r#"{"albums":{"items":[{"id":"123","title":"Kind of Blue","artist":{"id":1,"name":"Miles Davis"},"tracks_count":5}],"total":1}}"#;

    criterion.bench_function("search_result_deserialization", |b| {
        b.iter(|| {
            black_box(from_str::<Value>(black_box(json)).unwrap_or_default());
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = bench_sign_request, bench_sign_track_file_url, bench_search_deserialization
);
criterion_main!(benches);
