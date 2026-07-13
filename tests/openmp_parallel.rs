use roup::api::{OpenMpConfig, OpenMpParser};
use roup::ast::{OmpClauseKind, OmpDirectiveKind, OmpReductionIdentifier};
use roup::host::{BinaryOp, CppTemplateArgument, ExprKind, Literal, MemberAccess};
use roup::ir::{ClauseData, ClauseItem, ProcBind, ScheduleKind};
use roup::version::{CStandard, CppStandard, HostLanguageProfile, OpenMpVersion, SourceForm};

fn parser() -> OpenMpParser {
    OpenMpConfig::new(HostLanguageProfile::C(CStandard::C23), SourceForm::Pragma)
        .expect("valid C OpenMP configuration")
        .parser()
}

fn item_name(item: &ClauseItem) -> &str {
    match item {
        ClauseItem::Identifier(identifier) => identifier.as_str(),
        ClauseItem::Variable(variable) => variable.expression().source(),
        ClauseItem::FortranCommonBlock(name) => name.as_str(),
        ClauseItem::Expression(expression) => expression.source(),
        ClauseItem::OmpparserTrailingSlash(identifier) => identifier.as_str(),
    }
}

#[test]
fn parallel_clauses_are_typed_at_the_public_boundary() {
    let parsed = parser()
        .parse("#pragma omp parallel private(a, b) firstprivate(c) num_threads(4) proc_bind(close)")
        .expect("standard parallel clauses should parse");
    let directive = parsed.directive();

    assert_eq!(directive.kind(), OmpDirectiveKind::Parallel);
    assert_eq!(
        directive
            .clauses()
            .iter()
            .map(|clause| clause.kind())
            .collect::<Vec<_>>(),
        [
            OmpClauseKind::Private,
            OmpClauseKind::Firstprivate,
            OmpClauseKind::NumThreads,
            OmpClauseKind::ProcBind,
        ]
    );

    let ClauseData::Private { items } = directive.clauses()[0].payload() else {
        panic!("private must have a typed private payload");
    };
    assert_eq!(items.iter().map(item_name).collect::<Vec<_>>(), ["a", "b"]);

    let ClauseData::Firstprivate {
        modifier: None,
        items,
    } = directive.clauses()[1].payload()
    else {
        panic!("firstprivate must have a typed firstprivate payload");
    };
    assert_eq!(items.iter().map(item_name).collect::<Vec<_>>(), ["c"]);

    let ClauseData::NumThreads { nthreads, .. } = directive.clauses()[2].payload() else {
        panic!("num_threads must retain an expression AST");
    };
    assert_eq!(nthreads[0].source(), "4");
    assert_eq!(
        directive.clauses()[3].payload(),
        &ClauseData::ProcBind(ProcBind::Close)
    );
}

#[test]
fn combined_parallel_loop_preserves_typed_modifier_payloads() {
    let parsed = parser()
        .parse(
            "#pragma omp parallel for simd aligned(buf:64) schedule(static,4) collapse(2) reduction(+:sum)",
        )
        .expect("standard combined construct should parse");
    let directive = parsed.directive();

    assert_eq!(directive.kind(), OmpDirectiveKind::ParallelForSimd);
    let ClauseData::Schedule {
        kind,
        modifiers,
        chunk_size,
    } = directive.clauses()[1].payload()
    else {
        panic!("schedule must be typed");
    };
    assert_eq!(*kind, ScheduleKind::Static);
    assert!(modifiers.is_empty());
    assert_eq!(chunk_size.as_ref().map(|value| value.source()), Some("4"));

    let ClauseData::Reduction {
        modifiers,
        operator,
        items,
        ..
    } = directive.clauses()[3].payload()
    else {
        panic!("reduction must be typed");
    };
    assert!(modifiers.is_empty());
    assert_eq!(operator, &OmpReductionIdentifier::Add);
    assert_eq!(items.iter().map(item_name).collect::<Vec<_>>(), ["sum"]);
}

