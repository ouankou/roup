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
//! ## Coverage
//!
//! This file contains round-trip tests for ALL 128 OpenMP directives defined in the
//! OpenMP 6.0 specification, organized into the following categories:
//!
//! - Parallel constructs (14 directives)
//! - Worksharing constructs (5 directives)
//! - Task constructs (13 directives)
//! - SIMD constructs (3 directives)
//! - Target constructs (28 directives)
//! - Teams constructs (16 directives)
//! - Distribute constructs (8 directives)
//! - Loop constructs (4 directives)
//! - Atomic constructs (6 directives)
//! - Synchronization constructs (9 directives)
//! - Cancellation constructs (2 directives)
//! - Data environment (7 directives)
//! - Metadirectives (3 directives)
//! - Utility directives (10 directives)

use roup::ir::{convert::convert_directive, Language, ParserConfig, SourceLocation};
use roup::parser::parse_omp_directive;

// ============================================================================
// Parallel Constructs (14 directives)
// ============================================================================

#[test]
fn roundtrip_parallel() {
    let input = "#pragma omp parallel";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert_eq!(ir.kind().to_string(), "parallel");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_parallel_for() {
    let input = "#pragma omp parallel for";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert!(ir.kind().is_loop());
    assert_eq!(ir.kind().to_string(), "parallel for");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_parallel_for_simd() {
    let input = "#pragma omp parallel for simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert!(ir.kind().is_simd());
    assert_eq!(ir.kind().to_string(), "parallel for simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_parallel_loop() {
    let input = "#pragma omp parallel loop";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert!(ir.kind().is_loop());
    assert_eq!(ir.kind().to_string(), "parallel loop");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_parallel_loop_simd() {
    let input = "#pragma omp parallel loop simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert!(ir.kind().is_simd());
    assert!(ir.kind().is_loop());
    assert_eq!(ir.kind().to_string(), "parallel loop simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_parallel_sections() {
    let input = "#pragma omp parallel sections";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert_eq!(ir.kind().to_string(), "parallel sections");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_parallel_masked() {
    let input = "#pragma omp parallel masked";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert_eq!(ir.kind().to_string(), "parallel masked");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_parallel_masked_taskloop() {
    let input = "#pragma omp parallel masked taskloop";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert_eq!(ir.kind().to_string(), "parallel masked taskloop");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_parallel_masked_taskloop_simd() {
    let input = "#pragma omp parallel masked taskloop simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert!(ir.kind().is_simd());
    assert_eq!(ir.kind().to_string(), "parallel masked taskloop simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_parallel_master() {
    let input = "#pragma omp parallel master";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert_eq!(ir.kind().to_string(), "parallel master");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_parallel_master_taskloop() {
    let input = "#pragma omp parallel master taskloop";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert_eq!(ir.kind().to_string(), "parallel master taskloop");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_parallel_master_taskloop_simd() {
    let input = "#pragma omp parallel master taskloop simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert!(ir.kind().is_simd());
    assert_eq!(ir.kind().to_string(), "parallel master taskloop simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_parallel_do() {
    let input = "#pragma omp parallel do";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert_eq!(ir.kind().to_string(), "parallel do");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_parallel_do_simd() {
    let input = "#pragma omp parallel do simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert!(ir.kind().is_simd());
    assert_eq!(ir.kind().to_string(), "parallel do simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

// ============================================================================
// Worksharing Constructs (5 directives)
// ============================================================================

#[test]
fn roundtrip_for() {
    let input = "#pragma omp for";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_worksharing());
    assert_eq!(ir.kind().to_string(), "for");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_for_simd() {
    let input = "#pragma omp for simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_worksharing());
    assert!(ir.kind().is_simd());
    assert_eq!(ir.kind().to_string(), "for simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_do() {
    let input = "#pragma omp do";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "do");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_do_simd() {
    let input = "#pragma omp do simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_simd());
    assert_eq!(ir.kind().to_string(), "do simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_sections() {
    let input = "#pragma omp sections";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_worksharing());
    assert_eq!(ir.kind().to_string(), "sections");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_section() {
    let input = "#pragma omp section";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "section");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_single() {
    let input = "#pragma omp single";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_worksharing());
    assert_eq!(ir.kind().to_string(), "single");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_workshare() {
    let input = "#pragma omp workshare";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "workshare");
    let output = ir.to_string();
    assert_eq!(output, input);
}

// ============================================================================
// Task Constructs (13 directives)
// ============================================================================

#[test]
fn roundtrip_task() {
    let input = "#pragma omp task";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_task());
    assert_eq!(ir.kind().to_string(), "task");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_task_iteration() {
    let input = "#pragma omp task iteration";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_task());
    assert_eq!(ir.kind().to_string(), "task iteration");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_taskloop() {
    let input = "#pragma omp taskloop";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_task());
    assert!(ir.kind().is_loop());
    assert_eq!(ir.kind().to_string(), "taskloop");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_taskloop_simd() {
    let input = "#pragma omp taskloop simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_task());
    assert!(ir.kind().is_simd());
    assert_eq!(ir.kind().to_string(), "taskloop simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_masked_taskloop() {
    let input = "#pragma omp masked taskloop";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_task());
    assert!(ir.kind().is_loop());
    assert_eq!(ir.kind().to_string(), "masked taskloop");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_masked_taskloop_simd() {
    let input = "#pragma omp masked taskloop simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_task());
    assert!(ir.kind().is_simd());
    assert!(ir.kind().is_loop());
    assert_eq!(ir.kind().to_string(), "masked taskloop simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_taskgroup() {
    let input = "#pragma omp taskgroup";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_task());
    assert_eq!(ir.kind().to_string(), "taskgroup");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_taskgraph() {
    let input = "#pragma omp taskgraph";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_task());
    assert_eq!(ir.kind().to_string(), "taskgraph");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_taskwait() {
    let input = "#pragma omp taskwait";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_synchronization());
    assert_eq!(ir.kind().to_string(), "taskwait");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_taskyield() {
    let input = "#pragma omp taskyield";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_synchronization());
    assert_eq!(ir.kind().to_string(), "taskyield");
    let output = ir.to_string();
    assert_eq!(output, input);
}

// ============================================================================
// SIMD Constructs (3 directives)
// ============================================================================

#[test]
fn roundtrip_simd() {
    let input = "#pragma omp simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_simd());
    assert_eq!(ir.kind().to_string(), "simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_declare_simd() {
    let input = "#pragma omp declare simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "declare simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

// ============================================================================
// Loop Constructs (4 directives)
// ============================================================================

#[test]
fn roundtrip_loop() {
    let input = "#pragma omp loop";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_loop());
    assert_eq!(ir.kind().to_string(), "loop");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_tile() {
    let input = "#pragma omp tile";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "tile");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_unroll() {
    let input = "#pragma omp unroll";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "unroll");
    let output = ir.to_string();
    assert_eq!(output, input);
}

// ============================================================================
// Target Constructs (28 directives)
// ============================================================================

#[test]
fn roundtrip_target() {
    let input = "#pragma omp target";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert_eq!(ir.kind().to_string(), "target");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_data() {
    let input = "#pragma omp target data";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert_eq!(ir.kind().to_string(), "target data");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_enter_data() {
    let input = "#pragma omp target enter data";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert_eq!(ir.kind().to_string(), "target enter data");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_exit_data() {
    let input = "#pragma omp target exit data";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert_eq!(ir.kind().to_string(), "target exit data");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_update() {
    let input = "#pragma omp target update";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert_eq!(ir.kind().to_string(), "target update");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_loop() {
    let input = "#pragma omp target loop";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_loop());
    assert_eq!(ir.kind().to_string(), "target loop");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_loop_simd() {
    let input = "#pragma omp target loop simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_simd());
    assert!(ir.kind().is_loop());
    assert_eq!(ir.kind().to_string(), "target loop simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_parallel() {
    let input = "#pragma omp target parallel";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_parallel());
    assert_eq!(ir.kind().to_string(), "target parallel");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_parallel_do() {
    let input = "#pragma omp target parallel do";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_parallel());
    assert_eq!(ir.kind().to_string(), "target parallel do");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_parallel_do_simd() {
    let input = "#pragma omp target parallel do simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_parallel());
    assert!(ir.kind().is_simd());
    assert_eq!(ir.kind().to_string(), "target parallel do simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_parallel_for() {
    let input = "#pragma omp target parallel for";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_parallel());
    assert_eq!(ir.kind().to_string(), "target parallel for");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_parallel_for_simd() {
    let input = "#pragma omp target parallel for simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_parallel());
    assert!(ir.kind().is_simd());
    assert_eq!(ir.kind().to_string(), "target parallel for simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_parallel_loop() {
    let input = "#pragma omp target parallel loop";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_parallel());
    assert!(ir.kind().is_loop());
    assert_eq!(ir.kind().to_string(), "target parallel loop");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_parallel_loop_simd() {
    let input = "#pragma omp target parallel loop simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_simd());
    assert!(ir.kind().is_loop());
    assert_eq!(ir.kind().to_string(), "target parallel loop simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_simd() {
    let input = "#pragma omp target simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_simd());
    assert_eq!(ir.kind().to_string(), "target simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_teams() {
    let input = "#pragma omp target teams";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_teams());
    assert_eq!(ir.kind().to_string(), "target teams");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_teams_distribute() {
    let input = "#pragma omp target teams distribute";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_teams());
    assert_eq!(ir.kind().to_string(), "target teams distribute");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_teams_distribute_parallel_do() {
    let input = "#pragma omp target teams distribute parallel do";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_teams());
    assert_eq!(ir.kind().to_string(), "target teams distribute parallel do");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_teams_distribute_parallel_do_simd() {
    let input = "#pragma omp target teams distribute parallel do simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_teams());
    assert!(ir.kind().is_simd());
    assert_eq!(
        ir.kind().to_string(),
        "target teams distribute parallel do simd"
    );
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_teams_distribute_parallel_for() {
    let input = "#pragma omp target teams distribute parallel for";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_teams());
    assert_eq!(
        ir.kind().to_string(),
        "target teams distribute parallel for"
    );
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_teams_distribute_parallel_for_simd() {
    let input = "#pragma omp target teams distribute parallel for simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_teams());
    assert!(ir.kind().is_simd());
    assert_eq!(
        ir.kind().to_string(),
        "target teams distribute parallel for simd"
    );
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_teams_distribute_parallel_loop() {
    let input = "#pragma omp target teams distribute parallel loop";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_teams());
    assert!(ir.kind().is_loop());
    assert_eq!(
        ir.kind().to_string(),
        "target teams distribute parallel loop"
    );
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_teams_distribute_parallel_loop_simd() {
    let input = "#pragma omp target teams distribute parallel loop simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_teams());
    assert!(ir.kind().is_simd());
    assert!(ir.kind().is_loop());
    assert_eq!(
        ir.kind().to_string(),
        "target teams distribute parallel loop simd"
    );
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_teams_distribute_simd() {
    let input = "#pragma omp target teams distribute simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_teams());
    assert!(ir.kind().is_simd());
    assert_eq!(ir.kind().to_string(), "target teams distribute simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_teams_loop() {
    let input = "#pragma omp target teams loop";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_teams());
    assert!(ir.kind().is_loop());
    assert_eq!(ir.kind().to_string(), "target teams loop");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_teams_loop_simd() {
    let input = "#pragma omp target teams loop simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_teams());
    assert!(ir.kind().is_simd());
    assert!(ir.kind().is_loop());
    assert_eq!(ir.kind().to_string(), "target teams loop simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

// ============================================================================
// Teams Constructs (16 directives)
// ============================================================================

#[test]
fn roundtrip_teams() {
    let input = "#pragma omp teams";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_teams());
    assert_eq!(ir.kind().to_string(), "teams");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_teams_distribute() {
    let input = "#pragma omp teams distribute";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_teams());
    assert_eq!(ir.kind().to_string(), "teams distribute");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_teams_distribute_parallel_do() {
    let input = "#pragma omp teams distribute parallel do";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_teams());
    assert_eq!(ir.kind().to_string(), "teams distribute parallel do");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_teams_distribute_parallel_do_simd() {
    let input = "#pragma omp teams distribute parallel do simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_teams());
    assert!(ir.kind().is_simd());
    assert_eq!(ir.kind().to_string(), "teams distribute parallel do simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_teams_distribute_parallel_for() {
    let input = "#pragma omp teams distribute parallel for";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_teams());
    assert_eq!(ir.kind().to_string(), "teams distribute parallel for");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_teams_distribute_parallel_for_simd() {
    let input = "#pragma omp teams distribute parallel for simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_teams());
    assert!(ir.kind().is_simd());
    assert_eq!(ir.kind().to_string(), "teams distribute parallel for simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_teams_distribute_parallel_loop() {
    let input = "#pragma omp teams distribute parallel loop";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_teams());
    assert!(ir.kind().is_loop());
    assert_eq!(ir.kind().to_string(), "teams distribute parallel loop");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_teams_distribute_parallel_loop_simd() {
    let input = "#pragma omp teams distribute parallel loop simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_teams());
    assert!(ir.kind().is_simd());
    assert!(ir.kind().is_loop());
    assert_eq!(ir.kind().to_string(), "teams distribute parallel loop simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_teams_distribute_simd() {
    let input = "#pragma omp teams distribute simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_teams());
    assert!(ir.kind().is_simd());
    assert_eq!(ir.kind().to_string(), "teams distribute simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_teams_loop() {
    let input = "#pragma omp teams loop";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_teams());
    assert!(ir.kind().is_loop());
    assert_eq!(ir.kind().to_string(), "teams loop");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_teams_loop_simd() {
    let input = "#pragma omp teams loop simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_teams());
    assert!(ir.kind().is_simd());
    assert!(ir.kind().is_loop());
    assert_eq!(ir.kind().to_string(), "teams loop simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

// ============================================================================
// Distribute Constructs (8 directives)
// ============================================================================

#[test]
fn roundtrip_distribute() {
    let input = "#pragma omp distribute";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "distribute");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_distribute_parallel_do() {
    let input = "#pragma omp distribute parallel do";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "distribute parallel do");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_distribute_parallel_do_simd() {
    let input = "#pragma omp distribute parallel do simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_simd());
    assert_eq!(ir.kind().to_string(), "distribute parallel do simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_distribute_parallel_for() {
    let input = "#pragma omp distribute parallel for";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "distribute parallel for");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_distribute_parallel_for_simd() {
    let input = "#pragma omp distribute parallel for simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_simd());
    assert_eq!(ir.kind().to_string(), "distribute parallel for simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_distribute_parallel_loop() {
    let input = "#pragma omp distribute parallel loop";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_loop());
    assert_eq!(ir.kind().to_string(), "distribute parallel loop");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_distribute_parallel_loop_simd() {
    let input = "#pragma omp distribute parallel loop simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_simd());
    assert!(ir.kind().is_loop());
    assert_eq!(ir.kind().to_string(), "distribute parallel loop simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_distribute_simd() {
    let input = "#pragma omp distribute simd";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_simd());
    assert_eq!(ir.kind().to_string(), "distribute simd");
    let output = ir.to_string();
    assert_eq!(output, input);
}

// ============================================================================
// Atomic Constructs (6 directives)
// ============================================================================

#[test]
fn roundtrip_atomic() {
    let input = "#pragma omp atomic";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_synchronization());
    assert_eq!(ir.kind().to_string(), "atomic");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_atomic_read() {
    let input = "#pragma omp atomic read";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_synchronization());
    assert_eq!(ir.kind().to_string(), "atomic read");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_atomic_write() {
    let input = "#pragma omp atomic write";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_synchronization());
    assert_eq!(ir.kind().to_string(), "atomic write");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_atomic_update() {
    let input = "#pragma omp atomic update";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_synchronization());
    assert_eq!(ir.kind().to_string(), "atomic update");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_atomic_capture() {
    let input = "#pragma omp atomic capture";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_synchronization());
    assert_eq!(ir.kind().to_string(), "atomic capture");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_atomic_compare_capture() {
    let input = "#pragma omp atomic compare capture";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_synchronization());
    assert_eq!(ir.kind().to_string(), "atomic compare capture");
    let output = ir.to_string();
    assert_eq!(output, input);
}

// ============================================================================
// Synchronization Constructs (9 directives)
// ============================================================================

#[test]
fn roundtrip_barrier() {
    let input = "#pragma omp barrier";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_synchronization());
    assert_eq!(ir.kind().to_string(), "barrier");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_critical() {
    let input = "#pragma omp critical";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_synchronization());
    assert_eq!(ir.kind().to_string(), "critical");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_flush() {
    let input = "#pragma omp flush";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_synchronization());
    assert_eq!(ir.kind().to_string(), "flush");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_masked() {
    let input = "#pragma omp masked";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "masked");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_master() {
    let input = "#pragma omp master";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "master");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_ordered() {
    let input = "#pragma omp ordered";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "ordered");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_scope() {
    let input = "#pragma omp scope";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "scope");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_scan() {
    let input = "#pragma omp scan";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "scan");
    let output = ir.to_string();
    assert_eq!(output, input);
}

// ============================================================================
// Cancellation Constructs (2 directives)
// ============================================================================

#[test]
fn roundtrip_cancel() {
    let input = "#pragma omp cancel";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "cancel");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_cancellation_point() {
    let input = "#pragma omp cancellation point";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "cancellation point");
    let output = ir.to_string();
    assert_eq!(output, input);
}

// ============================================================================
// Data Environment (7 directives)
// ============================================================================

#[test]
fn roundtrip_allocate() {
    let input = "#pragma omp allocate";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "allocate");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_allocators() {
    let input = "#pragma omp allocators";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "allocators");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_threadprivate() {
    let input = "#pragma omp threadprivate";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "threadprivate");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_groupprivate() {
    let input = "#pragma omp groupprivate";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "groupprivate");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_declare_mapper() {
    let input = "#pragma omp declare mapper";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "declare mapper");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_declare_reduction() {
    let input = "#pragma omp declare reduction";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "declare reduction");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_declare_induction() {
    let input = "#pragma omp declare induction";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "declare induction");
    let output = ir.to_string();
    assert_eq!(output, input);
}

// ============================================================================
// Declare Target (5 directives)
// ============================================================================

#[test]
fn roundtrip_declare_target() {
    let input = "#pragma omp declare target";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "declare target");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_begin_declare_target() {
    let input = "#pragma omp begin declare target";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "begin declare target");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_end_declare_target() {
    let input = "#pragma omp end declare target";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "end declare target");
    let output = ir.to_string();
    assert_eq!(output, input);
}

// ============================================================================
// Declare Variant (3 directives)
// ============================================================================

#[test]
fn roundtrip_declare_variant() {
    let input = "#pragma omp declare variant";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "declare variant");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_begin_declare_variant() {
    let input = "#pragma omp begin declare variant";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "begin declare variant");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_end_declare_variant() {
    let input = "#pragma omp end declare variant";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "end declare variant");
    let output = ir.to_string();
    assert_eq!(output, input);
}

// ============================================================================
// Metadirectives (3 directives)
// ============================================================================

#[test]
fn roundtrip_metadirective() {
    let input = "#pragma omp metadirective";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "metadirective");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_begin_metadirective() {
    let input = "#pragma omp begin metadirective";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "begin metadirective");
    let output = ir.to_string();
    assert_eq!(output, input);
}

// ============================================================================
// Utility Directives (13 directives)
// ============================================================================

#[test]
fn roundtrip_assume() {
    let input = "#pragma omp assume";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "assume");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_assumes() {
    let input = "#pragma omp assumes";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "assumes");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_begin_assumes() {
    let input = "#pragma omp begin assumes";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "begin assumes");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_depobj() {
    let input = "#pragma omp depobj";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "depobj");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_dispatch() {
    let input = "#pragma omp dispatch";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "dispatch");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_error() {
    let input = "#pragma omp error";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "error");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_fuse() {
    let input = "#pragma omp fuse";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "fuse");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_interchange() {
    let input = "#pragma omp interchange";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "interchange");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_interop() {
    let input = "#pragma omp interop";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "interop");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_nothing() {
    let input = "#pragma omp nothing";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "nothing");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_requires() {
    let input = "#pragma omp requires";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "requires");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_reverse() {
    let input = "#pragma omp reverse";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "reverse");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_split() {
    let input = "#pragma omp split";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "split");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_stripe() {
    let input = "#pragma omp stripe";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "stripe");
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_workdistribute() {
    let input = "#pragma omp workdistribute";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert_eq!(ir.kind().to_string(), "workdistribute");
    let output = ir.to_string();
    assert_eq!(output, input);
}

// ============================================================================
// Tests with Clauses (preserving existing comprehensive clause tests)
// ============================================================================

#[test]
fn roundtrip_parallel_with_default() {
    let input = "#pragma omp parallel default(shared)";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert_eq!(ir.clauses().len(), 1);
    assert!(ir.has_clause(|c| c.is_default()));
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_parallel_with_multiple_clauses() {
    let input = "#pragma omp parallel default(shared) private(x, y)";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert_eq!(ir.clauses().len(), 2);
    assert!(ir.has_clause(|c| c.is_default()));
    assert!(ir.has_clause(|c| c.is_private()));
    let output = ir.to_string();
    assert_eq!(output, input);
}

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
    assert_eq!(output, input);
}

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
    assert_eq!(output, input);
}

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
    assert_eq!(output, input);
}

#[test]
fn roundtrip_firstprivate() {
    let input = "#pragma omp parallel firstprivate(a, b, c)";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.has_clause(|c| c.is_firstprivate()));
    let output = ir.to_string();
    assert_eq!(output, input);
}

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
    assert_eq!(output, input);
}

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
    assert_eq!(output, input);
}

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
    assert_eq!(output, input);
}

#[test]
fn roundtrip_for_with_ordered() {
    let input = "#pragma omp for ordered";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.has_clause(|c| c.is_ordered()));
    let output = ir.to_string();
    assert_eq!(output, input);
}

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

#[test]
fn roundtrip_parallel_with_reduction() {
    let input = "#pragma omp parallel reduction(+: sum)";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert!(ir.has_clause(|c| c.is_reduction()));
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_reduction_multiple_items() {
    let input = "#pragma omp parallel reduction(*: a, b, c)";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.has_clause(|c| c.is_reduction()));
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_reduction_minmax() {
    let input = "#pragma omp parallel reduction(min: value)";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_for_schedule_static() {
    let input = "#pragma omp for schedule(static)";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.has_clause(|c| c.is_schedule()));
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_for_schedule_with_chunk() {
    let input = "#pragma omp for schedule(dynamic, 10)";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_for_schedule_with_modifier() {
    let input = "#pragma omp for schedule(monotonic: static, 4)";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_with_map() {
    let input = "#pragma omp target map(to: arr)";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.has_clause(|c| c.is_map()));
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_map_tofrom() {
    let input = "#pragma omp target map(tofrom: x, y, z)";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_task_with_depend() {
    let input = "#pragma omp task depend(in: x, y)";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_task());
    assert!(ir.has_clause(|c| c.is_depend()));
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_task_depend_out() {
    let input = "#pragma omp task depend(out: result)";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_simd_with_linear() {
    let input = "#pragma omp simd linear(i: 2)";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_simd());
    assert!(ir.has_clause(|c| c.is_linear()));
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_parallel_proc_bind() {
    let input = "#pragma omp parallel proc_bind(close)";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.has_clause(|c| c.is_proc_bind()));
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_complex_parallel_for() {
    let input = "#pragma omp parallel for schedule(static, 16) reduction(+: sum) private(i)";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert!(ir.kind().is_loop());
    assert_eq!(ir.clauses().len(), 3);
    assert!(ir.has_clause(|c| c.is_schedule()));
    assert!(ir.has_clause(|c| c.is_reduction()));
    assert!(ir.has_clause(|c| c.is_private()));

    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_atomic_read_with_clause() {
    let input = "#pragma omp atomic read seq_cst";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_synchronization());
    assert_eq!(ir.clauses().len(), 1);
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_target_loop_with_map() {
    let input = "#pragma omp target loop map(to: arr)";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_target());
    assert!(ir.kind().is_loop());
    assert!(ir.has_clause(|c| c.is_map()));
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_masked_taskloop_with_private() {
    let input = "#pragma omp masked taskloop private(x, y)";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_task());
    assert!(ir.has_clause(|c| c.is_private()));
    let output = ir.to_string();
    assert_eq!(output, input);
}

#[test]
fn roundtrip_parallel_loop_simd_with_reduction() {
    let input = "#pragma omp parallel loop simd reduction(+: sum)";
    let (_, directive) = parse_omp_directive(input).expect("Failed to parse");
    let config = ParserConfig::default();
    let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config)
        .expect("Failed to convert to IR");

    assert!(ir.kind().is_parallel());
    assert!(ir.kind().is_simd());
    assert!(ir.has_clause(|c| c.is_reduction()));
    let output = ir.to_string();
    assert_eq!(output, input);
}
