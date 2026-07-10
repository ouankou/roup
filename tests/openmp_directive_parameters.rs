use roup::api::{OpenMpConfig, ParsedOpenMpDirective};
use roup::ast::{
    OmpDeclareTargetListItem, OmpDirectiveParameter, OmpFunctionName, OmpStorageListItem,
};
use roup::diagnostic::Diagnostic;
use roup::version::{
    CStandard, CppStandard, FortranStandard, HostLanguageProfile, OpenMpVersion, SourceForm,
};

fn parse_c(source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid C configuration")
        .parser()
        .parse(source)
}

fn parse_cpp(source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::new(
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
        SourceForm::Pragma,
    )
    .expect("valid C++ configuration")
    .parser()
    .parse(source)
}

fn parse_cpp_exact(
    version: OpenMpVersion,
    source: &str,
) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::exact(
        version,
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
        SourceForm::Pragma,
    )
    .expect("valid exact C++ configuration")
    .parser()
    .parse(source)
}

fn parse_fortran(source: &str) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::new(
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .expect("valid Fortran configuration")
    .parser()
    .parse(source)
}

fn parse_fortran_exact(
    version: OpenMpVersion,
    source: &str,
) -> Result<ParsedOpenMpDirective, Diagnostic> {
    OpenMpConfig::exact(
        version,
        HostLanguageProfile::Fortran(FortranStandard::Fortran2023),
        SourceForm::FortranFree,
    )
    .expect("valid exact Fortran configuration")
    .parser()
    .parse(source)
}

#[test]
fn depobj_requires_exactly_one_checked_lvalue() {
    for (source, expected) in [
        ("#pragma omp depobj(handle) destroy", "handle"),
        ("#pragma omp depobj(object.member) destroy", "object.member"),
        (
            "#pragma omp depobj(objects[index]) destroy",
            "objects[index]",
        ),
        ("#pragma omp depobj(*handle_ptr) destroy", "*handle_ptr"),
    ] {
        let parsed = parse_c(source).unwrap_or_else(|error| panic!("{source:?}: {error}"));
        let Some(OmpDirectiveParameter::Depobj(target)) = parsed.directive().parameter() else {
            panic!("depobj target must have a dedicated typed parameter");
        };
        assert_eq!(target.to_string(), expected);
    }

    for source in [
        "#pragma omp depobj(first, second) destroy",
        "#pragma omp depobj(42) destroy",
        "#pragma omp depobj(&handle) destroy",
        "#pragma omp depobj(left + right) destroy",
    ] {
        assert!(
            parse_c(source).is_err(),
            "non-lvalue or non-singleton depobj target was accepted: {source:?}"
        );
    }
}

#[test]
fn storage_directives_keep_distinct_whole_entity_lists() {
    let allocate = parse_cpp("#pragma omp allocate(::ns::value, Type::storage)")
        .expect("qualified whole variables must parse");
    let Some(OmpDirectiveParameter::AllocateList(items)) = allocate.directive().parameter() else {
        panic!("allocate list must have its directive-specific AST variant");
    };
    assert!(matches!(
        items.as_slice(),
        [OmpStorageListItem::Name(_), OmpStorageListItem::Name(_)]
    ));
    assert_eq!(
        items.iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["::ns::value", "Type::storage"]
    );

    let threadprivate = parse_cpp("#pragma omp threadprivate(::ns::value)")
        .expect("qualified threadprivate variable must parse");
    assert!(matches!(
        threadprivate.directive().parameter(),
        Some(OmpDirectiveParameter::ThreadprivateList(items)) if items.len() == 1
    ));

    let groupprivate = parse_cpp("#pragma omp groupprivate(::ns::value)")
        .expect("qualified groupprivate variable must parse");
    assert!(matches!(
        groupprivate.directive().parameter(),
        Some(OmpDirectiveParameter::GroupprivateList(items)) if items.len() == 1
    ));

    let declare_target = parse_cpp("#pragma omp declare target(::ns::value, ns::procedure)")
        .expect("historical declare-target extended list must parse");
    let Some(OmpDirectiveParameter::DeclareTargetList(items)) =
        declare_target.directive().parameter()
    else {
        panic!("declare target list must have its directive-specific AST variant");
    };
    assert!(matches!(
        items.as_slice(),
        [
            OmpDeclareTargetListItem::Name(_),
            OmpDeclareTargetListItem::Name(_)
        ]
    ));
}

#[test]
fn storage_directives_preserve_fortran_common_blocks() {
    for (source, expected_parameter) in [
        ("!$omp allocate(/BLOCK/)", "allocate"),
        ("!$omp threadprivate(/BLOCK/)", "threadprivate"),
        ("!$omp groupprivate(/BLOCK/)", "groupprivate"),
        ("!$omp declare target(/BLOCK/)", "declare_target"),
    ] {
        let parsed = parse_fortran(source)
            .unwrap_or_else(|error| panic!("rejected {expected_parameter}: {error}"));
        let common_block = match parsed.directive().parameter() {
            Some(OmpDirectiveParameter::AllocateList(items))
            | Some(OmpDirectiveParameter::ThreadprivateList(items))
            | Some(OmpDirectiveParameter::GroupprivateList(items)) => match items.as_slice() {
                [OmpStorageListItem::FortranCommonBlock(name)] => name,
                other => panic!("unexpected {expected_parameter} items: {other:?}"),
            },
            Some(OmpDirectiveParameter::DeclareTargetList(items)) => match items.as_slice() {
                [OmpDeclareTargetListItem::FortranCommonBlock(name)] => name,
                other => panic!("unexpected declare-target items: {other:?}"),
            },
            other => panic!("unexpected {expected_parameter} parameter: {other:?}"),
        };
        assert_eq!(common_block.as_str(), "block");
    }
}

