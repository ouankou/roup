use roup::parser::{
    openacc::{self, OpenAccClause, OpenAccDirective},
    parse_acc_directive, ClauseKind,
};

#[derive(Clone, Copy)]
enum ExpectedClause<'a> {
    Bare,
    Parenthesized(&'a str),
}

fn clause_sample(clause: OpenAccClause) -> (&'static str, ExpectedClause<'static>) {
    use ExpectedClause::{Bare, Parenthesized};

    match clause {
        OpenAccClause::Async => ("#pragma acc parallel async", Bare),
        OpenAccClause::Attach => ("#pragma acc enter data attach(ptr)", Parenthesized("ptr")),
        OpenAccClause::Auto => ("#pragma acc loop auto", Bare),
        OpenAccClause::Bind => (
            "#pragma acc routine bind(device_func)",
            Parenthesized("device_func"),
        ),
        OpenAccClause::Capture => ("#pragma acc atomic capture", Bare),
        OpenAccClause::Collapse => ("#pragma acc loop collapse(2)", Parenthesized("2")),
        OpenAccClause::Copy => ("#pragma acc data copy(arr)", Parenthesized("arr")),
        OpenAccClause::Copyin => ("#pragma acc data copyin(arr)", Parenthesized("arr")),
        OpenAccClause::Copyout => ("#pragma acc data copyout(arr)", Parenthesized("arr")),
        OpenAccClause::Create => ("#pragma acc data create(arr)", Parenthesized("arr")),
        OpenAccClause::Default => (
            "#pragma acc parallel default(present)",
            Parenthesized("present"),
        ),
        OpenAccClause::DefaultAsync => (
            "#pragma acc set default_async(acc_async_sync)",
            Parenthesized("acc_async_sync"),
        ),
        OpenAccClause::Delete => ("#pragma acc exit data delete(arr)", Parenthesized("arr")),
        OpenAccClause::Detach => ("#pragma acc exit data detach(ptr)", Parenthesized("ptr")),
        OpenAccClause::Device => ("#pragma acc update device(arr)", Parenthesized("arr")),
        OpenAccClause::DeviceNum => ("#pragma acc set device_num(0)", Parenthesized("0")),
        OpenAccClause::DeviceResident => (
            "#pragma acc declare device_resident(arr)",
            Parenthesized("arr"),
        ),
        OpenAccClause::DeviceType => (
            "#pragma acc set device_type(nvidia)",
            Parenthesized("nvidia"),
        ),
        OpenAccClause::Deviceptr => ("#pragma acc data deviceptr(ptr)", Parenthesized("ptr")),
        OpenAccClause::Finalize => ("#pragma acc exit data finalize", Bare),
        OpenAccClause::Firstprivate => ("#pragma acc parallel firstprivate(x)", Parenthesized("x")),
        OpenAccClause::Gang => ("#pragma acc loop gang(num:4)", Parenthesized("num:4")),
        OpenAccClause::Host => ("#pragma acc update host(arr)", Parenthesized("arr")),
        OpenAccClause::If => (
            "#pragma acc parallel if(condition)",
            Parenthesized("condition"),
        ),
        OpenAccClause::IfPresent => ("#pragma acc host_data use_device(ptr) if_present", Bare),
        OpenAccClause::Independent => ("#pragma acc loop independent", Bare),
        OpenAccClause::Link => ("#pragma acc declare link(arr)", Parenthesized("arr")),
        OpenAccClause::NoCreate => ("#pragma acc data no_create(arr)", Parenthesized("arr")),
        OpenAccClause::Nohost => ("#pragma acc routine nohost", Bare),
        OpenAccClause::NumGangs => ("#pragma acc parallel num_gangs(4)", Parenthesized("4")),
        OpenAccClause::NumWorkers => ("#pragma acc parallel num_workers(8)", Parenthesized("8")),
        OpenAccClause::Pcopy => ("#pragma acc data pcopy(arr)", Parenthesized("arr")),
        OpenAccClause::Pcopyin => ("#pragma acc data pcopyin(arr)", Parenthesized("arr")),
        OpenAccClause::Pcopyout => ("#pragma acc data pcopyout(arr)", Parenthesized("arr")),
        OpenAccClause::Pcreate => ("#pragma acc data pcreate(arr)", Parenthesized("arr")),
        OpenAccClause::Present => ("#pragma acc data present(arr)", Parenthesized("arr")),
        OpenAccClause::PresentOrCopy => (
            "#pragma acc data present_or_copy(arr)",
            Parenthesized("arr"),
        ),
        OpenAccClause::PresentOrCopyin => (
            "#pragma acc data present_or_copyin(arr)",
            Parenthesized("arr"),
        ),
        OpenAccClause::PresentOrCopyout => (
            "#pragma acc data present_or_copyout(arr)",
            Parenthesized("arr"),
        ),
        OpenAccClause::PresentOrCreate => (
            "#pragma acc data present_or_create(arr)",
            Parenthesized("arr"),
        ),
        OpenAccClause::Private => ("#pragma acc parallel private(x)", Parenthesized("x")),
        OpenAccClause::Reduction => (
            "#pragma acc parallel reduction(+:sum)",
            Parenthesized("+:sum"),
        ),
        OpenAccClause::Read => ("#pragma acc atomic read", Bare),
        OpenAccClause::SelfClause => ("#pragma acc update self(arr)", Parenthesized("arr")),
        OpenAccClause::Seq => ("#pragma acc loop seq", Bare),
        OpenAccClause::Tile => ("#pragma acc loop tile(8,8)", Parenthesized("8,8")),
        OpenAccClause::Update => ("#pragma acc atomic update", Bare),
        OpenAccClause::UseDevice => (
            "#pragma acc host_data use_device(ptr)",
            Parenthesized("ptr"),
        ),
        OpenAccClause::Vector => (
            "#pragma acc loop vector(length:32)",
            Parenthesized("length:32"),
        ),
        OpenAccClause::VectorLength => (
            "#pragma acc parallel vector_length(32)",
            Parenthesized("32"),
        ),
        OpenAccClause::Wait => ("#pragma acc parallel wait(1)", Parenthesized("1")),
        OpenAccClause::Worker => ("#pragma acc loop worker(num:2)", Parenthesized("num:2")),
        OpenAccClause::Write => ("#pragma acc atomic write", Bare),
    }
}

