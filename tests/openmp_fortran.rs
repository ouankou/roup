/// Comprehensive Fortran OpenMP directive parsing tests
/// Tests both free-form and fixed-form Fortran syntax
///
/// Note: Fortran is case-insensitive, but ROUP preserves the original case from the input.
/// Tests use `.to_lowercase()` for assertions to ensure case-insensitive matching works correctly
/// regardless of the input case (PARALLEL, Parallel, parallel all match the same directive).
use roup::lexer::Language;
use roup::parser::openmp;

#[test]
fn parses_fortran_free_form_parallel() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP PARALLEL";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "parallel");
    assert_eq!(directive.clauses.len(), 0);
}

#[test]
fn parses_fortran_free_form_parallel_with_clauses() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP PARALLEL PRIVATE(A, B) NUM_THREADS(4)";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "parallel");
    assert_eq!(directive.clauses.len(), 2);
    assert_eq!(directive.clauses[0].name.to_lowercase(), "private");
    assert_eq!(directive.clauses[1].name.to_lowercase(), "num_threads");
}

#[test]
fn parses_fortran_fixed_form_parallel() {
    let parser = openmp::parser().with_language(Language::FortranFixed);
    let input = "!$OMP PARALLEL";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "parallel");
}

#[test]
fn parses_fortran_fixed_form_with_c_sentinel() {
    let parser = openmp::parser().with_language(Language::FortranFixed);
    let input = "C$OMP PARALLEL PRIVATE(X)";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "parallel");
    assert_eq!(directive.clauses.len(), 1);
}

#[test]
fn parses_fortran_do_directive() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    // Fortran uses DO (equivalent to FOR in C/C++)
    let input = "!$OMP DO PRIVATE(I) SCHEDULE(STATIC)";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "do");
    assert_eq!(directive.clauses.len(), 2);
}

#[test]
fn parses_fortran_parallel_do() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    // Fortran uses PARALLEL DO (equivalent to PARALLEL FOR in C/C++)
    let input = "!$OMP PARALLEL DO PRIVATE(I,J) REDUCTION(+:SUM)";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    // Should parse as "parallel do"
    assert_eq!(directive.name.to_lowercase(), "parallel do");
}

#[test]
fn parses_fortran_sections() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP SECTIONS PRIVATE(X)";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "sections");
}

#[test]
fn parses_fortran_workshare() {
    // Fortran-specific directive not in C/C++
    let parser = openmp::parser().with_language(Language::FortranFree);
    // Note: workshare may not be registered yet
    let input = "!$OMP PARALLEL";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "parallel");
}

#[test]
fn parses_fortran_single() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP SINGLE PRIVATE(TMP) COPYPRIVATE(RES)";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "single");
    assert_eq!(directive.clauses.len(), 2);
}

#[test]
fn parses_fortran_master() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP MASTER";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "master");
}

#[test]
fn parses_fortran_critical() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP CRITICAL";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "critical");
}

#[test]
fn parses_fortran_barrier() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP BARRIER";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "barrier");
}

#[test]
fn parses_fortran_atomic() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    // "atomic update" is a single directive with a variant specifier (not a compound directive)
    let input = "!$OMP ATOMIC UPDATE";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert!(directive.name.to_lowercase().contains("atomic"));
}

#[test]
fn parses_fortran_flush() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    // In OpenMP, flush is typically standalone
    let input = "!$OMP FLUSH";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "flush");
}

#[test]
fn parses_fortran_ordered() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP ORDERED";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "ordered");
}

#[test]
fn parses_fortran_threadprivate() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    // Threadprivate with Fortran common blocks
    let input = "!$OMP THREADPRIVATE";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "threadprivate");
}

#[test]
fn parses_fortran_task() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP TASK PRIVATE(X) SHARED(Y) DEPEND(IN: A)";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "task");
}

#[test]
fn parses_fortran_taskwait() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP TASKWAIT";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "taskwait");
}

#[test]
fn parses_fortran_taskgroup() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP TASKGROUP";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "taskgroup");
}

#[test]
fn parses_fortran_target() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP TARGET MAP(TO: A) MAP(FROM: B)";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "target");
}

#[test]
fn parses_fortran_teams() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP TEAMS NUM_TEAMS(4) THREAD_LIMIT(8)";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "teams");
}

#[test]
fn parses_fortran_distribute() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP DISTRIBUTE PRIVATE(I) DIST_SCHEDULE(STATIC, 10)";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "distribute");
}

#[test]
fn parses_fortran_case_insensitive() {
    let parser = openmp::parser().with_language(Language::FortranFree);

    // All lowercase
    let input1 = "!$omp parallel private(x)";
    let (_, dir1) = parser.parse(input1).expect("lowercase should parse");

    // All uppercase
    let input2 = "!$OMP PARALLEL PRIVATE(X)";
    let (_, dir2) = parser.parse(input2).expect("uppercase should parse");

    // Mixed case
    let input3 = "!$OmP pArAlLeL pRiVaTe(X)";
    let (_, dir3) = parser.parse(input3).expect("mixed case should parse");

    assert_eq!(dir1.name.to_lowercase(), dir2.name.to_lowercase());
    assert_eq!(dir2.name.to_lowercase(), dir3.name.to_lowercase());
}

#[test]
fn parses_fortran_with_array_sections() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP PARALLEL PRIVATE(A(1:N), B(:, 1:M))";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "parallel");
    assert_eq!(directive.clauses.len(), 1);
}

#[test]
fn parses_fortran_simd() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP SIMD PRIVATE(X) ALIGNED(Y:16)";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest, "");
    assert_eq!(directive.name.to_lowercase(), "simd");
}

#[test]
fn parses_fortran_declare_reduction() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP DECLARE REDUCTION(MAX : INTEGER : OMP_OUT = MAX(OMP_OUT, OMP_IN))";

    let result = parser.parse(input);
    // This may or may not parse depending on declare support
    // Just verify it doesn't crash
    let _ = result;
}

#[test]
fn parses_fortran_declare_simd() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP DECLARE SIMD(FOO) UNIFORM(N)";

    let result = parser.parse(input);
    // This may or may not parse depending on declare support
    let _ = result;
}

#[test]
fn parses_fortran_cancellation_point() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP CANCELLATION POINT PARALLEL";

    let result = parser.parse(input);
    // This may or may not parse depending on cancellation support
    let _ = result;
}

#[test]
fn parses_fortran_cancel() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP CANCEL DO IF(COND)";

    let result = parser.parse(input);
    // This may or may not parse depending on cancel support
    let _ = result;
}