#[test]
fn storage_lists_reject_variable_parts_instead_of_widening_the_ast() {
    for source in [
        "#pragma omp allocate(array[0])",
        "#pragma omp allocate(object.member)",
        "#pragma omp threadprivate(array[0])",
        "#pragma omp threadprivate(object.member)",
        "#pragma omp groupprivate(array[0])",
        "#pragma omp groupprivate(object.member)",
        "#pragma omp declare target(array[0:length])",
        "#pragma omp declare target(object.member)",
    ] {
        assert!(
            parse_c(source).is_err(),
            "part of a variable was accepted as a whole storage entity: {source:?}"
        );
    }
}

#[test]
fn declare_variant_accepts_historical_cpp_template_ids_cumulatively() {
    let source =
        "#pragma omp declare variant(ns::fast<std::vector<int>>) match(construct={parallel})";
    for version in [
        OpenMpVersion::V5_0,
        OpenMpVersion::V5_1,
        OpenMpVersion::V5_2,
        OpenMpVersion::V6_0,
    ] {
        let parsed = parse_cpp_exact(version, source)
            .unwrap_or_else(|error| panic!("OpenMP {version} rejected {source:?}: {error}"));
        assert!(matches!(
            parsed.directive().parameter(),
            Some(OmpDirectiveParameter::DeclareVariant(target))
                if target.base().is_none()
                    && matches!(target.variant(), OmpFunctionName::CppTemplateId(_))
        ));
    }

    let qualified =
        parse_cpp("#pragma omp declare variant(A<int>::fast<long>) match(construct={parallel})")
            .expect("qualified C++ template-id must parse");
    assert!(matches!(
        qualified.directive().parameter(),
        Some(OmpDirectiveParameter::DeclareVariant(target))
            if matches!(target.variant(), OmpFunctionName::CppTemplateId(_))
    ));
}

#[test]
fn declare_variant_keeps_base_and_variant_as_separate_typed_fields() {
    let parsed =
        parse_cpp("#pragma omp declare variant(base:ns::fast<long>) match(construct={parallel})")
            .expect("base-name form must parse");
    let Some(OmpDirectiveParameter::DeclareVariant(target)) = parsed.directive().parameter() else {
        panic!("declare variant must have a typed target");
    };
    assert_eq!(target.base().map(|name| name.as_str()), Some("base"));
    assert!(matches!(
        target.variant(),
        OmpFunctionName::CppTemplateId(template_id)
            if template_id.to_string() == "ns::fast<long>"
    ));

    for source in [
        "#pragma omp declare variant() match(construct={parallel})",
        "#pragma omp declare variant(fast<int,>) match(construct={parallel})",
        "#pragma omp declare variant(fast + slow) match(construct={parallel})",
        "#pragma omp begin declare variant(foo)",
    ] {
        assert!(
            parse_cpp(source).is_err(),
            "malformed or parameterized begin-declare-variant was accepted: {source:?}"
        );
    }
}

#[test]
fn declare_variant_base_name_uses_its_host_specific_historical_floor() {
    let cpp = "#pragma omp declare variant(base:fast) match(construct={parallel})";
    assert!(parse_cpp_exact(OpenMpVersion::V5_1, cpp).is_err());
    for version in [OpenMpVersion::V5_2, OpenMpVersion::V6_0] {
        parse_cpp_exact(version, cpp)
            .unwrap_or_else(|error| panic!("OpenMP {version} rejected {cpp:?}: {error}"));
    }

    let fortran = "!$omp declare variant(BASE:FAST) match(construct={parallel})";
    for version in [
        OpenMpVersion::V5_0,
        OpenMpVersion::V5_1,
        OpenMpVersion::V5_2,
        OpenMpVersion::V6_0,
    ] {
        parse_fortran_exact(version, fortran)
            .unwrap_or_else(|error| panic!("OpenMP {version} rejected {fortran:?}: {error}"));
    }
}

#[test]
fn declare_simd_present_target_is_required_and_fortran_only() {
    let parsed = parse_fortran("!$omp declare simd(PROC) simdlen(4)")
        .expect("Fortran declare-simd proc-name must parse");
    let Some(OmpDirectiveParameter::DeclareSimd(target)) = parsed.directive().parameter() else {
        panic!("present declare-simd proc-name must have a typed target");
    };
    assert_eq!(target.function().as_str(), "proc");

    assert!(parse_fortran("!$omp declare simd()").is_err());
    assert!(parse_c("#pragma omp declare simd(proc)").is_err());

    let bare = parse_c("#pragma omp declare simd simdlen(4)")
        .expect("C declare simd is associated with the following declaration");
    assert!(bare.directive().parameter().is_none());
}

#[test]
fn empty_and_unknown_directive_parameters_are_hard_errors() {
    for source in [
        "#pragma omp flush()",
        "#pragma omp barrier(unexpected)",
        "#pragma omp begin declare variant(unexpected)",
    ] {
        assert!(
            parse_c(source).is_err(),
            "unexpected directive parameter was silently retained: {source:?}"
        );
    }
}