fn directive_sample(directive: OpenAccDirective) -> &'static str {
    match directive {
        OpenAccDirective::Atomic => "#pragma acc atomic update",
        OpenAccDirective::Cache => "#pragma acc cache(arr)",
        OpenAccDirective::Data => "#pragma acc data copy(arr)",
        OpenAccDirective::Declare => "#pragma acc declare create(arr)",
        OpenAccDirective::End => "#pragma acc end parallel loop",
        OpenAccDirective::EnterData => "#pragma acc enter data copyin(arr)",
        OpenAccDirective::EnterDataUnderscore => "#pragma acc enter_data copyin(arr)",
        OpenAccDirective::ExitData => "#pragma acc exit data delete(arr)",
        OpenAccDirective::ExitDataUnderscore => "#pragma acc exit_data delete(arr)",
        OpenAccDirective::HostData => "#pragma acc host_data use_device(ptr)",
        OpenAccDirective::HostDataSpace => "#pragma acc host data use_device(ptr)",
        OpenAccDirective::Init => "#pragma acc init",
        OpenAccDirective::Kernels => "#pragma acc kernels",
        OpenAccDirective::KernelsLoop => "#pragma acc kernels loop independent",
        OpenAccDirective::Loop => "#pragma acc loop gang",
        OpenAccDirective::Parallel => "#pragma acc parallel",
        OpenAccDirective::ParallelLoop => "#pragma acc parallel loop gang",
        OpenAccDirective::Routine => "#pragma acc routine seq",
        OpenAccDirective::Serial => "#pragma acc serial",
        OpenAccDirective::SerialLoop => "#pragma acc serial loop seq",
        OpenAccDirective::Set => "#pragma acc set device_num(0)",
        OpenAccDirective::Shutdown => "#pragma acc shutdown",
        OpenAccDirective::Update => "#pragma acc update device(arr)",
        OpenAccDirective::Wait => "#pragma acc wait",
    }
}

