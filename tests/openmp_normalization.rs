use roup::ast::ClauseNormalizationMode;
use roup::ir::{ClauseData, ParserConfig};
use roup::parser::openmp;

fn parse_with_mode(source: &str, mode: ClauseNormalizationMode) -> Vec<ClauseData> {
    let parser = openmp::parser();
    let ast = parser
        .parse_ast(source, mode, &ParserConfig::default())
        .expect("parse should succeed");
    let omp = match ast.body {
        roup::ast::DirectiveBody::OpenMp(d) => d,
        _ => panic!("expected OpenMP directive"),
    };
    omp.clauses.into_iter().map(|c| c.payload).collect()
}

#[test]
fn normalization_disabled_keeps_duplicate_shared() {
    let clauses = parse_with_mode(
        "#pragma omp parallel shared(a) shared(b)",
        ClauseNormalizationMode::Disabled,
    );
    assert_eq!(clauses.len(), 2);
    assert!(matches!(clauses[0], ClauseData::Shared { .. }));
    assert!(matches!(clauses[1], ClauseData::Shared { .. }));
}

#[test]
fn normalization_merge_concatenates_shared_lists() {
    let clauses = parse_with_mode(
        "#pragma omp parallel shared(a) shared(b)",
        ClauseNormalizationMode::MergeVariableLists,
    );
    assert_eq!(clauses.len(), 1);
    match &clauses[0] {
        ClauseData::Shared { items } => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].to_string(), "a");
            assert_eq!(items[1].to_string(), "b");
        }
        other => panic!("expected merged shared clause, got {other:?}"),
    }
}

#[test]
fn normalization_merges_reductions_with_different_colon_spacing() {
    let clauses = parse_with_mode(
        "#pragma omp parallel reduction(+:a) reduction(+: b)",
        ClauseNormalizationMode::MergeVariableLists,
    );
    assert_eq!(clauses.len(), 1);
    match &clauses[0] {
        ClauseData::Reduction {
            operator, items, ..
        } => {
            assert_eq!(operator.to_string(), "+");
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].to_string(), "a");
            assert_eq!(items[1].to_string(), "b");
        }
        other => panic!("expected merged reduction clause, got {other:?}"),
    }
}