#[test]
fn historical_proc_bind_master_is_accepted_and_canonicalized_in_six_zero() {
    let parser = OpenMpConfig::exact(
        OpenMpVersion::V6_0,
        HostLanguageProfile::C(CStandard::C23),
        SourceForm::Pragma,
    )
    .unwrap()
    .parser();
    let parsed = parser
        .parse("#pragma omp parallel proc_bind(master)")
        .expect("historical standardized syntax remains accepted");

    assert_eq!(
        parsed.directive().clauses()[0].payload(),
        &ClauseData::ProcBind(ProcBind::Primary)
    );
}

#[test]
fn cpp_if_relational_chain_is_not_misclassified_as_a_template_id() {
    let parser = OpenMpConfig::new(
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
        SourceForm::Pragma,
    )
    .expect("valid C++ OpenMP configuration")
    .parser();
    for source in [
        "#pragma omp parallel if(a < b > c)",
        "#pragma omp parallel if(a < b > -1)",
        "#pragma omp parallel if(a < b > +c)",
        "#pragma omp parallel if(a < b > (c))",
    ] {
        let parsed = parser
            .parse(source)
            .expect("ordinary C++ relational expression must remain viable");
        let ClauseData::If { condition } = parsed.directive().clauses()[0].payload() else {
            panic!("if clause must retain a typed condition")
        };
        assert!(
            matches!(
                condition.ast().kind,
                ExprKind::Binary {
                    op: BinaryOp::Greater,
                    ..
                }
            ),
            "expected a relational expression for {source}"
        );
        if source == "#pragma omp parallel if(a < b > (c))" {
            let ExprKind::Binary { ref right, .. } = condition.ast().kind else {
                unreachable!("the root was already checked as a binary expression")
            };
            assert!(matches!(right.kind, ExprKind::Parenthesized(_)));
        }
    }

    let parsed = parser
        .parse("#pragma omp parallel if(factory<int>(value))")
        .expect("a tightly spelled template-id followed by call arguments must remain a call");
    let ClauseData::If { condition } = parsed.directive().clauses()[0].payload() else {
        panic!("if clause must retain a typed condition")
    };
    assert!(matches!(
        condition.ast().kind,
        ExprKind::Call { ref callee, ref arguments }
            if matches!(callee.kind, ExprKind::CppTemplateId { .. })
                && arguments.len() == 1
    ));
}

#[test]
fn cpp_if_preserves_template_id_values_before_binary_plus_and_minus() {
    let parser = OpenMpConfig::new(
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
        SourceForm::Pragma,
    )
    .expect("valid C++ OpenMP configuration")
    .parser();
    for (source, expected_operator, expected_compact) in [
        (
            "#pragma omp parallel if(var<T> + 1)",
            BinaryOp::Add,
            "var<T>+1",
        ),
        (
            "#pragma omp parallel if(var<T> - 1)",
            BinaryOp::Subtract,
            "var<T>-1",
        ),
    ] {
        let parsed = parser
            .parse(source)
            .expect("a tightly spelled template-id may be a binary operand");
        let ClauseData::If { condition } = parsed.directive().clauses()[0].payload() else {
            panic!("if clause must retain a typed condition")
        };
        assert!(matches!(
            condition.ast().kind,
            ExprKind::Binary { op, ref left, .. }
                if op == expected_operator
                    && matches!(left.kind, ExprKind::CppTemplateId { .. })
        ));
        assert_eq!(condition.compact_source_spelling(), expected_compact);
    }
}

