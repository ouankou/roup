//! Integration tests for the step-by-step debugger

use roup::debugger::{DebugConfig, DebugSession};

#[test]
fn test_debug_session_openmp_parallel() {
    let input = "#pragma omp parallel shared(x) private(y)";
    let config = DebugConfig::openmp();

    let session = DebugSession::new(input, config).expect("Failed to create debug session");

    // Should have multiple steps
    assert!(
        session.total_steps() > 0,
        "Expected at least one step, got {}",
        session.total_steps()
    );

    // Should successfully parse the directive
    assert!(
        session.final_directive.is_some(),
        "Expected a parsed directive"
    );

    let directive = session.final_directive.as_ref().unwrap();
    assert_eq!(directive.name, "parallel");
    assert_eq!(directive.clauses.len(), 2);
    assert_eq!(directive.clauses[0].name, "shared");
    assert_eq!(directive.clauses[1].name, "private");
}

#[test]
fn test_debug_session_openacc() {
    let input = "#pragma acc parallel async(1) wait(2)";
    let config = DebugConfig::openacc();

    let session = DebugSession::new(input, config).expect("Failed to create debug session");

    // Should successfully parse
    assert!(session.final_directive.is_some());

    let directive = session.final_directive.as_ref().unwrap();
    assert_eq!(directive.name, "parallel");
    assert_eq!(directive.clauses.len(), 2);
}

#[test]
fn test_debug_session_complex_directive() {
    let input = "#pragma omp parallel for simd collapse(2) reduction(+:sum)";
    let config = DebugConfig::openmp();

    let session = DebugSession::new(input, config).expect("Failed to create debug session");

    // Verify the directive was parsed correctly
    let directive = session.final_directive.as_ref().unwrap();
    assert_eq!(directive.name, "parallel for simd");
    assert_eq!(directive.clauses.len(), 2);
}

#[test]
fn test_debug_session_navigation() {
    let input = "#pragma omp parallel shared(x)";
    let config = DebugConfig::openmp();

    let mut session = DebugSession::new(input, config).expect("Failed to create debug session");

    // Test navigation
    let total = session.total_steps();
    assert!(total > 0);

    // Start at step 0
    assert_eq!(session.current_step_index, 0);

    // Move forward
    assert!(session.next_step());
    assert_eq!(session.current_step_index, 1);

    // Move back
    assert!(session.prev_step());
    assert_eq!(session.current_step_index, 0);

    // Can't go before first step
    assert!(!session.prev_step());
    assert_eq!(session.current_step_index, 0);
}

#[test]
fn test_debug_step_kinds() {
    let input = "#pragma omp parallel shared(x)";
    let config = DebugConfig::openmp();

    let session = DebugSession::new(input, config).expect("Failed to create debug session");

    // Check that we have different step kinds
    let step_kinds: Vec<_> = session.steps.iter().map(|s| &s.kind).collect();

    // Should have at least a pragma prefix step
    assert!(step_kinds
        .iter()
        .any(|k| matches!(k, roup::debugger::StepKind::PragmaPrefix)));

    // Should have a directive name step
    assert!(step_kinds
        .iter()
        .any(|k| matches!(k, roup::debugger::StepKind::DirectiveName)));

    // Should have a clause name step
    assert!(step_kinds
        .iter()
        .any(|k| matches!(k, roup::debugger::StepKind::ClauseName)));
}

#[test]
fn test_debug_steps_accumulate_correctly() {
    let input = "#pragma omp parallel private(x, y)";
    let config = DebugConfig::openmp();

    let session = DebugSession::new(input, config).expect("Failed to create debug session");

    // Each step should have increasing step numbers
    for (i, step) in session.steps.iter().enumerate() {
        assert_eq!(
            step.step_number, i,
            "Step {} should have step_number {}, got {}",
            i, i, step.step_number
        );
    }

    // Steps should show progression through the input
    let first_step = &session.steps[0];
    assert!(
        first_step.position == 0,
        "First step should start at position 0"
    );
}

#[test]
fn test_ast_display() {
    use roup::debugger::display_ast_tree;

    let input = "#pragma omp parallel shared(x)";
    let config = DebugConfig::openmp();

    let session = DebugSession::new(input, config).expect("Failed to create debug session");

    let directive = session.final_directive.as_ref().unwrap();
    let ast_string = display_ast_tree(directive);

    // Should contain key elements
    assert!(ast_string.contains("Directive"));
    assert!(ast_string.contains("parallel"));
    assert!(ast_string.contains("shared"));
}

