use roup::parser::openmp;

#[test]
fn parses_loop_transformation_directives() {
    let parser = openmp::parser();
    let samples = [
        ("#pragma omp fuse", "fuse", None),
        ("#pragma omp split", "split", None),
        ("#pragma omp tile sizes(4)", "tile", Some("sizes")),
        ("#pragma omp interchange", "interchange", None),
        ("#pragma omp reverse", "reverse", None),
        ("#pragma omp stripe", "stripe", None),
        ("#pragma omp unroll", "unroll", None),
    ];

    for (source, expected_name, expected_clause) in samples {
        let (rest, parsed) = parser
            .parse(source)
            .unwrap_or_else(|_| panic!("failed to parse `{source}`"));
        assert!(rest.is_empty(), "`{source}` should consume all input");
        assert_eq!(parsed.name, expected_name);

        match expected_clause {
            Some(clause) => {
                assert_eq!(
                    parsed.clauses.len(),
                    1,
                    "`{source}` should carry one clause"
                );
                assert_eq!(parsed.clauses[0].name, clause);
            }
            None => assert!(
                parsed.clauses.is_empty(),
                "`{source}` should not parse clauses"
            ),
        }
    }
}
