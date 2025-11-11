use std::collections::HashMap;
use std::fs;
use std::path::Path;

// This test verifies that the generated header `src/roup_constants.h` contains
// all expected `ROUP_ACC_CLAUSE_*` macro definitions and none are left as the
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
        "ROUP_ACC_CLAUSE_ASYNC",
        "ROUP_ACC_CLAUSE_WAIT",
        "ROUP_ACC_CLAUSE_NUM_GANGS",
        "ROUP_ACC_CLAUSE_NUM_WORKERS",
        "ROUP_ACC_CLAUSE_VECTOR_LENGTH",
        "ROUP_ACC_CLAUSE_GANG",
        "ROUP_ACC_CLAUSE_WORKER",
        "ROUP_ACC_CLAUSE_VECTOR",
        "ROUP_ACC_CLAUSE_SEQ",
        "ROUP_ACC_CLAUSE_INDEPENDENT",
        "ROUP_ACC_CLAUSE_AUTO",
        "ROUP_ACC_CLAUSE_COLLAPSE",
        "ROUP_ACC_CLAUSE_DEVICE_TYPE",
        "ROUP_ACC_CLAUSE_BIND",
        "ROUP_ACC_CLAUSE_IF",
        "ROUP_ACC_CLAUSE_DEFAULT",
        "ROUP_ACC_CLAUSE_FIRSTPRIVATE",
        "ROUP_ACC_CLAUSE_DEFAULT_ASYNC",
        "ROUP_ACC_CLAUSE_LINK",
        "ROUP_ACC_CLAUSE_NO_CREATE",
        "ROUP_ACC_CLAUSE_NOHOST",
        "ROUP_ACC_CLAUSE_PRESENT",
        "ROUP_ACC_CLAUSE_PRIVATE",
        "ROUP_ACC_CLAUSE_REDUCTION",
        "ROUP_ACC_CLAUSE_READ",
        "ROUP_ACC_CLAUSE_SELF",
        "ROUP_ACC_CLAUSE_TILE",
        "ROUP_ACC_CLAUSE_USE_DEVICE",
        "ROUP_ACC_CLAUSE_ATTACH",
        "ROUP_ACC_CLAUSE_DETACH",
        "ROUP_ACC_CLAUSE_FINALIZE",
        "ROUP_ACC_CLAUSE_IF_PRESENT",
        "ROUP_ACC_CLAUSE_CAPTURE",
        "ROUP_ACC_CLAUSE_WRITE",
        "ROUP_ACC_CLAUSE_UPDATE",
        "ROUP_ACC_CLAUSE_COPY",
        "ROUP_ACC_CLAUSE_COPYIN",
        "ROUP_ACC_CLAUSE_COPYOUT",
        "ROUP_ACC_CLAUSE_CREATE",
        "ROUP_ACC_CLAUSE_DELETE",
        "ROUP_ACC_CLAUSE_DEVICE",
        "ROUP_ACC_CLAUSE_DEVICEPTR",
        "ROUP_ACC_CLAUSE_DEVICE_NUM",
        "ROUP_ACC_CLAUSE_DEVICE_RESIDENT",
        "ROUP_ACC_CLAUSE_HOST",
        "ROUP_ACC_CLAUSE_NUM_THREADS",
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
        eprintln!("Found duplicate ROUP_ACC_CLAUSE_* numeric values:");
        for (val, names) in &duplicates {
            eprintln!("  {} -> {:?}", val, names);
        }
        panic!("Duplicate ROUP_ACC_CLAUSE_* numeric values found in generated header");
    }
}
