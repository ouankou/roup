use std::collections::HashMap;
use std::fs;
use std::path::Path;

// This test verifies that the generated header `src/roup_constants.h` contains
// all expected `ACC_CLAUSE_*` macro definitions and none are left as the
// UNKNOWN_KIND sentinel (999). This provides fast feedback when adding new
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
        "ACC_CLAUSE_ASYNC",
        "ACC_CLAUSE_WAIT",
        "ACC_CLAUSE_NUM_GANGS",
        "ACC_CLAUSE_NUM_WORKERS",
        "ACC_CLAUSE_VECTOR_LENGTH",
        "ACC_CLAUSE_GANG",
        "ACC_CLAUSE_WORKER",
        "ACC_CLAUSE_VECTOR",
        "ACC_CLAUSE_SEQ",
        "ACC_CLAUSE_INDEPENDENT",
        "ACC_CLAUSE_AUTO",
        "ACC_CLAUSE_COLLAPSE",
        "ACC_CLAUSE_DEVICE_TYPE",
        "ACC_CLAUSE_BIND",
        "ACC_CLAUSE_IF",
        "ACC_CLAUSE_DEFAULT",
        "ACC_CLAUSE_FIRSTPRIVATE",
        "ACC_CLAUSE_DEFAULT_ASYNC",
        "ACC_CLAUSE_LINK",
        "ACC_CLAUSE_NO_CREATE",
        "ACC_CLAUSE_NOHOST",
        "ACC_CLAUSE_PRESENT",
        "ACC_CLAUSE_PRIVATE",
        "ACC_CLAUSE_REDUCTION",
        "ACC_CLAUSE_READ",
        "ACC_CLAUSE_SELF",
        "ACC_CLAUSE_TILE",
        "ACC_CLAUSE_USE_DEVICE",
        "ACC_CLAUSE_ATTACH",
        "ACC_CLAUSE_DETACH",
        "ACC_CLAUSE_FINALIZE",
        "ACC_CLAUSE_IF_PRESENT",
        "ACC_CLAUSE_CAPTURE",
        "ACC_CLAUSE_WRITE",
        "ACC_CLAUSE_UPDATE",
        "ACC_CLAUSE_COPY",
        "ACC_CLAUSE_COPYIN",
        "ACC_CLAUSE_COPYOUT",
        "ACC_CLAUSE_CREATE",
        "ACC_CLAUSE_DELETE",
        "ACC_CLAUSE_DEVICE",
        "ACC_CLAUSE_DEVICEPTR",
        "ACC_CLAUSE_DEVICE_NUM",
        "ACC_CLAUSE_DEVICE_RESIDENT",
        "ACC_CLAUSE_HOST",
        "ACC_CLAUSE_NUM_THREADS",
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

        // Ensure macro value is not the UNKNOWN_KIND (999)
        assert_ne!(val, 999, "Macro {} is UNKNOWN_KIND in header", name);

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
        eprintln!("Found duplicate ACC_CLAUSE_* numeric values:");
        for (val, names) in &duplicates {
            eprintln!("  {} -> {:?}", val, names);
        }
        panic!("Duplicate ACC_CLAUSE_* numeric values found in generated header");
    }
}
