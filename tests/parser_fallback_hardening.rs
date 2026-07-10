use roup::api::{OpenAccConfig, OpenMpConfig};
use roup::version::{CStandard, CppStandard, FortranStandard, HostLanguageProfile, SourceForm};

fn omp_c() -> roup::api::OpenMpParser {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .unwrap()
        .parser()
}

fn omp_cpp() -> roup::api::OpenMpParser {
    OpenMpConfig::new(
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
        SourceForm::Pragma,
    )
    .unwrap()
    .parser()
}

fn acc_c() -> roup::api::OpenAccParser {
    OpenAccConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .unwrap()
        .parser()
}

fn acc_fortran() -> roup::api::OpenAccParser {
    OpenAccConfig::new(
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .unwrap()
    .parser()
}

#[test]
fn malformed_openmp_never_falls_through_to_a_looser_grammar() {
    for source in [
        "#pragma omp parallel reduction(original(a], +: x)",
        "#pragma omp parallel reduction(original(a) trailing, +: x)",
        "#pragma omp parallel reduction(original(a,,b), +: x)",
        "#pragma omp parallel reduction(original(\"unterminated), +: x)",
        "#pragma omp parallel if(\"unterminated)",
        "#pragma omp parallel if(/* unterminated)",
        "#pragma omp parallel private(a])",
        "#pragma omp parallel private((a])",
        "#pragma omp parallel if(paralell: enabled)",
        "#pragma omp parallel if(parallel(: enabled)",
        "#pragma omp target map(targte: x)",
        "#pragma omp target map(mapper(foo]: x)",
        "#pragma omp teams num_teams(lower: upper: trailing)",
        "#pragma omp teams num_teams(lower ? first : second:)",
        "#pragma omp teams num_teams(: upper)",
        "#pragma omp ordered doacross(sink: outer - (distance])",
        "#pragma omp target uses_allocators(custom_allocator(traits])",
    ] {
        assert!(
            omp_c().parse(source).is_err(),
            "malformed syntax fell through to an accepting grammar: {source:?}"
        );
    }
}

#[test]
fn malformed_cpp_name_alternatives_do_not_mask_the_first_parse_error() {
    for source in [
        "#pragma omp declare variant(fast<int,>) match(construct={parallel})",
        "#pragma omp declare variant(fast<int) match(construct={parallel})",
        "#pragma omp declare variant(fast + slow) match(construct={parallel})",
        "#pragma omp declare variant(operator<) match(construct={parallel})",
        "#pragma omp declare variant(ns::operator<junk) match(construct={parallel})",
        "#pragma omp parallel for induction(step(1), next<int,: index)",
    ] {
        assert!(
            omp_cpp().parse(source).is_err(),
            "malformed C++ name was accepted by a secondary grammar: {source:?}"
        );
    }
}

#[test]
fn malformed_openacc_optional_forms_do_not_degrade_to_bare_forms() {
    for source in [
        "#pragma acc wait(queue]",
        "#pragma acc routine(worker]",
        "#pragma acc parallel async(queue]",
        "#pragma acc parallel copy(a,,b)",
        "#pragma acc parallel reduction(+: a]",
        "#pragma acc parallel if(\"unterminated)",
        "#pragma acc parallel if(/* unterminated)",
    ] {
        assert!(
            acc_c().parse(source).is_err(),
            "malformed OpenACC optional form degraded to another grammar: {source:?}"
        );
    }
    for source in [
        "!$acc end parallel(extra)",
        "!$acc end parallel extra",
        "!$acc end parallel /* unterminated",
    ] {
        assert!(
            acc_fortran().parse(source).is_err(),
            "malformed OpenACC end form degraded to another grammar: {source:?}"
        );
    }
}

#[test]
fn genuine_grammar_alternatives_remain_accepted() {
    omp_cpp()
        .parse("#pragma omp declare variant(ns::fast<int>) match(construct={parallel})")
        .expect("a C++ template-id is a real function-name alternative");
    omp_c()
        .parse("#pragma omp teams num_teams(flag ? lower : upper)")
        .expect("a conditional expression is not a num_teams bound separator");
    omp_c()
        .parse("#pragma omp target map(always to: value)")
        .expect("the standardized historical map modifier spelling remains accepted");
    acc_c()
        .parse("#pragma acc wait")
        .expect("bare wait is standardized");
    acc_c()
        .parse("#pragma acc routine")
        .expect("bare routine is standardized historically");
    acc_fortran()
        .parse("!$acc end parallel")
        .expect("an OpenACC end kind is a real optional directive parameter");
}
