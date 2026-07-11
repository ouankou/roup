use roup::api::{OpenAccConfig, OpenMpConfig, ParsedOpenAccDirective, ParsedOpenMpDirective};
use roup::ast::{AccClausePayload, OmpDirectiveParameter};
use roup::diagnostic::Diagnostic;
use roup::ir::{ClauseData, ClauseItem};
use roup::version::{
    CStandard, FortranStandard, HostLanguageProfile, OpenAccVersion, OpenMpVersion, SourceForm,
};

fn parse_omp_fortran(source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::exact(
        OpenMpVersion::V5_2,
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .expect("valid Fortran OpenMP profile")
    .parser()
    .parse(source)
}

fn parse_acc_fortran(
    version: OpenAccVersion,
    source: &str,
) -> Result<ParsedOpenAccDirective, Diagnostic> {
    OpenAccConfig::exact(
        version,
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .expect("valid Fortran OpenACC profile")
    .parser()
    .parse(source)
}

#[test]
fn openmp_private_and_copyin_retain_typed_named_common_blocks() {
    let parsed = parse_omp_fortran("!$omp parallel private(/WORK/) copyin(/STATE/)")
        .expect("named common blocks are OpenMP variable-list items");
    assert!(matches!(
        parsed.directive().clauses()[0].payload(),
        ClauseData::Private { items }
            if matches!(items.as_slice(), [ClauseItem::FortranCommonBlock(name)] if name.as_str() == "work")
    ));
    assert!(matches!(
        parsed.directive().clauses()[1].payload(),
        ClauseData::Copyin { items }
            if matches!(items.as_slice(), [ClauseItem::FortranCommonBlock(name)] if name.as_str() == "state")
    ));

    let copyprivate = parse_omp_fortran("!$omp single copyprivate(/THREAD_STATE/)")
        .expect("copyprivate retains a syntactically valid named common block");
    assert!(matches!(
        copyprivate.directive().clauses()[0].payload(),
        ClauseData::Copyprivate { items }
            if matches!(items.as_slice(), [ClauseItem::FortranCommonBlock(name)] if name.as_str() == "thread_state")
    ));
}

#[test]
fn openmp_common_block_spelling_and_linear_restriction_are_hard_errors() {
    for source in [
        "!$omp parallel private(//)",
        "!$omp parallel private(/ /)",
        "!$omp parallel private(/BLOCK)",
        "!$omp parallel private(BLOCK/)",
        "!$omp parallel private(/FIRST/SECOND/)",
        "!$omp simd linear(/BLOCK/)",
    ] {
        assert!(
            parse_omp_fortran(source).is_err(),
            "unexpectedly accepted {source:?}"
        );
    }

    let c_parser = OpenMpConfig::exact(
        OpenMpVersion::V5_2,
        HostLanguageProfile::C(CStandard::C23),
        SourceForm::Pragma,
    )
    .expect("valid C OpenMP profile")
    .parser();
    assert!(
        c_parser
            .parse("#pragma omp parallel private(/block/)")
            .is_err()
    );
}

#[test]
fn openacc_common_blocks_have_the_2_0_floor_and_remain_typed() {
    let source = "!$acc declare copyin(/STATE/)";
    assert!(parse_acc_fortran(OpenAccVersion::V1_0, source).is_err());
    let parsed = parse_acc_fortran(OpenAccVersion::V2_0, source)
        .expect("OpenACC 2.0 added common blocks to declare data clauses");
    let AccClausePayload::Copy(copy) = parsed.directive().clauses()[0].payload() else {
        panic!("expected typed OpenACC copyin payload");
    };
    assert!(matches!(
        copy.variables(),
        [ClauseItem::FortranCommonBlock(name)] if name.as_str() == "state"
    ));

    let private = parse_acc_fortran(OpenAccVersion::V2_0, "!$acc parallel private(/WORK/)")
        .expect("OpenACC private uses the general var-list grammar");
    assert!(matches!(
        private.directive().clauses()[0].payload(),
        AccClausePayload::ItemList(items)
            if matches!(items.as_slice(), [ClauseItem::FortranCommonBlock(name)] if name.as_str() == "work")
    ));
}

#[test]
fn openacc_clause_specific_common_block_exclusions_are_hard_errors() {
    for source in [
        "!$acc parallel deviceptr(/BLOCK/)",
        "!$acc data present(/BLOCK/)",
        "!$acc parallel reduction(+: /BLOCK/)",
        "!$acc cache(/BLOCK/)",
    ] {
        assert!(
            parse_acc_fortran(OpenAccVersion::V3_4, source).is_err(),
            "unexpectedly accepted {source:?}"
        );
    }
}

#[test]
fn flush_parameter_state_is_unrelated_to_general_clause_common_blocks() {
    let parsed = parse_omp_fortran("!$omp flush(/BLOCK/)")
        .expect("flush retains its directive-specific common-block item");
    assert!(matches!(
        parsed.directive().parameter(),
        Some(OmpDirectiveParameter::FlushList(items)) if items.len() == 1
    ));
}
