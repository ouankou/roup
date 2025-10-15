use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use roup::lexer::collapse_line_continuations;

fn bench_no_continuation(c: &mut Criterion) {
    let mut group = c.benchmark_group("no_continuation");

    let cases = vec![
        ("short", "parallel"),
        ("medium", "parallel for private(i,j,k)"),
        ("long", "target teams distribute parallel for simd reduction(+:sum) private(i,j,k) firstprivate(n)"),
        ("very_long", "target teams distribute parallel for simd collapse(3) reduction(+:sum,min:min_val,max:max_val) private(i,j,k,temp) firstprivate(n,m) shared(data,result) default(none) if(n>100) schedule(static,8) num_threads(4)"),
    ];

    for (name, input) in cases {
        group.bench_with_input(BenchmarkId::from_parameter(name), &input, |b, &input| {
            b.iter(|| {
                let result = collapse_line_continuations(black_box(input));
                black_box(result);
            });
        });
    }

    group.finish();
}

fn bench_c_continuation(c: &mut Criterion) {
    let mut group = c.benchmark_group("c_continuation");

    let cases = vec![
        ("single", "parallel \\\n    num_threads(4)"),
        ("double", "parallel for \\\n    private(i,j) \\\n    reduction(+:sum)"),
        ("complex", "target teams \\\n    distribute \\\n    parallel for \\\n    simd \\\n    reduction(+:sum)"),
    ];

    for (name, input) in cases {
        group.bench_with_input(BenchmarkId::from_parameter(name), &input, |b, &input| {
            b.iter(|| {
                let result = collapse_line_continuations(black_box(input));
                black_box(result);
            });
        });
    }

    group.finish();
}

fn bench_fortran_continuation(c: &mut Criterion) {
    let mut group = c.benchmark_group("fortran_continuation");

    let cases = vec![
        ("simple", "parallel do &\n!$omp private(i,j)"),
        ("multiple", "target teams &\n!$omp& distribute &\n!$omp& parallel do"),
        ("complex", "target teams distribute &\n!$omp& parallel do &\n!$omp& reduction(+:sum) &\n!$omp& private(i,j,k)"),
    ];

    for (name, input) in cases {
        group.bench_with_input(BenchmarkId::from_parameter(name), &input, |b, &input| {
            b.iter(|| {
                let result = collapse_line_continuations(black_box(input));
                black_box(result);
            });
        });
    }

    group.finish();
}

fn bench_repeated_calls(c: &mut Criterion) {
    let mut group = c.benchmark_group("repeated_calls");

    // Simulate parser behavior: 3 calls on same input (directive.rs pattern)
    let input = "parallel for private(i,j,k) reduction(+:sum)";

    group.bench_function("three_calls_same_input", |b| {
        b.iter(|| {
            // Simulates directive.rs lines 157, 185, 213
            let r1 = collapse_line_continuations(black_box(input));
            black_box(&r1);
            let r2 = collapse_line_continuations(black_box(input));
            black_box(&r2);
            let r3 = collapse_line_continuations(black_box(input));
            black_box(&r3);
        });
    });

    let input_with_cont = "parallel \\\n    for private(i,j,k)";

    group.bench_function("three_calls_with_continuation", |b| {
        b.iter(|| {
            let r1 = collapse_line_continuations(black_box(input_with_cont));
            black_box(&r1);
            let r2 = collapse_line_continuations(black_box(input_with_cont));
            black_box(&r2);
            let r3 = collapse_line_continuations(black_box(input_with_cont));
            black_box(&r3);
        });
    });

    group.finish();
}

fn bench_mixed_workload(c: &mut Criterion) {
    // Realistic workload: 90% no continuation, 10% with continuation
    let inputs: Vec<&str> = vec![
        "parallel",
        "parallel for",
        "parallel for private(i)",
        "target teams",
        "teams distribute",
        "parallel for reduction(+:sum)",
        "simd",
        "for",
        "sections",
        "parallel \\\n    num_threads(4)", // 10% have continuation
    ];

    c.bench_function("mixed_workload_realistic", |b| {
        b.iter(|| {
            for input in &inputs {
                let result = collapse_line_continuations(black_box(*input));
                black_box(result);
            }
        });
    });
}

criterion_group!(
    benches,
    bench_no_continuation,
    bench_c_continuation,
    bench_fortran_continuation,
    bench_repeated_calls,
    bench_mixed_workload
);
criterion_main!(benches);