// ===========================================================================
// Tests for Custom Directive Parsers (ensure debugger works with all parser types)
// ===========================================================================

#[test]
fn test_custom_parser_scan_directive() {
    // scan has a custom parser with special parameter syntax
    let input = "#pragma omp scan exclusive(x, y)";
    let config = DebugConfig::openmp();

    let session = DebugSession::new(input, config).expect("Failed to create debug session");

    let directive = session.final_directive.as_ref().unwrap();
    assert_eq!(directive.name, "scan");
    assert!(directive.parameter.is_some());
    assert_eq!(directive.parameter.as_ref().unwrap(), "exclusive(x, y)");

    // Verify we captured a DirectiveParameter step
    assert!(session
        .steps
        .iter()
        .any(|s| matches!(s.kind, roup::debugger::StepKind::DirectiveParameter)));
}

#[test]
fn test_custom_parser_allocate_directive() {
    // allocate has a custom parser with both parameter and clauses
    let input = "#pragma omp allocate(arr) allocator(omp_default_mem_alloc)";
    let config = DebugConfig::openmp();

    let session = DebugSession::new(input, config).expect("Failed to create debug session");

    let directive = session.final_directive.as_ref().unwrap();
    assert_eq!(directive.name, "allocate");
    assert!(directive.parameter.is_some());
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "allocator");
}

#[test]
fn test_custom_parser_metadirective() {
    // metadirective has complex nested syntax
    let input = "#pragma omp metadirective when(device={kind(gpu)}:parallel) default(serial)";
    let config = DebugConfig::openmp();

    let session = DebugSession::new(input, config).expect("Failed to create debug session");

    let directive = session.final_directive.as_ref().unwrap();
    assert_eq!(directive.name, "metadirective");
    assert_eq!(directive.clauses.len(), 2);

    // Verify clauses have complex arguments
    let when_clause = &directive.clauses[0];
    assert_eq!(when_clause.name, "when");
    assert!(matches!(
        &when_clause.kind,
        roup::parser::ClauseKind::Parenthesized(_)
    ));
}

#[test]
fn test_custom_parser_cancel_directive() {
    // cancel has parameter-like syntax
    let input = "#pragma omp cancel parallel";
    let config = DebugConfig::openmp();

    let session = DebugSession::new(input, config).expect("Failed to create debug session");

    let directive = session.final_directive.as_ref().unwrap();
    assert_eq!(directive.name, "cancel");
}

// ===========================================================================
// Tests for Bare Clauses (clauses without arguments)
// ===========================================================================

#[test]
fn test_bare_clauses() {
    let input = "#pragma omp parallel nowait default(shared)";
    let config = DebugConfig::openmp();

    let session = DebugSession::new(input, config).expect("Failed to create debug session");

    let directive = session.final_directive.as_ref().unwrap();
    assert_eq!(directive.clauses.len(), 2);

    // Find the nowait clause (bare clause)
    let nowait = directive.clauses.iter().find(|c| c.name == "nowait");
    assert!(nowait.is_some());
    assert!(matches!(
        nowait.unwrap().kind,
        roup::parser::ClauseKind::Bare
    ));

    // Verify AST displays bare clauses correctly
    use roup::debugger::display_ast_tree;
    let ast = display_ast_tree(directive);
    assert!(ast.contains("Bare"));
}

// ===========================================================================
// Tests for Directive Parameters vs. Clauses
// ===========================================================================

#[test]
fn test_directive_parameter_vs_clause() {
    // Test that we distinguish directive parameters from clauses correctly
    let inputs = vec![
        ("#pragma omp scan exclusive(x)", true, 0), // parameter, no clauses
        ("#pragma omp parallel private(x)", false, 1), // no parameter, has clauses
        ("#pragma omp allocate(x) align(16)", true, 1), // both parameter and clauses
    ];

    for (input, has_param, clause_count) in inputs {
        let config = DebugConfig::openmp();
        let session = DebugSession::new(input, config)
            .unwrap_or_else(|_| panic!("Failed to parse: {}", input));

        let directive = session.final_directive.as_ref().unwrap();

        assert_eq!(
            directive.parameter.is_some(),
            has_param,
            "Parameter presence mismatch for: {}",
            input
        );

        assert_eq!(
            directive.clauses.len(),
            clause_count,
            "Clause count mismatch for: {}",
            input
        );
    }
}

// ===========================================================================
// Tests for Error Handling
// ===========================================================================

