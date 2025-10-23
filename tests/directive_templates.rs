use roup::ir::{
    convert::convert_directive, DirectiveBuilder, Language, ParserConfig, SourceLocation,
};
use roup::parser::parse_omp_directive;

fn template_for(input: &str) -> String {
    let (_, directive) = parse_omp_directive(input).expect("failed to parse directive");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("failed to build IR");
    ir.to_template_string()
}

#[test]
fn map_clause_template_removes_user_symbols() {
    let template = template_for(
        "#pragma omp target data map(tofrom: a[0:ARRAY_SIZE], num_teams) map(to: b[0:ARRAY_SIZE])",
    );
    assert_eq!(
        template, "#pragma omp target data map(tofrom: ) map(to: )",
        "map clause template should drop variable names while preserving map type"
    );
}

#[test]
fn schedule_and_reduction_templates_preserve_keywords() {
    let template =
        template_for("#pragma omp for schedule(static,64) collapse(2) reduction(*: sum)");
    assert_eq!(
        template, "#pragma omp for schedule(static, ) collapse() reduction(*: )",
        "template keeps schedule kind and reduction operator while removing expressions"
    );
}

#[test]
fn fortran_language_uses_fortran_prefix() {
    let directive = DirectiveBuilder::target().build(SourceLocation::start(), Language::Fortran);
    assert!(
        directive.to_template_string().starts_with("!$omp "),
        "Fortran directives must use !$omp prefix in template output"
    );
}
