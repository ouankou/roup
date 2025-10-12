//! Build script to generate roup_constants.h from source.
//!
//! **Usage:**
//! - `cargo build` - Generate header during build
//! - `cargo run --bin gen` - Verify header is up-to-date (CI mode)
//!
//! **Source of truth:** `src/c_api.rs` (directive_name_to_kind, convert_clause)
//!
//! For design rationale, maintenance instructions, and dual-purpose pattern explanation:
//! See [`docs/BUILD_SCRIPT_RATIONALE.md`](../docs/BUILD_SCRIPT_RATIONALE.md)

use std::env;
use std::fs;
use std::path::Path;
use std::process;

// Use #[path] to share code between build.rs and main crate
// This pattern is documented in docs/BUILD_SCRIPT_RATIONALE.md (see section "Dual-purpose #[path] pattern" for rationale and ecosystem examples).
// Rationale: build.rs runs in a separate environment (can't access crate modules), so we use
// #[path] to include constants_gen for both build-time generation and runtime verification.
// Alternative (separate build-utils crate) rejected: adds workspace complexity for several hundred lines.
#[path = "src/constants_gen.rs"]
mod constants_gen;

use constants_gen::*;

/// Verify mode: Check if committed header matches source
fn verify_header() -> ! {
    println!("Verifying src/roup_constants.h is up-to-date...");

    // Step 1: Check header exists
    if !Path::new("src/roup_constants.h").exists() {
        eprintln!("❌ ERROR: src/roup_constants.h not found");
        eprintln!("   Run 'cargo build' to generate the header");
        process::exit(1);
    }

    // Step 2: Calculate expected checksum from source (using syn)
    let directives = parse_directive_mappings();
    let clauses = parse_clause_mappings();
    let expected_checksum = calculate_checksum(&directives, &clauses);

    println!(
        "Calculated from source: {} (directives: {}, clauses: {})",
        expected_checksum,
        directives.len(),
        clauses.len()
    );

    // Step 3: Read checksum from committed header
    let header_content =
        fs::read_to_string("src/roup_constants.h").expect("Failed to read header file");

    let header_checksum = extract_checksum_from_header(&header_content);

    match header_checksum {
        Some(checksum) => {
            println!("Committed header:       {}", checksum);

            if checksum == expected_checksum {
                println!("✓ Header is up-to-date with source code");
                process::exit(0);
            } else {
                eprintln!("");
                eprintln!("❌ ERROR: Header is out of date!");
                eprintln!("   Expected: {}", expected_checksum);
                eprintln!("   Found:    {}", checksum);
                eprintln!("");
                eprintln!("   The committed src/roup_constants.h doesn't match src/c_api.rs");
                eprintln!("   To fix: Run 'cargo build' locally and commit the updated header");
                process::exit(1);
            }
        }
        None => {
            eprintln!("❌ ERROR: Could not find ROUP_CONSTANTS_CHECKSUM in header");
            process::exit(1);
        }
    }
}

/// Build mode: Generate header
fn generate_mode() {
    // Parse constants from source
    let directives = parse_directive_mappings();
    let clauses = parse_clause_mappings();

    // Get the output directory
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("roup_constants.h");

    // Generate the header
    let header = generate_header(&directives, &clauses);

    // Write to OUT_DIR
    fs::write(&dest_path, &header).expect("Failed to write to OUT_DIR");

    // Also copy to src/ for easier access during development
    let src_dest = Path::new("src").join("roup_constants.h");
    fs::write(&src_dest, &header).expect("Failed to write to src/");

    // Validate checksum by reading back the generated file (Case 1 validation)
    let generated_content =
        fs::read_to_string(&src_dest).expect("Failed to read generated header for validation");

    let extracted_checksum = extract_checksum_from_header(&generated_content);
    let expected_checksum = calculate_checksum(&directives, &clauses);

    match extracted_checksum {
        Some(checksum) => {
            assert_eq!(
                checksum,
                expected_checksum,
                "FATAL: Constants checksum mismatch!\n\
                 Expected: 0x{:08X} (FNV-1a hash of {} directives + {} clauses)\n\
                 Found in header: 0x{:08X}\n\
                 The generated header is out of sync with c_api.rs.\n\
                 This should never happen - build.rs generates both values.\n\
                 Please file a bug report.",
                expected_checksum,
                directives.len(),
                clauses.len(),
                checksum
            );
        }
        None => {
            panic!("FATAL: Could not find ROUP_CONSTANTS_CHECKSUM in generated header");
        }
    }

    println!("cargo:rerun-if-changed=src/c_api.rs");
    println!("cargo:rerun-if-changed=src/constants_gen.rs");
    println!("cargo:rerun-if-changed=build.rs");
}

fn main() {
    // Detect mode: build script vs standalone binary
    // When cargo runs build.rs during compilation, OUT_DIR is set
    // When run as `cargo run --bin gen`, OUT_DIR is not set

    if env::var("OUT_DIR").is_ok() {
        // Build mode: Generate header during cargo build
        generate_mode();
    } else {
        // Standalone mode: Verify header is up-to-date (for CI)
        verify_header();
    }
}
