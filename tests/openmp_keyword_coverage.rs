//! Representative standardized directive coverage through the strict facade.
//!
//! This intentionally uses complete legal specimens. Enumerating private raw
//! parser keywords with fabricated arguments used to bless malformed syntax.

use roup::api::{OpenMpConfig, OpenMpParser};
use roup::ast::OmpDirectiveKind;
use roup::version::{CStandard, HostLanguageProfile, OpenMpVersion, SourceForm};

fn parser() -> OpenMpParser {
    OpenMpConfig::exact(
        OpenMpVersion::V6_0,
        HostLanguageProfile::C(CStandard::C23),
        SourceForm::Pragma,
    )
    .unwrap()
    .parser()
}

#[test]
fn standardized_directive_families_have_legal_public_specimens() {
    let specimens = [
        (
            "#pragma omp parallel num_threads(4)",
            OmpDirectiveKind::Parallel,
        ),
        ("#pragma omp for private(i)", OmpDirectiveKind::For),
        ("#pragma omp simd safelen(8)", OmpDirectiveKind::Simd),
        ("#pragma omp teams num_teams(2)", OmpDirectiveKind::Teams),
        (
            "#pragma omp distribute private(i)",
            OmpDirectiveKind::Distribute,
        ),
        ("#pragma omp task depend(in:a)", OmpDirectiveKind::Task),
        (
            "#pragma omp taskloop grainsize(4)",
            OmpDirectiveKind::Taskloop,
        ),
        ("#pragma omp target map(tofrom:a)", OmpDirectiveKind::Target),
        (
            "#pragma omp target data map(tofrom:a)",
            OmpDirectiveKind::TargetData,
        ),
        (
            "#pragma omp target enter data map(to:a)",
            OmpDirectiveKind::TargetEnterData,
        ),
        (
            "#pragma omp target exit data map(from:a)",
            OmpDirectiveKind::TargetExitData,
        ),
        (
            "#pragma omp target update to(a)",
            OmpDirectiveKind::TargetUpdate,
        ),
        ("#pragma omp barrier", OmpDirectiveKind::Barrier),
        ("#pragma omp taskwait", OmpDirectiveKind::Taskwait),
        ("#pragma omp taskgroup", OmpDirectiveKind::Taskgroup),
        ("#pragma omp ordered", OmpDirectiveKind::Ordered),
        ("#pragma omp atomic read", OmpDirectiveKind::Atomic),
        ("#pragma omp flush(a)", OmpDirectiveKind::Flush),
        ("#pragma omp critical(lock)", OmpDirectiveKind::Critical),
        ("#pragma omp master", OmpDirectiveKind::Master),
        ("#pragma omp masked filter(0)", OmpDirectiveKind::Masked),
        ("#pragma omp scope private(a)", OmpDirectiveKind::Scope),
        ("#pragma omp scan inclusive(a)", OmpDirectiveKind::Scan),
        (
            "#pragma omp metadirective when(device={kind(cpu)}: parallel)",
            OmpDirectiveKind::Metadirective,
        ),
        (
            "#pragma omp error at(compilation) severity(warning)",
            OmpDirectiveKind::Error,
        ),
        ("#pragma omp cancel parallel", OmpDirectiveKind::Cancel),
        (
            "#pragma omp cancellation point parallel",
            OmpDirectiveKind::CancellationPoint,
        ),
        (
            "#pragma omp threadprivate(a)",
            OmpDirectiveKind::Threadprivate,
        ),
        ("#pragma omp tile sizes(4)", OmpDirectiveKind::Tile),
        ("#pragma omp unroll full", OmpDirectiveKind::Unroll),
    ];

    let mut failures = Vec::new();
    for (source, expected_kind) in specimens {
        match parser().parse(source) {
            Ok(parsed) if parsed.directive().kind() == expected_kind => {}
            Ok(parsed) => failures.push(format!(
                "{source:?}: expected {expected_kind:?}, got {:?}",
                parsed.directive().kind()
            )),
            Err(error) => failures.push(format!("{source:?}: {error}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn fabricated_keywords_and_incomplete_directives_are_rejected() {
    for source in [
        "#pragma omp declare reduction",
        "#pragma omp declare mapper",
        "#pragma omp cancellation point",
        "#pragma omp target enter data",
        "#pragma omp made_up_directive",
        "#pragma omp parallel made_up_clause(value)",
    ] {
        assert!(parser().parse(source).is_err(), "{source} must be rejected");
    }
}
