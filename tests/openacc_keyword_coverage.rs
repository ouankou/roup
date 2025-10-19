use roup::parser::{openacc, ClauseKind};

fn parse_directive(input: &str) -> roup::parser::Directive<'_> {
    let parser = openacc::parser();
    let (_, directive) = parser.parse(input).expect("directive should parse");
    directive
}

#[test]
fn all_openacc34_directives_parse() {
    let cases = [
        ("#pragma acc parallel", "parallel"),
        ("#pragma acc serial", "serial"),
        ("#pragma acc kernels", "kernels"),
        ("#pragma acc data copy(a)", "data"),
        ("#pragma acc enter data copyin(a)", "enter data"),
        ("#pragma acc enter_data copyin(a)", "enter_data"),
        ("#pragma acc exit data delete(a)", "exit data"),
        ("#pragma acc exit_data delete(a)", "exit_data"),
        ("#pragma acc host_data use_device(ptr)", "host_data"),
        ("#pragma acc host data use_device(ptr)", "host data"),
        ("#pragma acc loop gang", "loop"),
        ("#pragma acc kernels loop independent", "kernels loop"),
        ("#pragma acc parallel loop gang", "parallel loop"),
        ("#pragma acc serial loop seq", "serial loop"),
        ("#pragma acc atomic update", "atomic"),
        ("#pragma acc atomic capture", "atomic"),
        ("#pragma acc cache(arr)", "cache(arr)"),
        ("#pragma acc wait(1)", "wait(1)"),
        ("#pragma acc declare create(a)", "declare"),
        ("#pragma acc routine gang", "routine"),
        ("#pragma acc init device_type(default)", "init"),
        ("#pragma acc shutdown", "shutdown"),
        ("#pragma acc set device_num(0)", "set"),
        ("#pragma acc update host(a)", "update"),
        ("#pragma acc end parallel loop", "end parallel loop"),
    ];

    for (pragma, expected_name) in cases {
        let directive = parse_directive(pragma);
        assert_eq!(directive.name, expected_name, "{}", pragma);
    }
}

enum ClauseExpectation {
    Bare,
    Parenthesized,
}

fn assert_clause_case(pragma: &str, expected_name: &str, expected_kind: ClauseExpectation) {
    let directive = parse_directive(pragma);
    let clause = directive
        .clauses
        .last()
        .unwrap_or_else(|| panic!("{} should contain clause", pragma));
    assert_eq!(clause.name, expected_name, "{}", pragma);
    match (expected_kind, &clause.kind) {
        (ClauseExpectation::Bare, ClauseKind::Bare) => {}
        (ClauseExpectation::Parenthesized, ClauseKind::Parenthesized(_)) => {}
        (ClauseExpectation::Bare, ClauseKind::Parenthesized(_)) => {
            panic!("{} expected bare clause", pragma)
        }
        (ClauseExpectation::Parenthesized, ClauseKind::Bare) => {
            panic!("{} expected parenthesized clause", pragma)
        }
    }
}

