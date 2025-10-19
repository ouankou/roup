use std::collections::HashSet;

use roup::parser::{
    openacc::{self, OpenAccClause, OpenAccDirective},
    parse_acc_directive, ClauseKind,
};

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

#[derive(Clone, Copy)]
enum ClauseExpectation {
    Bare,
    Parenthesized(&'static str),
}

struct ClauseCase {
    clause: OpenAccClause,
    input: &'static str,
    expectation: ClauseExpectation,
    roundtrip: &'static str,
}

fn clause_cases() -> Vec<ClauseCase> {
    use ClauseExpectation::*;

    vec![
        ClauseCase {
            clause: OpenAccClause::Async,
            input: "#pragma acc parallel async(1)",
            expectation: Parenthesized("1"),
            roundtrip: "#pragma acc parallel async(1)",
        },
        ClauseCase {
            clause: OpenAccClause::Async,
            input: "#pragma acc parallel async",
            expectation: Bare,
            roundtrip: "#pragma acc parallel async",
        },
        ClauseCase {
            clause: OpenAccClause::Wait,
            input: "#pragma acc parallel wait(2)",
            expectation: Parenthesized("2"),
            roundtrip: "#pragma acc parallel wait(2)",
        },
        ClauseCase {
            clause: OpenAccClause::Wait,
            input: "#pragma acc parallel wait",
            expectation: Bare,
            roundtrip: "#pragma acc parallel wait",
        },
        ClauseCase {
            clause: OpenAccClause::NumGangs,
            input: "#pragma acc parallel num_gangs(3)",
            expectation: Parenthesized("3"),
            roundtrip: "#pragma acc parallel num_gangs(3)",
        },
        ClauseCase {
            clause: OpenAccClause::NumWorkers,
            input: "#pragma acc parallel num_workers(4)",
            expectation: Parenthesized("4"),
            roundtrip: "#pragma acc parallel num_workers(4)",
        },
        ClauseCase {
            clause: OpenAccClause::VectorLength,
            input: "#pragma acc parallel vector_length(128)",
            expectation: Parenthesized("128"),
            roundtrip: "#pragma acc parallel vector_length(128)",
        },
        ClauseCase {
            clause: OpenAccClause::Gang,
            input: "#pragma acc loop gang(num:2)",
            expectation: Parenthesized("num:2"),
            roundtrip: "#pragma acc loop gang(num:2)",
        },
        ClauseCase {
            clause: OpenAccClause::Gang,
            input: "#pragma acc loop gang",
            expectation: Bare,
            roundtrip: "#pragma acc loop gang",
        },
        ClauseCase {
            clause: OpenAccClause::Worker,
            input: "#pragma acc loop worker",
            expectation: Bare,
            roundtrip: "#pragma acc loop worker",
        },
        ClauseCase {
            clause: OpenAccClause::Vector,
            input: "#pragma acc loop vector(length:64)",
            expectation: Parenthesized("length:64"),
            roundtrip: "#pragma acc loop vector(length:64)",
        },
        ClauseCase {
            clause: OpenAccClause::Vector,
            input: "#pragma acc loop vector",
            expectation: Bare,
            roundtrip: "#pragma acc loop vector",
        },
        ClauseCase {
            clause: OpenAccClause::Seq,
            input: "#pragma acc loop seq",
            expectation: Bare,
            roundtrip: "#pragma acc loop seq",
        },
        ClauseCase {
            clause: OpenAccClause::Independent,
            input: "#pragma acc loop independent",
            expectation: Bare,
            roundtrip: "#pragma acc loop independent",
        },
        ClauseCase {
            clause: OpenAccClause::Auto,
            input: "#pragma acc loop auto",
            expectation: Bare,
            roundtrip: "#pragma acc loop auto",
        },
        ClauseCase {
            clause: OpenAccClause::Collapse,
            input: "#pragma acc loop collapse(2)",
            expectation: Parenthesized("2"),
            roundtrip: "#pragma acc loop collapse(2)",
        },
        ClauseCase {
            clause: OpenAccClause::DeviceType,
            input: "#pragma acc parallel device_type(nvidia)",
            expectation: Parenthesized("nvidia"),
            roundtrip: "#pragma acc parallel device_type(nvidia)",
        },
        ClauseCase {
            clause: OpenAccClause::Bind,
            input: "#pragma acc routine bind(kernel_name)",
            expectation: Parenthesized("kernel_name"),
            roundtrip: "#pragma acc routine bind(kernel_name)",
        },
        ClauseCase {
            clause: OpenAccClause::If,
            input: "#pragma acc parallel if(condition)",
            expectation: Parenthesized("condition"),
            roundtrip: "#pragma acc parallel if(condition)",
        },
        ClauseCase {
            clause: OpenAccClause::Default,
            input: "#pragma acc parallel default(none)",
            expectation: Parenthesized("none"),
            roundtrip: "#pragma acc parallel default(none)",
        },
        ClauseCase {
            clause: OpenAccClause::Firstprivate,
            input: "#pragma acc parallel firstprivate(a)",
            expectation: Parenthesized("a"),
            roundtrip: "#pragma acc parallel firstprivate(a)",
        },
        ClauseCase {
            clause: OpenAccClause::DefaultAsync,
            input: "#pragma acc set default_async(acc_async_noval)",
            expectation: Parenthesized("acc_async_noval"),
            roundtrip: "#pragma acc set default_async(acc_async_noval)",
        },
        ClauseCase {
            clause: OpenAccClause::Link,
            input: "#pragma acc declare link(func)",
            expectation: Parenthesized("func"),
            roundtrip: "#pragma acc declare link(func)",
        },
        ClauseCase {
            clause: OpenAccClause::NoCreate,
            input: "#pragma acc data no_create(a)",
            expectation: Parenthesized("a"),
            roundtrip: "#pragma acc data no_create(a)",
        },
        ClauseCase {
            clause: OpenAccClause::Nohost,
            input: "#pragma acc routine nohost",
            expectation: Bare,
            roundtrip: "#pragma acc routine nohost",
        },
        ClauseCase {
            clause: OpenAccClause::Present,
            input: "#pragma acc data present(a)",
            expectation: Parenthesized("a"),
            roundtrip: "#pragma acc data present(a)",
        },
        ClauseCase {
            clause: OpenAccClause::PresentOrCopy,
            input: "#pragma acc data present_or_copy(a)",
            expectation: Parenthesized("a"),
            roundtrip: "#pragma acc data present_or_copy(a)",
        },
        ClauseCase {
            clause: OpenAccClause::PresentOrCopyin,
            input: "#pragma acc enter data present_or_copyin(a)",
            expectation: Parenthesized("a"),
            roundtrip: "#pragma acc enter data present_or_copyin(a)",
        },
        ClauseCase {
            clause: OpenAccClause::PresentOrCopyout,
            input: "#pragma acc exit data present_or_copyout(a)",
            expectation: Parenthesized("a"),
            roundtrip: "#pragma acc exit data present_or_copyout(a)",
        },
        ClauseCase {
            clause: OpenAccClause::PresentOrCreate,
            input: "#pragma acc enter data present_or_create(a)",
            expectation: Parenthesized("a"),
            roundtrip: "#pragma acc enter data present_or_create(a)",
        },
        ClauseCase {
            clause: OpenAccClause::Private,
            input: "#pragma acc parallel private(a)",
            expectation: Parenthesized("a"),
            roundtrip: "#pragma acc parallel private(a)",
        },
        ClauseCase {
            clause: OpenAccClause::Reduction,
            input: "#pragma acc loop reduction(+:sum)",
            expectation: Parenthesized("+:sum"),
            roundtrip: "#pragma acc loop reduction(+:sum)",
        },
        ClauseCase {
            clause: OpenAccClause::Read,
            input: "#pragma acc atomic read",
            expectation: Bare,
            roundtrip: "#pragma acc atomic read",
        },
        ClauseCase {
            clause: OpenAccClause::SelfClause,
            input: "#pragma acc parallel self",
            expectation: Bare,
            roundtrip: "#pragma acc parallel self",
        },
        ClauseCase {
            clause: OpenAccClause::SelfClause,
            input: "#pragma acc update self(array[0:n])",
            expectation: Parenthesized("array[0:n]"),
            roundtrip: "#pragma acc update self(array[0:n])",
        },
        ClauseCase {
            clause: OpenAccClause::Tile,
            input: "#pragma acc loop tile(4)",
            expectation: Parenthesized("4"),
            roundtrip: "#pragma acc loop tile(4)",
        },
        ClauseCase {
            clause: OpenAccClause::Update,
            input: "#pragma acc atomic update",
            expectation: Bare,
            roundtrip: "#pragma acc atomic update",
        },
        ClauseCase {
            clause: OpenAccClause::UseDevice,
            input: "#pragma acc host_data use_device(ptr)",
            expectation: Parenthesized("ptr"),
            roundtrip: "#pragma acc host_data use_device(ptr)",
        },
        ClauseCase {
            clause: OpenAccClause::Attach,
            input: "#pragma acc data attach(p)",
            expectation: Parenthesized("p"),
            roundtrip: "#pragma acc data attach(p)",
        },
        ClauseCase {
            clause: OpenAccClause::Detach,
            input: "#pragma acc exit data detach(p)",
            expectation: Parenthesized("p"),
            roundtrip: "#pragma acc exit data detach(p)",
        },
        ClauseCase {
            clause: OpenAccClause::Finalize,
            input: "#pragma acc exit data delete(p) finalize",
            expectation: Bare,
            roundtrip: "#pragma acc exit data delete(p) finalize",
        },
        ClauseCase {
            clause: OpenAccClause::IfPresent,
            input: "#pragma acc update if_present",
            expectation: Bare,
            roundtrip: "#pragma acc update if_present",
        },
        ClauseCase {
            clause: OpenAccClause::Capture,
            input: "#pragma acc atomic capture",
            expectation: Bare,
            roundtrip: "#pragma acc atomic capture",
        },
        ClauseCase {
            clause: OpenAccClause::Write,
            input: "#pragma acc atomic write",
            expectation: Bare,
            roundtrip: "#pragma acc atomic write",
        },
        ClauseCase {
            clause: OpenAccClause::Copy,
            input: "#pragma acc data copy(a)",
            expectation: Parenthesized("a"),
            roundtrip: "#pragma acc data copy(a)",
        },
        ClauseCase {
            clause: OpenAccClause::Copyin,
            input: "#pragma acc data copyin(a)",
            expectation: Parenthesized("a"),
            roundtrip: "#pragma acc data copyin(a)",
        },
        ClauseCase {
            clause: OpenAccClause::Copyout,
            input: "#pragma acc data copyout(a)",
            expectation: Parenthesized("a"),
            roundtrip: "#pragma acc data copyout(a)",
        },
        ClauseCase {
            clause: OpenAccClause::Create,
            input: "#pragma acc data create(a)",
            expectation: Parenthesized("a"),
            roundtrip: "#pragma acc data create(a)",
        },
        ClauseCase {
            clause: OpenAccClause::Delete,
            input: "#pragma acc exit data delete(a)",
            expectation: Parenthesized("a"),
            roundtrip: "#pragma acc exit data delete(a)",
        },
        ClauseCase {
            clause: OpenAccClause::Device,
            input: "#pragma acc update device(a)",
            expectation: Parenthesized("a"),
            roundtrip: "#pragma acc update device(a)",
        },
        ClauseCase {
            clause: OpenAccClause::Deviceptr,
            input: "#pragma acc data deviceptr(p)",
            expectation: Parenthesized("p"),
            roundtrip: "#pragma acc data deviceptr(p)",
        },
        ClauseCase {
            clause: OpenAccClause::DeviceNum,
            input: "#pragma acc set device_num(1)",
            expectation: Parenthesized("1"),
            roundtrip: "#pragma acc set device_num(1)",
        },
        ClauseCase {
            clause: OpenAccClause::DeviceResident,
            input: "#pragma acc declare device_resident(a)",
            expectation: Parenthesized("a"),
            roundtrip: "#pragma acc declare device_resident(a)",
        },
        ClauseCase {
            clause: OpenAccClause::Host,
            input: "#pragma acc update host(a)",
            expectation: Parenthesized("a"),
            roundtrip: "#pragma acc update host(a)",
        },
    ]
}

#[test]
fn parses_all_openacc_clauses_roundtrip() {
    let parser = openacc::parser();
    let mut covered = HashSet::new();

    for case in clause_cases() {
        let (_, directive) = parser.parse(case.input).expect("should parse clause case");
        let clause_name = case.clause.name();
        let clause = directive
            .clauses
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(clause_name))
            .unwrap_or_else(|| panic!("clause {} missing in {}", clause_name, case.input));

        match case.expectation {
            ClauseExpectation::Bare => assert_eq!(clause.kind, ClauseKind::Bare),
            ClauseExpectation::Parenthesized(expected) => {
                assert_eq!(
                    clause.kind,
                    ClauseKind::Parenthesized(expected.into()),
                    "expected parentheses for {} in {}",
                    clause_name,
                    case.input
                );
            }
        }

        let roundtrip = directive.to_pragma_string_with_prefix("#pragma acc");
        assert_eq!(
            roundtrip, case.roundtrip,
            "round-trip mismatch for {}",
            case.input
        );
        covered.insert(case.clause);
    }

