use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use search_and_replace::{parse_pattern, SearchAndReplace};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn cleanup_replacement_artifacts(results: &[search_and_replace::FileMatches]) {
    for result in results {
        if let Some(path) = &result.replacement_file_path {
            let _ = fs::remove_file(path);
        }
    }
}

fn bench_parse_pattern(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_pattern");

    group.bench_function("literal", |b| {
        b.iter(|| {
            black_box(parse_pattern(
                black_box("JustFailIfThisStringIsFound"),
                black_box(false),
                black_box(false),
            ))
        })
    });

    group.bench_function("regex", |b| {
        b.iter(|| {
            black_box(parse_pattern(
                black_box("/Bad\\s*Regexp/"),
                black_box(false),
                black_box(false),
            ))
        })
    });

    group.bench_function("regex_insensitive_extended", |b| {
        b.iter(|| {
            black_box(parse_pattern(
                black_box("/( *\\/\\/\\/ +([@\\\\]param)) +(\\w+) *(\\[(in|out|in, *out)\\])?"),
                black_box(true),
                black_box(true),
            ))
        })
    });

    group.finish();
}

fn bench_parse_files_fixed_fixtures(c: &mut Criterion) {
    let bad = fixture_path("bad_content.txt");
    let good = fixture_path("good_content.txt");
    let files = vec![bad, good];

    let literal = SearchAndReplace::new(
        files.clone(),
        "foobar",
        false,
        false,
        Some("fooBAZ".to_string()),
    );
    let literal_insensitive =
        SearchAndReplace::new(files.clone(), "there are so many", true, false, None);
    let regex = SearchAndReplace::new(files.clone(), "/Bad\\s*Regexp/", false, false, None);

    let mut group = c.benchmark_group("parse_files_fixtures");

    group.bench_function("literal_with_replacement", |b| {
        b.iter(|| {
            let results = literal.parse_files();
            let hit_count: usize = results.iter().map(|m| m.len()).sum();
            cleanup_replacement_artifacts(&results);
            black_box(hit_count)
        })
    });

    group.bench_function("literal_insensitive_no_replacement", |b| {
        b.iter(|| {
            let results = literal_insensitive.parse_files();
            let hit_count: usize = results.iter().map(|m| m.len()).sum();
            black_box(hit_count)
        })
    });

    group.bench_function("regex_no_replacement", |b| {
        b.iter(|| {
            let results = regex.parse_files();
            let hit_count: usize = results.iter().map(|m| m.len()).sum();
            black_box(hit_count)
        })
    });

    group.finish();
}

fn bench_parse_files_scaling(c: &mut Criterion) {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir for benchmarks");
    let mut group = c.benchmark_group("parse_files_scaling");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));

    // Keep runs practical while still surfacing obvious nonlinear regressions.
    let cases = [
        (500usize, "500_lines"),
        (1_000usize, "1k_lines"),
        (2_000usize, "2k_lines"),
    ];

    for (line_count, label) in cases {
        let path = temp_dir.path().join(format!("{}.txt", label));
        let mut content = String::with_capacity(line_count * 64);
        for i in 0..line_count {
            if i % 17 == 0 {
                content.push_str("Here is one: foobar and maybe BadRegexp\n");
            } else {
                content.push_str("There is nothing wrong with this content.\n");
            }
        }
        fs::write(&path, content).expect("failed to write benchmark fixture");

        let literal = SearchAndReplace::new(
            vec![path.clone()],
            "foobar",
            false,
            false,
            Some("fooBAZ".to_string()),
        );
        let regex = SearchAndReplace::new(vec![path], "/Bad\\s*Regexp/", false, false, None);

        group.bench_with_input(
            BenchmarkId::new("literal_with_replacement", label),
            &line_count,
            |b, _| {
                b.iter(|| {
                    let results = literal.parse_files();
                    let hit_count: usize = results.iter().map(|m| m.len()).sum();
                    cleanup_replacement_artifacts(&results);
                    black_box(hit_count)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("regex_no_replacement", label),
            &line_count,
            |b, _| {
                b.iter(|| {
                    let results = regex.parse_files();
                    let hit_count: usize = results.iter().map(|m| m.len()).sum();
                    black_box(hit_count)
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_parse_pattern,
    bench_parse_files_fixed_fixtures,
    bench_parse_files_scaling
);
criterion_main!(benches);
