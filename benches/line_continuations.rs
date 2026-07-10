use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use roup::api::OpenMpConfig;
use roup::version::{CStandard, FortranStandard, HostLanguageProfile, SourceForm};
use std::hint::black_box;

fn bench_c_directives(c: &mut Criterion) {
    let parser = OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid C configuration")
        .parser();
    let mut group = c.benchmark_group("openmp_c_parse");
    let cases = [
        ("short", "#pragma omp parallel"),
        (
            "medium",
            "#pragma omp parallel for private(i,j,k) schedule(static)",
        ),
        (
            "long",
            "#pragma omp target teams distribute parallel for simd reduction(+:sum) private(i,j,k) firstprivate(n)",
        ),
        (
            "continued",
            "#pragma omp parallel for \\\n             private(i,j) \\\n             reduction(+:sum)",
        ),
    ];

    for (name, source) in cases {
        group.bench_with_input(BenchmarkId::from_parameter(name), source, |b, source| {
            b.iter(|| {
                black_box(
                    parser
                        .parse(black_box(source))
                        .expect("benchmark directive must parse"),
                );
            });
        });
    }
    group.finish();
}

fn bench_fortran_directives(c: &mut Criterion) {
    let parser = OpenMpConfig::new(
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .expect("valid Fortran configuration")
    .parser();
    let mut group = c.benchmark_group("openmp_fortran_parse");
    let cases = [
        ("simple", "!$omp parallel do private(i,j)"),
        (
            "continued",
            "!$omp parallel do &\n!$omp& private(i,j) &\n!$omp& reduction(+:sum)",
        ),
        (
            "combined",
            "!$omp target teams distribute parallel do reduction(+:sum)",
        ),
    ];

    for (name, source) in cases {
        group.bench_with_input(BenchmarkId::from_parameter(name), source, |b, source| {
            b.iter(|| {
                black_box(
                    parser
                        .parse(black_box(source))
                        .expect("benchmark directive must parse"),
                );
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_c_directives, bench_fortran_directives);
criterion_main!(benches);
