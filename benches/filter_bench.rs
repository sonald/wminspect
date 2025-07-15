use criterion::{black_box, criterion_group, criterion_main, Criterion};
use wminspect::dsl::filter::{scan_tokens, parse_rule, wild_match, Filter};

fn benchmark_tokenizer(c: &mut Criterion) {
    let simple_rule = "name = test";
    let complex_rule = "any(all(geom.width > 400, geom.height > 300), not(name = dde*))";
    let very_complex_rule = "any(all(geom.x > 0, geom.y > 0, geom.width > 400, geom.height > 300), all(name = firefox*, attrs.map_state = viewable), not(any(name = dde*, name = deepin*)))";

    c.bench_function("tokenize_simple", |b| {
        b.iter(|| scan_tokens(black_box(simple_rule)))
    });

    c.bench_function("tokenize_complex", |b| {
        b.iter(|| scan_tokens(black_box(complex_rule)))
    });

    c.bench_function("tokenize_very_complex", |b| {
        b.iter(|| scan_tokens(black_box(very_complex_rule)))
    });
}

fn benchmark_parser(c: &mut Criterion) {
    let simple_tokens = scan_tokens("name = test");
    let complex_tokens = scan_tokens("any(all(geom.width > 400, geom.height > 300), not(name = dde*))");

    c.bench_function("parse_simple", |b| {
        b.iter(|| {
            let mut tokens = simple_tokens.clone();
            parse_rule(black_box(&mut tokens))
        })
    });

    c.bench_function("parse_complex", |b| {
        b.iter(|| {
            let mut tokens = complex_tokens.clone();
            parse_rule(black_box(&mut tokens))
        })
    });
}

fn benchmark_wildcard_matching(c: &mut Criterion) {
    let patterns = vec![
        ("test*", "test123"),
        ("*test*", "this is a test string"),
        ("t?st", "test"),
        ("*", "any string"),
        ("complex*pattern*", "complex_long_pattern_match"),
    ];

    c.bench_function("wildcard_match", |b| {
        b.iter(|| {
            for (pattern, text) in &patterns {
                wild_match(black_box(pattern), black_box(text));
            }
        })
    });
}

fn benchmark_filter_creation(c: &mut Criterion) {
    let simple_rule = "name = test";
    let complex_rule = "any(all(geom.width > 400, geom.height > 300), not(name = dde*))";

    c.bench_function("filter_create_simple", |b| {
        b.iter(|| Filter::parse(black_box(simple_rule)))
    });

    c.bench_function("filter_create_complex", |b| {
        b.iter(|| Filter::parse(black_box(complex_rule)))
    });
}

fn benchmark_serialization(c: &mut Criterion) {
    let filter_items = {
        let mut tokens = scan_tokens("name = test: pin; id = 0x123");
        parse_rule(&mut tokens).unwrap()
    };

    c.bench_function("serialize_json", |b| {
        b.iter(|| serde_json::to_string(black_box(&filter_items)))
    });

    c.bench_function("serialize_bincode", |b| {
        b.iter(|| bincode::serialize(black_box(&filter_items)))
    });

    let json_data = serde_json::to_string(&filter_items).unwrap();
    let bincode_data = bincode::serialize(&filter_items).unwrap();

    c.bench_function("deserialize_json", |b| {
        b.iter(|| {
            let _: Vec<wminspect::dsl::filter::FilterItem> = 
                serde_json::from_str(black_box(&json_data)).unwrap();
        })
    });

    c.bench_function("deserialize_bincode", |b| {
        b.iter(|| {
            let _: Vec<wminspect::dsl::filter::FilterItem> = 
                bincode::deserialize(black_box(&bincode_data)).unwrap();
        })
    });
}

fn benchmark_optimized_wildcard(c: &mut Criterion) {
    use wminspect::core::wildcard::OptimizedWildcardMatcher;
    
    let patterns = vec![
        ("test*", "test123"),
        ("*test*", "this is a test string"),
        ("t?st", "test"),
        ("*", "any string"),
        ("complex*pattern*", "complex_long_pattern_match"),
    ];

    c.bench_function("optimized_wildcard_match", |b| {
        b.iter(|| {
            for (pattern, text) in &patterns {
                OptimizedWildcardMatcher::match_pattern(black_box(pattern), black_box(text));
            }
        })
    });
    
    // Test batch matching
    let batch_patterns = vec!["test*", "*example*", "exact", "complex*pattern*"];
    c.bench_function("optimized_batch_match", |b| {
        b.iter(|| {
            OptimizedWildcardMatcher::batch_match(black_box(&batch_patterns), black_box("test_example_text"));
        })
    });
}

fn benchmark_stack_diff(c: &mut Criterion) {
    use wminspect::core::stack_diff::CachedStackDiff;
    
    let mut diff_calc = CachedStackDiff::new();
    let initial_stack = vec![1, 2, 3, 4, 5];
    let _ = diff_calc.compute_diff(&initial_stack);
    
    c.bench_function("stack_diff_no_change", |b| {
        b.iter(|| {
            diff_calc.compute_diff(black_box(&initial_stack))
        })
    });
    
    let modified_stack = vec![1, 3, 5, 6, 7];
    c.bench_function("stack_diff_with_changes", |b| {
        b.iter(|| {
            diff_calc.compute_diff(black_box(&modified_stack))
        })
    });
}

fn benchmark_colorized_output(c: &mut Criterion) {
    use wminspect::core::colorized_output::{ColorizedFormatter, OutputMode};
    
    let formatter = ColorizedFormatter::with_mode(OutputMode::Colorized);
    let no_color_formatter = ColorizedFormatter::with_mode(OutputMode::NoColor);
    
    c.bench_function("colorized_format_window_entry", |b| {
        b.iter(|| {
            formatter.format_window_entry(
                black_box(0),
                black_box(0x12345),
                black_box("test-window"),
                black_box("100x200+50+75"),
                black_box("OR Viewable"),
                black_box(false)
            )
        })
    });
    
    c.bench_function("no_color_format_window_entry", |b| {
        b.iter(|| {
            no_color_formatter.format_window_entry(
                black_box(0),
                black_box(0x12345),
                black_box("test-window"),
                black_box("100x200+50+75"),
                black_box("OR Viewable"),
                black_box(false)
            )
        })
    });
}

criterion_group!(
    benches,
    benchmark_tokenizer,
    benchmark_parser,
    benchmark_wildcard_matching,
    benchmark_filter_creation,
    benchmark_serialization,
    benchmark_optimized_wildcard,
    benchmark_stack_diff,
    benchmark_colorized_output
);
criterion_main!(benches);