#[test]
fn cpp_if_accepts_a_template_qualified_static_member() {
    let parser = OpenMpConfig::new(
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
        SourceForm::Pragma,
    )
    .expect("valid C++ OpenMP configuration")
    .parser();
    let parsed = parser
        .parse("#pragma omp parallel if(Trait<int>::enabled)")
        .expect("a template-id may qualify a static member");
    let ClauseData::If { condition } = parsed.directive().clauses()[0].payload() else {
        panic!("if clause must retain a typed condition")
    };
    assert!(matches!(
        condition.ast().kind,
        ExprKind::Member {
            ref base,
            access: MemberAccess::Scope,
            ref member,
        } if matches!(base.kind, ExprKind::CppTemplateId { .. })
            && member.as_str() == "enabled"
    ));
    assert_eq!(condition.compact_source_spelling(), "Trait<int>::enabled");
}

#[test]
fn cpp_if_retains_expression_valued_template_arguments() {
    let parser = OpenMpConfig::new(
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
        SourceForm::Pragma,
    )
    .expect("valid C++ OpenMP configuration")
    .parser();
    let parsed = parser
        .parse("#pragma omp parallel if(Trait<4>::enabled)")
        .expect("a non-type template argument must remain a typed expression");
    let ClauseData::If { condition } = parsed.directive().clauses()[0].payload() else {
        panic!("if clause must retain a typed condition")
    };
    let ExprKind::Member { base, .. } = &condition.ast().kind else {
        panic!("condition must remain a template-qualified static member")
    };
    let ExprKind::CppTemplateId { arguments, .. } = &base.kind else {
        panic!("static-member base must remain a typed template-id")
    };
    assert!(matches!(
        arguments.as_slice(),
        [CppTemplateArgument::Expression(value)]
            if matches!(value.kind, ExprKind::Literal(Literal::Integer(_)))
    ));
    assert_eq!(condition.compact_source_spelling(), "Trait<4>::enabled");

    let parsed = parser
        .parse("#pragma omp parallel if(factory<int, 4, N, (N > 0)>())")
        .expect("mixed type, value, and dependent template arguments must parse");
    let ClauseData::If { condition } = parsed.directive().clauses()[0].payload() else {
        panic!("if clause must retain a typed condition")
    };
    let ExprKind::Call { callee, .. } = &condition.ast().kind else {
        panic!("condition must remain a template function call")
    };
    let ExprKind::CppTemplateId { arguments, .. } = &callee.kind else {
        panic!("callee must remain a typed template-id")
    };
    assert!(matches!(&arguments[0], CppTemplateArgument::Type(_)));
    assert!(matches!(&arguments[1], CppTemplateArgument::Expression(_)));
    assert!(matches!(
        &arguments[2],
        CppTemplateArgument::Ambiguous { .. }
    ));
    assert!(matches!(&arguments[3], CppTemplateArgument::Expression(_)));
    assert_eq!(
        condition.compact_source_spelling(),
        "factory<int,4,N,(N>0)>()"
    );
}

#[test]
fn cpp_if_accepts_nested_template_ids_with_combined_closers() {
    let parser = OpenMpConfig::new(
        HostLanguageProfile::Cpp(CppStandard::Cpp23),
        SourceForm::Pragma,
    )
    .expect("valid C++ OpenMP configuration")
    .parser();
    for (source, expected) in [
        (
            "#pragma omp parallel if(Trait<std::vector<int>>::enabled)",
            "Trait<std::vector<int>>::enabled",
        ),
        (
            "#pragma omp parallel if(factory<std::vector<int>>())",
            "factory<std::vector<int>>()",
        ),
    ] {
        let parsed = parser
            .parse(source)
            .expect("combined template closers must remain typed C++ syntax");
        let ClauseData::If { condition } = parsed.directive().clauses()[0].payload() else {
            panic!("if clause must retain a typed condition")
        };
        assert_eq!(condition.compact_source_spelling(), expected);
    }
}

#[test]
fn invalid_parallel_clauses_are_hard_errors() {
    for source in [
        "#pragma omp parallel unsupported_clause",
        "#pragma omp parallel nowait",
        "#pragma omp parallel private()",
    ] {
        assert!(parser().parse(source).is_err(), "{source} must be rejected");
    }
}
