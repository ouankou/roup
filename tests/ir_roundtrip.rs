//! Integration test for IR round-trip conversion
//!
//! This test demonstrates the complete pipeline:
//! String → Lexer → Parser → IR → Display → String
//!
//! ## Learning Objectives
//!
//! - **Integration testing**: Testing multiple components working together
//! - **Round-trip verification**: Input → Transform → Output → Verify
//! - **Semantic preservation**: Output doesn't need exact syntax, but same meaning
//! - **Display trait verification**: Using Display to convert IR back to strings
//!
//! ## What We're Testing
//!
//! 1. Parser can handle directive syntax
//! 2. Conversion layer correctly interprets parsed data
//! 3. IR Display produces valid pragmas
//! 4. Semantic information is preserved (not necessarily exact syntax)
//!
//! ## Example Flow
//!
//! ```text
//! Input:  "#pragma omp parallel default(shared) private(x, y)"
//!           ↓ Lexer
//! Tokens: ["pragma", "omp", "parallel", "default", "(", "shared", ")", ...]
//!           ↓ Parser
//! Parser: Directive { name: "parallel",
//!                     clauses: [Clause { name: "default", ... },
//!                               Clause { name: "private", ... }] }
//!           ↓ Conversion
//! IR:     DirectiveIR { kind: DirectiveKind::Parallel,
//!                       clauses: [ClauseData::Default(Shared),
//!                                 ClauseData::Private { items: [...] }] }
//!           ↓ Display
//! Output: "#pragma omp parallel default(shared) private(x, y)"
//! ```

use roup::ir::{convert::convert_directive, Language, ParserConfig, SourceLocation};
use roup::parser::parse_omp_directive;

/// Test round-trip for simple parallel directive
#[test]
fn roundtrip_parallel_simple() {
    let input = "#pragma omp parallel";

    // Parse
    let (remaining, directive) =
        parse_omp_directive(input).expect("Failed to parse parallel directive");
    assert!(remaining.is_empty(), "Should consume entire input");

    // Convert to IR
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    // Verify IR structure
    assert!(ir.kind().is_parallel());
    assert_eq!(ir.clauses().len(), 0);

    // Convert back to string via Display
    let output = ir.to_string();
    assert_eq!(output, "#pragma omp parallel");
}

/// Test round-trip with default clause
#[test]
fn roundtrip_parallel_with_default() {
    let input = "#pragma omp parallel default(shared)";

    // Parse
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");

    // Convert to IR
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    // Verify
    assert!(ir.kind().is_parallel());
    assert_eq!(ir.clauses().len(), 1);
    assert!(ir.has_clause(|c| c.is_default()));

    // Display
    let output = ir.to_string();
    assert_eq!(output, "#pragma omp parallel default(shared)");
}

/// Test round-trip with multiple clauses
#[test]
fn roundtrip_parallel_with_multiple_clauses() {
    let input = "#pragma omp parallel default(shared) private(x, y)";

    // Parse
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");

    // Convert to IR
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    // Verify IR structure
    assert!(ir.kind().is_parallel());
    assert_eq!(ir.clauses().len(), 2);
    assert!(ir.has_clause(|c| c.is_default()));
    assert!(ir.has_clause(|c| c.is_private()));

    // Display - note: clause order is preserved
    let output = ir.to_string();
    assert_eq!(output, "#pragma omp parallel default(shared) private(x, y)");
}

/// Test round-trip for parallel for directive
#[test]
fn roundtrip_parallel_for() {
    let input = "#pragma omp parallel for";

    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "parallel for");
    assert!(ir.kind().is_parallel());
    assert!(ir.kind().is_loop()); // Combined parallel + loop construct

    let output = ir.to_string();
    assert_eq!(output, "#pragma omp parallel for");
}

/// Test round-trip with if clause
#[test]
fn roundtrip_parallel_with_if() {
    let input = "#pragma omp parallel if(n > 100)";

    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert!(ir.has_clause(|c| c.is_if()));

    let output = ir.to_string();
    assert_eq!(output, "#pragma omp parallel if(n > 100)");
}