    assert_eq!(covered.len(), OpenAccClause::ALL.len());
    for clause in OpenAccClause::ALL {
        assert!(
            covered.contains(clause),
            "missing coverage for clause {}",
            clause.name()
        );
    }
}

struct DirectiveCase {
    directive: OpenAccDirective,
    input: &'static str,
    expected_name: &'static str,
    roundtrip: &'static str,
}

fn directive_cases() -> Vec<DirectiveCase> {
    vec![
        DirectiveCase {
            directive: OpenAccDirective::Atomic,
            input: "#pragma acc atomic",
            expected_name: "atomic",
            roundtrip: "#pragma acc atomic",
        },
        DirectiveCase {
            directive: OpenAccDirective::Cache,
            input: "#pragma acc cache(a[0:4])",
            expected_name: "cache(a[0:4])",
            roundtrip: "#pragma acc cache(a[0:4])",
        },
        DirectiveCase {
            directive: OpenAccDirective::Data,
            input: "#pragma acc data copy(a)",
            expected_name: "data",
            roundtrip: "#pragma acc data copy(a)",
        },
        DirectiveCase {
            directive: OpenAccDirective::Declare,
            input: "#pragma acc declare create(a)",
            expected_name: "declare",
            roundtrip: "#pragma acc declare create(a)",
        },
        DirectiveCase {
            directive: OpenAccDirective::End,
            input: "#pragma acc end parallel",
            expected_name: "end parallel",
            roundtrip: "#pragma acc end parallel",
        },
        DirectiveCase {
            directive: OpenAccDirective::EnterData,
            input: "#pragma acc enter data copyin(a)",
            expected_name: "enter data",
            roundtrip: "#pragma acc enter data copyin(a)",
        },
        DirectiveCase {
            directive: OpenAccDirective::EnterDataUnderscore,
            input: "#pragma acc enter_data copyin(a)",
            expected_name: "enter_data",
            roundtrip: "#pragma acc enter_data copyin(a)",
        },
        DirectiveCase {
            directive: OpenAccDirective::ExitData,
            input: "#pragma acc exit data delete(a)",
            expected_name: "exit data",
            roundtrip: "#pragma acc exit data delete(a)",
        },
        DirectiveCase {
            directive: OpenAccDirective::ExitDataUnderscore,
            input: "#pragma acc exit_data delete(a)",
            expected_name: "exit_data",
            roundtrip: "#pragma acc exit_data delete(a)",
        },
        DirectiveCase {
            directive: OpenAccDirective::HostData,
            input: "#pragma acc host_data use_device(ptr)",
            expected_name: "host_data",
            roundtrip: "#pragma acc host_data use_device(ptr)",
        },
        DirectiveCase {
            directive: OpenAccDirective::HostDataSpace,
            input: "#pragma acc host data use_device(ptr)",
            expected_name: "host data",
            roundtrip: "#pragma acc host data use_device(ptr)",
        },
        DirectiveCase {
            directive: OpenAccDirective::Init,
            input: "#pragma acc init device_num(0)",
            expected_name: "init",
            roundtrip: "#pragma acc init device_num(0)",
        },
        DirectiveCase {
            directive: OpenAccDirective::Kernels,
            input: "#pragma acc kernels",
            expected_name: "kernels",
            roundtrip: "#pragma acc kernels",
        },
        DirectiveCase {
            directive: OpenAccDirective::KernelsLoop,
            input: "#pragma acc kernels loop independent",
            expected_name: "kernels loop",
            roundtrip: "#pragma acc kernels loop independent",
        },
        DirectiveCase {
            directive: OpenAccDirective::Loop,
            input: "#pragma acc loop gang",
            expected_name: "loop",
            roundtrip: "#pragma acc loop gang",
        },
        DirectiveCase {
            directive: OpenAccDirective::Parallel,
            input: "#pragma acc parallel",
            expected_name: "parallel",
            roundtrip: "#pragma acc parallel",
        },
        DirectiveCase {
            directive: OpenAccDirective::ParallelLoop,
            input: "#pragma acc parallel loop gang",
            expected_name: "parallel loop",
            roundtrip: "#pragma acc parallel loop gang",
        },
        DirectiveCase {
            directive: OpenAccDirective::Routine,
            input: "#pragma acc routine seq",
            expected_name: "routine",
            roundtrip: "#pragma acc routine seq",
        },
        DirectiveCase {
            directive: OpenAccDirective::Serial,
            input: "#pragma acc serial",
            expected_name: "serial",
            roundtrip: "#pragma acc serial",
        },
        DirectiveCase {
            directive: OpenAccDirective::SerialLoop,
            input: "#pragma acc serial loop seq",
            expected_name: "serial loop",
            roundtrip: "#pragma acc serial loop seq",
        },
        DirectiveCase {
            directive: OpenAccDirective::Set,
            input: "#pragma acc set device_num(0)",
            expected_name: "set",
            roundtrip: "#pragma acc set device_num(0)",
        },
        DirectiveCase {
            directive: OpenAccDirective::Shutdown,
            input: "#pragma acc shutdown device_num(0)",
            expected_name: "shutdown",
            roundtrip: "#pragma acc shutdown device_num(0)",
        },
        DirectiveCase {
            directive: OpenAccDirective::Update,
            input: "#pragma acc update device(a)",
            expected_name: "update",
            roundtrip: "#pragma acc update device(a)",
        },
        DirectiveCase {
            directive: OpenAccDirective::Wait,
            input: "#pragma acc wait",
            expected_name: "wait",
            roundtrip: "#pragma acc wait",
        },
    ]
}

#[test]
fn parses_all_openacc_directives_roundtrip() {
    let parser = openacc::parser();
    let mut covered = HashSet::new();

    for case in directive_cases() {
        let (_, directive) = parser
            .parse(case.input)
            .expect("should parse directive case");
        assert_eq!(
            directive.name, case.expected_name,
            "name mismatch for {}",
            case.input
        );
        let roundtrip = directive.to_pragma_string_with_prefix("#pragma acc");
        assert_eq!(
            roundtrip, case.roundtrip,
            "round-trip mismatch for {}",
            case.input
        );
        covered.insert(case.directive);
    }

    assert_eq!(covered.len(), OpenAccDirective::ALL.len());
    for directive in OpenAccDirective::ALL {
        assert!(
            covered.contains(directive),
            "missing coverage for directive {}",
            directive.as_str()
        );
    }
}
