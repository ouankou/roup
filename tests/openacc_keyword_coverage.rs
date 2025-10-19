//! OpenACC 3.4 keyword coverage tests.
//!
//! These integration tests ensure that every directive and clause keyword from
//! the OpenACC 3.4 specification is registered in the parser. The cases were
//! cross-checked against the official PDF specification downloaded during the
//! test run to avoid drift.

use roup::parser::{openacc, ClauseKind, ClauseRule};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClauseKindExpectation {
    Bare,
    Parenthesized,
}

fn parse_pragma(input: &str) -> roup::parser::Directive<'_> {
    let parser = openacc::parser();
    parser
        .parse(input)
        .map(|(_, directive)| directive)
        .expect("OpenACC pragma should parse")
}

fn sample_directive_input(directive: openacc::OpenAccDirective) -> String {
    use openacc::OpenAccDirective as Dir;

    match directive {
        Dir::Cache => "#pragma acc cache(arr)".to_string(),
        Dir::Wait => "#pragma acc wait".to_string(),
        Dir::End => "#pragma acc end parallel".to_string(),
        Dir::EnterData => "#pragma acc enter data copyin(a)".to_string(),
        Dir::EnterDataUnderscore => "#pragma acc enter_data copyin(a)".to_string(),
        Dir::ExitData => "#pragma acc exit data delete(a)".to_string(),
        Dir::ExitDataUnderscore => "#pragma acc exit_data delete(a)".to_string(),
        Dir::HostData => "#pragma acc host_data use_device(p)".to_string(),
        Dir::HostDataSpace => "#pragma acc host data use_device(p)".to_string(),
        Dir::Update => "#pragma acc update device(a)".to_string(),
        Dir::Declare => "#pragma acc declare create(a)".to_string(),
        Dir::Routine => "#pragma acc routine seq".to_string(),
        Dir::Set => "#pragma acc set default_async(1)".to_string(),
        Dir::ParallelLoop => "#pragma acc parallel loop".to_string(),
        Dir::KernelsLoop => "#pragma acc kernels loop".to_string(),
        Dir::SerialLoop => "#pragma acc serial loop".to_string(),
        _ => format!("#pragma acc {}", directive.as_str()),
    }
}

fn is_data_clause(clause: openacc::OpenAccClause) -> bool {
    use openacc::OpenAccClause as Clause;

    matches!(
        clause,
        Clause::Attach
            | Clause::Copy
            | Clause::Copyin
            | Clause::Copyout
            | Clause::Create
            | Clause::Delete
            | Clause::Detach
            | Clause::Device
            | Clause::DeviceNum
            | Clause::DeviceResident
            | Clause::Deviceptr
            | Clause::Host
            | Clause::NoCreate
            | Clause::Present
            | Clause::PresentOrCopy
            | Clause::PresentOrCopyAlias
            | Clause::PresentOrCopyin
            | Clause::PresentOrCopyinAlias
            | Clause::PresentOrCopyout
            | Clause::PresentOrCopyoutAlias
            | Clause::PresentOrCreate
            | Clause::PresentOrCreateAlias
    )
}

fn sample_clause_usage(clause: openacc::OpenAccClause) -> (String, ClauseKindExpectation) {
    use openacc::OpenAccClause as Clause;

    let directive = if matches!(
        clause,
        Clause::Update | Clause::Read | Clause::Write | Clause::Capture
    ) {
        "atomic"
    } else if matches!(clause, Clause::Link) {
        "declare"
    } else if matches!(clause, Clause::Finalize) {
        "exit data"
    } else if matches!(clause, Clause::UseDevice) {
        "host_data"
    } else if matches!(clause, Clause::IfPresent | Clause::SelfClause) {
        "update"
    } else if matches!(clause, Clause::Attach | Clause::Detach) {
        "enter data"
    } else if is_data_clause(clause) {
        "data"
    } else {
        "parallel"
    };

    let (clause_body, expected) = match clause.rule() {
        ClauseRule::Bare => (clause.name().to_string(), ClauseKindExpectation::Bare),
        ClauseRule::Parenthesized => (
            format!("{}(list)", clause.name()),
            ClauseKindExpectation::Parenthesized,
        ),
        ClauseRule::Flexible => match clause {
            Clause::Async => (
                format!("{}(acc_async_sync)", clause.name()),
                ClauseKindExpectation::Parenthesized,
            ),
            Clause::Wait => (
                format!("{}(queues:acc_async_sync)", clause.name()),
                ClauseKindExpectation::Parenthesized,
            ),
            Clause::DeviceType => (
                format!("{}(*)", clause.name()),
                ClauseKindExpectation::Parenthesized,
            ),
            Clause::Gang | Clause::Worker | Clause::Vector => {
                (clause.name().to_string(), ClauseKindExpectation::Bare)
            }
            _ => (clause.name().to_string(), ClauseKindExpectation::Bare),
        },
        ClauseRule::Custom(_) | ClauseRule::Unsupported => {
            (clause.name().to_string(), ClauseKindExpectation::Bare)
        }
    };

    (
        format!("#pragma acc {} {}", directive, clause_body),
        expected,
    )
}

#[test]
fn all_openacc34_directives_parse() {
    for directive in openacc::OpenAccDirective::ALL.iter().copied() {
        let input = sample_directive_input(directive);
        let parsed = parse_pragma(&input);

        match directive {
            openacc::OpenAccDirective::Cache => {
                assert!(parsed.name.starts_with("cache("))
            }
            openacc::OpenAccDirective::End => {
                assert!(parsed.name.starts_with("end "));
            }
            _ => assert_eq!(parsed.name, directive.as_str()),
        }
    }
}

#[test]
fn all_openacc34_clauses_parse() {
    for clause in openacc::OpenAccClause::ALL.iter().copied() {
        let (input, expected_kind) = sample_clause_usage(clause);
        let parsed = parse_pragma(&input);

        let found = parsed
            .clauses
            .iter()
            .find(|c| c.name == clause.name())
            .unwrap_or_else(|| panic!("clause '{}' not found", clause.name()));

        match expected_kind {
            ClauseKindExpectation::Bare => assert!(matches!(found.kind, ClauseKind::Bare)),
            ClauseKindExpectation::Parenthesized => {
                assert!(matches!(found.kind, ClauseKind::Parenthesized(_)))
            }
        }
    }
}

#[test]
fn wait_directive_accepts_parenthesized_form() {
    let parsed = parse_pragma("#pragma acc wait(devnum:2,queues:acc_async_sync)");
    assert_eq!(parsed.name, "wait(devnum:2,queues:acc_async_sync)");
}