#[test]
fn parses_basic_parallel_loop() {
    let input = "#pragma acc parallel loop gang vector tile(32)";
    let (_, directive) = parse_acc_directive(input).expect("should parse");

    assert_eq!(directive.name, "parallel loop");
    assert_eq!(directive.clauses.len(), 3);
    assert_eq!(directive.clauses[0].name, "gang");
    assert_eq!(directive.clauses[0].kind, ClauseKind::Bare);
    assert_eq!(directive.clauses[1].name, "vector");
    assert_eq!(directive.clauses[1].kind, ClauseKind::Bare);
    assert_eq!(directive.clauses[2].name, "tile");
    assert_eq!(
        directive.clauses[2].kind,
        ClauseKind::Parenthesized("32".into())
    );

    let roundtrip = directive.to_pragma_string_with_prefix("#pragma acc");
    assert_eq!(roundtrip, "#pragma acc parallel loop gang vector tile(32)");
}

#[test]
fn parses_wait_directive_with_clauses() {
    let parser = openacc::parser();
    let input = "#pragma acc wait(1) async(2)";
    let (_, directive) = parser.parse(input).expect("should parse");

    assert_eq!(directive.name, "wait(1)");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "async");
    assert_eq!(
        directive.clauses[0].kind,
        ClauseKind::Parenthesized("2".into())
    );

    let roundtrip = directive.to_pragma_string_with_prefix("#pragma acc");
    assert_eq!(roundtrip, "#pragma acc wait(1) async(2)");
}

#[test]
fn openacc_clause_keyword_coverage() {
    let parser = openacc::parser();

    for clause in OpenAccClause::ALL {
        let clause = *clause;
        let (pragma, expected) = clause_sample(clause);
        let clause_name = clause.name();

        let (_, directive) = parser.parse(pragma).expect("clause should parse");
        let parsed_clause = directive
            .clauses
            .iter()
            .find(|c| c.name == clause_name)
            .unwrap_or_else(|| panic!("{} clause missing", clause_name));

        match expected {
            ExpectedClause::Bare => assert!(matches!(parsed_clause.kind, ClauseKind::Bare)),
            ExpectedClause::Parenthesized(expected_text) => match &parsed_clause.kind {
                ClauseKind::Parenthesized(actual) => assert_eq!(actual, expected_text),
                _ => panic!("{} clause should be parenthesized", clause_name),
            },
        }

        let roundtrip = directive.to_pragma_string_with_prefix("#pragma acc");
        assert_eq!(roundtrip, pragma, "round-trip mismatch for {clause_name}");
    }
}

#[test]
fn openacc_directive_keyword_coverage() {
    for directive in OpenAccDirective::ALL {
        let directive = *directive;
        let pragma = directive_sample(directive);
        let (_, parsed) = parse_acc_directive(pragma).expect("directive should parse");

        let roundtrip = parsed.to_pragma_string_with_prefix("#pragma acc");
        assert_eq!(
            roundtrip,
            pragma,
            "round-trip mismatch for {}",
            directive.as_str()
        );
    }
}

#[test]
fn self_clause_allows_bare_and_parenthesized() {
    let parser = openacc::parser();

    // Bare form used on compute constructs
    let (_, parallel) = parser
        .parse("#pragma acc parallel self")
        .expect("bare self should parse");
    assert!(parallel
        .clauses
        .iter()
        .any(|c| c.name == "self" && matches!(c.kind, ClauseKind::Bare)));

    // Parenthesized form used on update directive
    let (_, update) = parser
        .parse("#pragma acc update self(arr)")
        .expect("parenthesized self should parse");
    assert!(update.clauses.iter().any(|c| {
        c.name == "self" && matches!(c.kind, ClauseKind::Parenthesized(ref value) if value == "arr")
    }));
}
