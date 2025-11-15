use std::collections::HashMap;
use std::fs;
use std::path::Path;

// This test verifies that the generated header `src/roup_constants.h` contains
// all expected `ROUP_ACCC_*` macro definitions and none are left as the
// UNKNOWN_KIND sentinel (-1). This provides fast feedback when adding new
// OpenACC clauses or changing the generator.

#[test]
fn generated_header_has_all_acc_clauses() {
    let header_path = Path::new("src/roup_constants.h");
    assert!(
        header_path.exists(),
        "Expected generated header at src/roup_constants.h"
    );

    let content = fs::read_to_string(header_path).expect("Failed to read generated header");

    // These names must match the list in src/constants_gen.rs expected list.
    let expected = [
        "ROUP_ACCC_async",
        "ROUP_ACCC_wait",
        "ROUP_ACCC_num_gangs",
        "ROUP_ACCC_num_workers",
        "ROUP_ACCC_vector_length",
        "ROUP_ACCC_gang",
        "ROUP_ACCC_worker",
        "ROUP_ACCC_vector",
        "ROUP_ACCC_seq",
        "ROUP_ACCC_independent",
        "ROUP_ACCC_auto",
        "ROUP_ACCC_collapse",
        "ROUP_ACCC_device_type",
        "ROUP_ACCC_bind",
        "ROUP_ACCC_if",
        "ROUP_ACCC_default",
        "ROUP_ACCC_firstprivate",
        "ROUP_ACCC_default_async",
        "ROUP_ACCC_link",
        "ROUP_ACCC_no_create",
        "ROUP_ACCC_nohost",
        "ROUP_ACCC_present",
        "ROUP_ACCC_private",
        "ROUP_ACCC_reduction",
        "ROUP_ACCC_read",
        "ROUP_ACCC_self",
        "ROUP_ACCC_tile",
        "ROUP_ACCC_use_device",
        "ROUP_ACCC_attach",
        "ROUP_ACCC_detach",
        "ROUP_ACCC_finalize",
        "ROUP_ACCC_if_present",
        "ROUP_ACCC_capture",
        "ROUP_ACCC_write",
        "ROUP_ACCC_update",
        "ROUP_ACCC_copy",
        "ROUP_ACCC_copyin",
        "ROUP_ACCC_copyout",
        "ROUP_ACCC_create",
        "ROUP_ACCC_delete",
        "ROUP_ACCC_device",
        "ROUP_ACCC_deviceptr",
        "ROUP_ACCC_device_num",
        "ROUP_ACCC_device_resident",
        "ROUP_ACCC_host",
        "ROUP_ACCC_num_threads",
    ];

    // Map from macro name -> numeric value (as i64) for later duplicate-checks
    let mut values: HashMap<String, i64> = HashMap::new();

    for name in &expected {
        // Find macro definition line: match lines like `#define NAME <value>`
        let found = content.lines().find(|l| {
            let parts: Vec<&str> = l.split_whitespace().collect();
            parts.len() >= 2 && parts[0] == "#define" && parts[1] == *name
        });
        assert!(
            found.is_some(),
            "Missing macro {} in generated header",
            name
        );

        let line = found.unwrap();

        // Extract the last whitespace-separated token as the value. Support decimal
        // and hex (0x...) numeric formats.
        let parts: Vec<&str> = line.split_whitespace().collect();
        assert!(
            parts.len() >= 3,
            "Unexpected macro format for {}: {}",
            name,
            line
        );
        let val_str = parts.last().unwrap();

        let val: i64 = if val_str.starts_with("0x") || val_str.starts_with("0X") {
            i64::from_str_radix(&val_str[2..], 16).expect("Failed to parse hex macro value")
        } else {
            val_str
                .parse::<i64>()
                .expect("Failed to parse decimal macro value")
        };

        // Print macros and values for easier debugging when the test runs in CI
        println!("{} = {}", name, val);

        // Ensure macro value is not the UNKNOWN_KIND (-1)
        assert_ne!(val, -1, "Macro {} is UNKNOWN_KIND in header", name);

        values.insert(name.to_string(), val);
    }

    // Check for duplicate numeric values: build reverse map value -> names
    let mut rev: HashMap<i64, Vec<String>> = HashMap::new();
    for (name, v) in &values {
        rev.entry(*v).or_default().push(name.clone());
    }

    // Collect duplicates (values with more than one name)
    let duplicates: Vec<(&i64, &Vec<String>)> =
        rev.iter().filter(|(_, names)| names.len() > 1).collect();
    if !duplicates.is_empty() {
        eprintln!("Found duplicate ROUP_ACCC_* numeric values:");
        for (val, names) in &duplicates {
            eprintln!("  {} -> {:?}", val, names);
        }
        panic!("Duplicate ROUP_ACCC_* numeric values found in generated header");
    }
}
