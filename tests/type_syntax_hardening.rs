use roup::api::{OpenMpConfig, OpenMpParser};
use roup::version::{CppStandard, HostLanguageProfile, OpenMpVersion, SourceForm};

fn cpp() -> OpenMpParser {
    OpenMpConfig::exact(
        OpenMpVersion::V6_0,
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
        SourceForm::Pragma,
    )
    .expect("valid C++ OpenMP configuration")
    .parser()
}

#[test]
fn structured_types_reach_all_openmp_declaration_paths() {
    for source in [
        "#pragma omp declare_reduction(ns::sum<N + 1> : std::array<std::pair<int, long>, N + 1>) combiner(omp_out += omp_in)",
        "#pragma omp declare_reduction(sum : std::pair /* type */ < int, long >, double) combiner(omp_out += omp_in)",
        "#pragma omp declare_mapper(std::array<std::pair<int, long>, N + 1> value) map(value)",
        "#pragma omp declare_variant(ns::fast<N + 1>) match(construct={parallel})",
        "#pragma omp declare_induction(ns::step<N + 1> : int (*)(double)) inductor(omp_var += omp_step) collector(omp_step * omp_idx)",
    ] {
        cpp().parse(source).unwrap_or_else(|error| {
            panic!("valid structured declaration {source:?} was rejected: {error}")
        });
    }
}

#[test]
fn malformed_type_bags_fail_through_every_public_declaration_path() {
    for source in [
        "#pragma omp declare_reduction(sum : int + garbage) combiner(omp_out += omp_in)",
        "#pragma omp declare_reduction(ns::sum<int +> : int) combiner(omp_out += omp_in)",
        "#pragma omp declare_mapper(int + garbage value) map(value)",
        "#pragma omp declare_variant(ns::fast<int +>) match(construct={parallel})",
        "#pragma omp declare_induction(ns::step<int +> : int) inductor(omp_var += omp_step) collector(omp_step * omp_idx)",
        "#pragma omp declare_induction(step : int && &&) inductor(omp_var += omp_step) collector(omp_step * omp_idx)",
    ] {
        assert!(
            cpp().parse(source).is_err(),
            "malformed type or template-id {source:?} was accepted"
        );
    }
}