#[test]
fn all_openacc34_clauses_parse() {
    use ClauseExpectation::{Bare, Parenthesized};

    let cases = [
        ("#pragma acc parallel async", "async", Bare),
        ("#pragma acc parallel wait(1)", "wait", Parenthesized),
        (
            "#pragma acc parallel num_gangs(4)",
            "num_gangs",
            Parenthesized,
        ),
        (
            "#pragma acc parallel num_workers(8)",
            "num_workers",
            Parenthesized,
        ),
        (
            "#pragma acc parallel vector_length(128)",
            "vector_length",
            Parenthesized,
        ),
        ("#pragma acc loop gang", "gang", Bare),
        ("#pragma acc loop worker", "worker", Bare),
        ("#pragma acc loop vector", "vector", Bare),
        ("#pragma acc loop seq", "seq", Bare),
        ("#pragma acc loop independent", "independent", Bare),
        ("#pragma acc loop auto", "auto", Bare),
        ("#pragma acc loop collapse(2)", "collapse", Parenthesized),
        ("#pragma acc loop tile(8)", "tile", Parenthesized),
        (
            "#pragma acc loop device_type(nvidia)",
            "device_type",
            Parenthesized,
        ),
        ("#pragma acc loop dtype(*)", "dtype", Parenthesized),
        ("#pragma acc parallel if(condition)", "if", Parenthesized),
        (
            "#pragma acc parallel default(present)",
            "default",
            Parenthesized,
        ),
        (
            "#pragma acc parallel firstprivate(x)",
            "firstprivate",
            Parenthesized,
        ),
        (
            "#pragma acc parallel reduction(+:sum)",
            "reduction",
            Parenthesized,
        ),
        ("#pragma acc parallel self", "self", Bare),
        ("#pragma acc routine bind(foo)", "bind", Parenthesized),
        ("#pragma acc routine nohost", "nohost", Bare),
        ("#pragma acc routine seq", "seq", Bare),
        ("#pragma acc routine gang", "gang", Bare),
        (
            "#pragma acc set default_async(queue0)",
            "default_async",
            Parenthesized,
        ),
        ("#pragma acc set device_num(1)", "device_num", Parenthesized),
        (
            "#pragma acc init device_type(default)",
            "device_type",
            Parenthesized,
        ),
        (
            "#pragma acc shutdown device_type(multicore)",
            "device_type",
            Parenthesized,
        ),
        ("#pragma acc data copy(a)", "copy", Parenthesized),
        ("#pragma acc data pcopy(a)", "pcopy", Parenthesized),
        (
            "#pragma acc data present_or_copy(a)",
            "present_or_copy",
            Parenthesized,
        ),
        ("#pragma acc data copyin(a)", "copyin", Parenthesized),
        ("#pragma acc data pcopyin(a)", "pcopyin", Parenthesized),
        (
            "#pragma acc data present_or_copyin(a)",
            "present_or_copyin",
            Parenthesized,
        ),
        ("#pragma acc data copyout(a)", "copyout", Parenthesized),
        ("#pragma acc data pcopyout(a)", "pcopyout", Parenthesized),
        (
            "#pragma acc data present_or_copyout(a)",
            "present_or_copyout",
            Parenthesized,
        ),
        ("#pragma acc data create(a)", "create", Parenthesized),
        ("#pragma acc data pcreate(a)", "pcreate", Parenthesized),
        (
            "#pragma acc data present_or_create(a)",
            "present_or_create",
            Parenthesized,
        ),
        ("#pragma acc data delete(a)", "delete", Parenthesized),
        ("#pragma acc data no_create(a)", "no_create", Parenthesized),
        ("#pragma acc data present(a)", "present", Parenthesized),
        ("#pragma acc data private(a)", "private", Parenthesized),
        (
            "#pragma acc data firstprivate(a)",
            "firstprivate",
            Parenthesized,
        ),
        (
            "#pragma acc data reduction(+:a)",
            "reduction",
            Parenthesized,
        ),
        ("#pragma acc data link(a)", "link", Parenthesized),
        (
            "#pragma acc data deviceptr(ptr)",
            "deviceptr",
            Parenthesized,
        ),
        (
            "#pragma acc declare device_resident(x)",
            "device_resident",
            Parenthesized,
        ),
        (
            "#pragma acc enter data attach(ptr)",
            "attach",
            Parenthesized,
        ),
        ("#pragma acc exit data detach(ptr)", "detach", Parenthesized),
        ("#pragma acc exit data finalize", "finalize", Bare),
        ("#pragma acc update host(arr)", "host", Parenthesized),
        ("#pragma acc update device(arr)", "device", Parenthesized),
        ("#pragma acc update wait(1)", "wait", Parenthesized),
        (
            "#pragma acc host_data use_device(ptr)",
            "use_device",
            Parenthesized,
        ),
        ("#pragma acc host_data if_present", "if_present", Bare),
        ("#pragma acc atomic read", "read", Bare),
        ("#pragma acc atomic write", "write", Bare),
        ("#pragma acc atomic capture", "capture", Bare),
        ("#pragma acc atomic update", "update", Bare),
    ];

    for (pragma, clause_name, kind) in cases {
        assert_clause_case(pragma, clause_name, kind);
    }
}
