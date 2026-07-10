use roup::api::OpenMpConfig;
use roup::ast::{OmpClauseKind, OmpDirectiveKind};
use roup::ir::{ClauseData, OmpCount};
use roup::version::{CStandard, HostLanguageProfile, OpenMpVersion, SourceForm};

fn parser(version: OpenMpVersion) -> roup::api::OpenMpParser {
    OpenMpConfig::exact(
        version,
        HostLanguageProfile::C(CStandard::C23),
        SourceForm::Pragma,
    )
    .unwrap()
    .parser()
}

#[test]
fn standardized_loop_transformations_parse_through_the_public_api() {
    let tile = parser(OpenMpVersion::V6_0)
        .parse("#pragma omp tile sizes(4, 8)")
        .expect("tile is standardized");
    assert_eq!(tile.directive().kind(), OmpDirectiveKind::Tile);
    assert_eq!(tile.directive().clauses()[0].kind(), OmpClauseKind::Sizes);
    let ClauseData::Sizes { sizes } = tile.directive().clauses()[0].payload() else {
        panic!("tile sizes must be typed expressions");
    };
    assert_eq!(
        sizes.iter().map(|value| value.source()).collect::<Vec<_>>(),
        ["4", "8"]
    );

    for (source, expected) in [
        ("#pragma omp unroll full", OmpDirectiveKind::Unroll),
        ("#pragma omp fuse", OmpDirectiveKind::Fuse),
        ("#pragma omp reverse", OmpDirectiveKind::Reverse),
        ("#pragma omp stripe sizes(16)", OmpDirectiveKind::Stripe),
    ] {
        let parsed = parser(OpenMpVersion::V6_0)
            .parse(source)
            .unwrap_or_else(|error| panic!("{source} should parse: {error}"));
        assert_eq!(parsed.directive().kind(), expected);
    }

    let permutation = parser(OpenMpVersion::V6_0)
        .parse("#pragma omp interchange permutation(2, 1)")
        .expect("permutation should parse");
    let ClauseData::Permutation { positions } = permutation.directive().clauses()[0].payload()
    else {
        panic!("permutation must have a dedicated payload");
    };
    assert_eq!(
        positions
            .iter()
            .map(|value| value.source())
            .collect::<Vec<_>>(),
        ["2", "1"]
    );

    let counts = parser(OpenMpVersion::V6_0)
        .parse("#pragma omp split counts(10, omp_fill, 20)")
        .expect("counts should parse");
    let ClauseData::Counts { counts } = counts.directive().clauses()[0].payload() else {
        panic!("counts must have a dedicated payload");
    };
    assert!(matches!(counts[1], OmpCount::Fill));
    assert!(matches!(&counts[0], OmpCount::Expression(value) if value.source() == "10"));
    assert!(matches!(&counts[2], OmpCount::Expression(value) if value.source() == "20"));
}

#[test]
fn transformation_version_floors_and_payload_errors_are_strict() {
    assert!(parser(OpenMpVersion::V5_0)
        .parse("#pragma omp tile sizes(4)")
        .is_err());
    assert!(parser(OpenMpVersion::V5_2)
        .parse("#pragma omp fuse")
        .is_err());

    for source in [
        "#pragma omp tile sizes()",
        "#pragma omp tile sizes(4, @)",
        "#pragma omp split counts()",
        "#pragma omp split counts(1, 2)",
        "#pragma omp split counts(omp_fill, omp_fill)",
        "#pragma omp interchange permutation()",
        "#pragma omp interchange permutation(1)",
        "#pragma omp interchange permutation(1, 1)",
    ] {
        assert!(
            parser(OpenMpVersion::V6_0).parse(source).is_err(),
            "{source} must be rejected"
        );
    }
}