/// Test round-trip with num_threads clause
#[test]
fn roundtrip_parallel_with_num_threads() {
    let input = "#pragma omp parallel num_threads(4)";

    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert!(ir.has_clause(|c| c.is_num_threads()));

    let output = ir.to_string();
    assert_eq!(output, "#pragma omp parallel num_threads(4)");
}

/// Test round-trip for target directive
#[test]
fn roundtrip_target() {
    let input = "#pragma omp target";

    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    let output = ir.to_string();
    assert_eq!(output, "#pragma omp target");
}

/// Test round-trip for teams directive
#[test]
fn roundtrip_teams() {
    let input = "#pragma omp teams";

    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_teams());
    let output = ir.to_string();
    assert_eq!(output, "#pragma omp teams");
}

/// Test round-trip for combined target teams distribute
#[test]
fn roundtrip_target_teams_distribute() {
    let input = "#pragma omp target teams distribute";

    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_teams());
    let output = ir.to_string();
    assert_eq!(output, "#pragma omp target teams distribute");
}

/// Test round-trip for task with clauses
#[test]
fn roundtrip_task_with_clauses() {
    let input = "#pragma omp task private(x) shared(y)";

    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_task());
    assert_eq!(ir.clauses().len(), 2);
    let output = ir.to_string();
    assert_eq!(output, "#pragma omp task private(x) shared(y)");
}

/// Test round-trip preserves firstprivate
#[test]
fn roundtrip_firstprivate() {
    let input = "#pragma omp parallel firstprivate(a, b, c)";

    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.has_clause(|c| c.is_firstprivate()));
    let output = ir.to_string();
    assert_eq!(output, "#pragma omp parallel firstprivate(a, b, c)");
}

/// Test round-trip with bare clause
#[test]
fn roundtrip_single_with_nowait() {
    let input = "#pragma omp single nowait";

    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_worksharing());
    assert!(ir.has_clause(|c| matches!(c, roup::ir::ClauseData::Bare(_))));
    let output = ir.to_string();
    assert_eq!(output, "#pragma omp single nowait");
}

/// Test round-trip for Fortran syntax (when parser supports it)
#[test]
#[ignore = "Parser doesn't yet support Fortran comment syntax"]
fn roundtrip_fortran_syntax() {
    let input = "!$omp parallel default(shared)";

    let (_, directive) = parse_omp_directive(input).expect("Failed to parse Fortran");
    let config = ParserConfig::default();
    let ir = convert_directive(
        &directive,
        SourceLocation::start(),
        Language::Fortran,
        &config,
    )
    .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert_eq!(ir.language(), Language::Fortran);

    let output = ir.to_string();
    // Should use Fortran comment syntax
    assert_eq!(output, "!$omp parallel default(shared)");
}

/// Test round-trip with collapse
#[test]
fn roundtrip_for_with_collapse() {
    let input = "#pragma omp for collapse(2)";

    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_worksharing());
    assert!(ir.has_clause(|c| c.is_collapse()));
    let output = ir.to_string();
    assert_eq!(output, "#pragma omp for collapse(2)");
}

/// Test round-trip with ordered
#[test]
fn roundtrip_for_with_ordered() {
    let input = "#pragma omp for ordered";

    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.has_clause(|c| c.is_ordered()));
    let output = ir.to_string();
    assert_eq!(output, "#pragma omp for ordered");
}

/// Test that semantic information is preserved even if syntax differs slightly
///
/// This tests the key idea: we care about *meaning*, not exact syntax.
#[test]
fn roundtrip_semantic_preservation() {
    // Input with extra whitespace
    let input = "#pragma omp parallel  default ( shared )  private( x,  y )";

    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    // IR should normalize the representation
    let output = ir.to_string();

    // Exact whitespace might differ, but semantic content is the same
    assert!(output.contains("parallel"));
    assert!(output.contains("default(shared)"));
    assert!(output.contains("private(x, y)"));

    // We can re-parse the output and get the same IR
    let (_, directive2) = parse_omp_directive(&output).expect("Failed to re-parse");
    let ir2 = convert_directive(&directive2, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert re-parsed IR");

    // The two IRs should be equal
    assert_eq!(ir.kind(), ir2.kind());
    assert_eq!(ir.clauses().len(), ir2.clauses().len());
}