#[test]
fn test_invalid_input_produces_error() {
    let input = "#pragma omp INVALID_DIRECTIVE_NAME";
    let config = DebugConfig::openmp();

    let result = DebugSession::new(input, config);

    // Should fail gracefully
    assert!(result.is_err(), "Expected error for invalid directive");
}

// ===========================================================================
// Tests for Multiple Whitespace/Comments
// ===========================================================================

#[test]
fn test_multiple_whitespace_steps() {
    let input = "#pragma   omp   parallel    shared(x)";
    let config = DebugConfig::openmp();

    let session = DebugSession::new(input, config).expect("Failed to parse");

    // Should have whitespace-skipping steps
    // Note: The parser may consolidate multiple whitespaces in a single step
    let whitespace_steps: Vec<_> = session
        .steps
        .iter()
        .filter(|s| matches!(s.kind, roup::debugger::StepKind::SkipWhitespace))
        .collect();

    // Just verify we have at least one whitespace step
    assert!(
        !whitespace_steps.is_empty(),
        "Expected at least 1 whitespace step, got {}",
        whitespace_steps.len()
    );

    // More importantly, verify the directive was parsed correctly despite extra whitespace
    let directive = session.final_directive.as_ref().unwrap();
    assert_eq!(directive.name, "parallel");
    assert_eq!(directive.clauses.len(), 1);
}

// ===========================================================================
// Tests for OpenACC-Specific Features
// ===========================================================================

#[test]
fn test_openacc_data_directive() {
    let input = "#pragma acc data copy(a) copyin(b) copyout(c)";
    let config = DebugConfig::openacc();

    let session = DebugSession::new(input, config).expect("Failed to parse");

    let directive = session.final_directive.as_ref().unwrap();
    assert_eq!(directive.name, "data");
    assert_eq!(directive.clauses.len(), 3);
}

#[test]
fn test_openacc_kernels_directive() {
    let input = "#pragma acc kernels async(1) wait(2) num_gangs(4)";
    let config = DebugConfig::openacc();

    let session = DebugSession::new(input, config).expect("Failed to parse");

    let directive = session.final_directive.as_ref().unwrap();
    assert_eq!(directive.name, "kernels");
    assert_eq!(directive.clauses.len(), 3);
}

// ===========================================================================
// Tests for Fortran Syntax (ensure language detection works)
// ===========================================================================

#[test]
fn test_fortran_free_form() {
    use roup::lexer::Language;

    let input = "!$omp parallel shared(x)";
    let config = DebugConfig::new(roup::parser::Dialect::OpenMp, Language::FortranFree);

    let session = DebugSession::new(input, config).expect("Failed to parse Fortran free-form");
    let directive = session.final_directive.as_ref().unwrap();
    assert_eq!(directive.name, "parallel");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "shared");

    // Verify we captured Fortran sentinel parsing step
    assert!(session
        .steps
        .iter()
        .any(|s| s.description.contains("Fortran free-form sentinel")));

    // Verify the sentinel step consumed !$omp
    let sentinel_step = session
        .steps
        .iter()
        .find(|s| matches!(s.kind, roup::debugger::StepKind::PragmaPrefix))
        .unwrap();
    assert_eq!(sentinel_step.consumed, "!$omp");
}

#[test]
fn test_fortran_fixed_form() {
    use roup::lexer::Language;

    let input = "c$omp parallel private(y)";
    let config = DebugConfig::new(roup::parser::Dialect::OpenMp, Language::FortranFixed);

    let session = DebugSession::new(input, config).expect("Failed to parse Fortran fixed-form");
    let directive = session.final_directive.as_ref().unwrap();
    assert_eq!(directive.name, "parallel");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "private");

    // Verify we captured Fortran sentinel parsing step
    assert!(session
        .steps
        .iter()
        .any(|s| s.description.contains("Fortran fixed-form sentinel")));

    // Verify the sentinel step consumed c$omp
    let sentinel_step = session
        .steps
        .iter()
        .find(|s| matches!(s.kind, roup::debugger::StepKind::PragmaPrefix))
        .unwrap();
    assert_eq!(sentinel_step.consumed, "c$omp");
}

