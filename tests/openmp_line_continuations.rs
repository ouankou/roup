//! Integration tests for OpenMP line continuation handling across languages.
//!
//! These tests exercise the lexer normalization that stitches multi-line
//! directives into the logical single-line representation consumed by the
//! parser. Both unit-level coverage (lexer tests) and end-to-end parser
//! coverage are provided to ensure the continuation rules stay in sync.

use roup::lexer::Language;
use roup::parser::openmp;

#[test]
fn parses_c_pragma_with_backslash_continuation() {
    let parser = openmp::parser().with_language(Language::C);
    let input = [
        "#pragma omp parallel \\",
        "// preserve structure just like in source files",
        "    default(shared) \\",
        "    private(i, j)",
    ]
    .join("\n");

    let (rest, directive) = parser.parse(&input).expect("parsing should succeed");

    assert_eq!(rest.trim(), "");
    assert_eq!(directive.name, "parallel");
    assert_eq!(directive.clauses.len(), 2);
    assert_eq!(directive.clauses[0].name, "default");
    assert_eq!(directive.clauses[1].name, "private");
}

#[test]
fn parses_fortran_free_with_repeated_sentinel() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$OMP PARALLEL DO &\n!$OMP& PRIVATE(I, J) &\n!$OMP  SCHEDULE(STATIC)";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest.trim(), "");
    assert_eq!(directive.name.to_lowercase(), "parallel do");
    assert_eq!(directive.clauses.len(), 2);
    assert_eq!(directive.clauses[0].name.to_lowercase(), "private");
    assert_eq!(directive.clauses[1].name.to_lowercase(), "schedule");
}

#[test]
fn parses_fortran_free_with_leading_ampersand() {
    let parser = openmp::parser().with_language(Language::FortranFree);
    let input = "!$omp parallel &\n  private(i) &\n& num_threads(4)";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest.trim(), "");
    assert_eq!(directive.name.to_lowercase(), "parallel");
    assert_eq!(directive.clauses.len(), 2);
    assert_eq!(directive.clauses[0].name.to_lowercase(), "private");
    assert_eq!(directive.clauses[1].name.to_lowercase(), "num_threads");
}

#[test]
fn parses_fortran_fixed_with_column_six_ampersand() {
    let parser = openmp::parser().with_language(Language::FortranFixed);
    let input = "C$OMP PARALLEL DO&\nC$OMP& PRIVATE(I)";

    let (rest, directive) = parser.parse(input).expect("parsing should succeed");

    assert_eq!(rest.trim(), "");
    assert_eq!(directive.name.to_lowercase(), "parallel do");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name.to_lowercase(), "private");
}
