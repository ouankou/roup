//! Header verification binary - ensures roup_constants.h is up-to-date
//!
//! **Usage:** `cargo run --bin gen`
//!
//! This binary verifies that the committed `src/roup_constants.h` matches
//! the constants defined in `src/c_api.rs`. It's used in CI to catch cases
//! where the header wasn't regenerated after modifying the source.
//!
//! For design rationale and implementation details, see:
//! [`docs/BUILD_SCRIPT_RATIONALE.md`](../../docs/BUILD_SCRIPT_RATIONALE.md)

use std::fs;
use std::path::Path;
use std::process;

// Use #[path] to share code with build.rs
// This pattern is documented in docs/BUILD_SCRIPT_RATIONALE.md
#[path = "../constants_gen.rs"]
mod constants_gen;

// Only import the functions we actually use (verification, not generation)
use constants_gen::{
    calculate_checksum, extract_checksum_from_header, parse_clause_mappings,
    parse_directive_mappings,
};

fn main() {
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
            println!("Committed header:       {checksum}");

            if checksum == expected_checksum {
                println!("✓ Header is up-to-date with source code");
                process::exit(0);
            } else {
                eprintln!();
                eprintln!("❌ ERROR: Header is out of date!");
                eprintln!("   Expected: {expected_checksum}");
                eprintln!("   Found:    {checksum}");
                eprintln!();
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