#[test]
fn test_openacc_fortran() {
    use roup::lexer::Language;

    let input = "!$acc parallel async(1)";
    let config = DebugConfig::new(roup::parser::Dialect::OpenAcc, Language::FortranFree);

    let session = DebugSession::new(input, config).expect("Failed to parse OpenACC Fortran");
    let directive = session.final_directive.as_ref().unwrap();
    assert_eq!(directive.name, "parallel");
    assert_eq!(directive.clauses.len(), 1);
    assert_eq!(directive.clauses[0].name, "async");

    // Verify we captured Fortran sentinel parsing step
    assert!(session
        .steps
        .iter()
        .any(|s| s.description.contains("Fortran free-form sentinel")));

    // Verify the sentinel step consumed !$acc
    let sentinel_step = session
        .steps
        .iter()
        .find(|s| matches!(s.kind, roup::debugger::StepKind::PragmaPrefix))
        .unwrap();
    assert_eq!(sentinel_step.consumed, "!$acc");
}

// ===========================================================================
// Tests for Complex/Real-World Directives
// ===========================================================================

#[test]
fn test_complex_target_teams_distribute() {
    let input = "#pragma omp target teams distribute parallel for simd \
                 map(to: a[0:N]) map(from: b[0:N]) \
                 num_teams(4) thread_limit(256) \
                 collapse(2) reduction(+:sum)";
    let config = DebugConfig::openmp();

    let session = DebugSession::new(input, config).expect("Failed to parse");

    let directive = session.final_directive.as_ref().unwrap();
    assert_eq!(directive.name, "target teams distribute parallel for simd");
    assert!(directive.clauses.len() >= 5);

    // Should have many steps for this complex directive
    assert!(session.total_steps() >= 10);
}

#[test]
fn test_reduction_with_operators() {
    let inputs = vec![
        "#pragma omp parallel reduction(+:sum)",
        "#pragma omp parallel reduction(-:diff)",
        "#pragma omp parallel reduction(*:prod)",
        "#pragma omp parallel reduction(max:maximum)",
    ];

    for input in inputs {
        let config = DebugConfig::openmp();
        let session = DebugSession::new(input, config)
            .unwrap_or_else(|_| panic!("Failed to parse: {}", input));

        let directive = session.final_directive.as_ref().unwrap();
        assert_eq!(directive.name, "parallel");

        let reduction_clause = directive.clauses.iter().find(|c| c.name == "reduction");
        assert!(
            reduction_clause.is_some(),
            "Missing reduction clause in: {}",
            input
        );
    }
}

// ===========================================================================
// Tests for Step Consistency (ensure steps match final directive)
// ===========================================================================

#[test]
fn test_steps_match_final_directive() {
    let input = "#pragma omp parallel private(x, y) shared(z)";
    let config = DebugConfig::openmp();

    let session = DebugSession::new(input, config).expect("Failed to parse");
    let directive = session.final_directive.as_ref().unwrap();

    // Count directive name steps
    let directive_name_steps = session
        .steps
        .iter()
        .filter(|s| matches!(s.kind, roup::debugger::StepKind::DirectiveName))
        .count();
    assert_eq!(directive_name_steps, 1);

    // Count clause name steps - should match number of clauses in final directive
    let clause_name_steps = session
        .steps
        .iter()
        .filter(|s| matches!(s.kind, roup::debugger::StepKind::ClauseName))
        .count();
    assert_eq!(
        clause_name_steps,
        directive.clauses.len(),
        "Clause step count mismatch"
    );
}

// ===========================================================================
// Test for Future-Proofing: Ensure no hardcoded directive names
// ===========================================================================

#[test]
fn test_no_hardcoded_directive_names() {
    // This test verifies that the debugger works generically
    // by testing directives that might be added in future OpenMP versions
    // If these fail, it's because the parser doesn't support them yet,
    // NOT because the debugger is broken

    let test_cases = vec![
        ("#pragma omp parallel", "parallel"),
        ("#pragma omp for", "for"),
        ("#pragma omp sections", "sections"),
        ("#pragma omp single", "single"),
        ("#pragma omp task", "task"),
        ("#pragma omp taskwait", "taskwait"),
        ("#pragma omp barrier", "barrier"),
    ];

    for (input, expected_name) in test_cases {
        let config = DebugConfig::openmp();
        let session = DebugSession::new(input, config)
            .unwrap_or_else(|_| panic!("Failed to parse: {}", input));

        let directive = session.final_directive.as_ref().unwrap();
        assert_eq!(
            directive.name, expected_name,
            "Directive name mismatch for: {}",
            input
        );

        // Verify we got steps
        assert!(session.total_steps() > 0);

        // Verify final step is Complete
        let last_step = session.steps.last().unwrap();
        assert!(matches!(last_step.kind, roup::debugger::StepKind::Complete));
    }
}
