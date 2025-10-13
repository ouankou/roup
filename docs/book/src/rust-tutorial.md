# Rust Tutorial

Learn idiomatic Rust patterns for parsing and querying OpenMP directives with ROUP.

---

## Overview

This tutorial covers:

1. [Basic Parsing](#basic-parsing) - Parse your first directive
2. [Error Handling](#error-handling) - Robust error handling patterns
3. [Querying Directives](#querying-directives) - Extract directive information
4. [Working with Clauses](#working-with-clauses) - Iterate and pattern match clauses
5. [Advanced Patterns](#advanced-patterns) - Real-world usage patterns
6. [Testing](#testing) - Writing tests for your parser integration

---

## Basic Parsing

### Your First Parse

```rust,ignore
use roup::parser::openmp::parse_openmp_directive;
use roup::lexer::Language;

fn main() {
    let input = "#pragma omp parallel";
    
    match parse_openmp_directive(input, Language::C) {
        Ok(directive) => {
            println!("Successfully parsed: {:?}", directive.kind);
        }
        Err(e) => {
            eprintln!("Parse error: {}", e);
        }
    }
}
```

### Parse with Clauses

```rust,ignore
use roup::parser::openmp::parse_openmp_directive;
use roup::lexer::Language;

fn main() {
    let input = "#pragma omp parallel for num_threads(4) private(x, y)";
    
    match parse_openmp_directive(input, Language::C) {
        Ok(directive) => {
            println!("Directive: {:?}", directive.kind);
            println!("Clauses: {}", directive.clauses.len());
            
            for (i, clause) in directive.clauses.iter().enumerate() {
                println!("  Clause {}: {:?}", i + 1, clause);
            }
        }
        Err(e) => {
            eprintln!("Failed to parse: {}", e);
        }
    }
}
```

**Output:**
```text
Directive: ParallelFor
Clauses: 2
  Clause 1: NumThreads(Expr { value: "4", .. })
  Clause 2: Private { items: ["x", "y"], .. }
```

---

## Error Handling

### Basic Error Handling

```rust,ignore
use roup::parser::openmp::parse_openmp_directive;
use roup::lexer::Language;

fn parse_directive(input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let directive = parse_openmp_directive(input, Language::C)?;
    
    println!("Parsed: {:?}", directive.kind);
    println!("Location: line {}, column {}", 
             directive.location.line, 
             directive.location.column);
    
    Ok(())
}

fn main() {
    let inputs = vec![
        "#pragma omp parallel",
        "#pragma omp for schedule(static)",
        "#pragma omp invalid",  // This will fail
    ];
    
    for input in inputs {
        match parse_directive(input) {
            Ok(()) => println!("✓ Success: {}", input),
            Err(e) => eprintln!("✗ Error: {} - {}", input, e),
        }
    }
}
```

### Custom Error Type

```rust,ignore
use roup::parser::openmp::parse_openmp_directive;
use roup::lexer::Language;
use std::fmt;

#[derive(Debug)]
enum OpenMPError {
    ParseError(String),
    UnsupportedDirective(String),
    MissingRequiredClause(String),
}

impl fmt::Display for OpenMPError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OpenMPError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            OpenMPError::UnsupportedDirective(kind) => {
                write!(f, "Unsupported directive: {}", kind)
            }
            OpenMPError::MissingRequiredClause(clause) => {
                write!(f, "Missing required clause: {}", clause)
            }
        }
    }
}

impl std::error::Error for OpenMPError {}

fn validate_parallel_directive(input: &str) -> Result<(), OpenMPError> {
    let directive = parse_openmp_directive(input, Language::C)
        .map_err(|e| OpenMPError::ParseError(e.to_string()))?;
    
    if !directive.kind.is_parallel() {
        return Err(OpenMPError::UnsupportedDirective(
            format!("{:?}", directive.kind)
        ));
    }
    
    // Check for required clauses (example: must have num_threads)
    let has_num_threads = directive.clauses.iter()
        .any(|c| c.is_num_threads());
    
    if !has_num_threads {
        return Err(OpenMPError::MissingRequiredClause("num_threads".into()));
    }
    
    Ok(())
}

fn main() {
    match validate_parallel_directive("#pragma omp parallel num_threads(4)") {
        Ok(()) => println!("✓ Valid parallel directive"),
        Err(e) => eprintln!("✗ {}", e),
    }
}
```

---

## Querying Directives

### Check Directive Kind

```rust,ignore
use roup::parser::openmp::parse_openmp_directive;
use roup::ir::DirectiveKind;
use roup::lexer::Language;

fn main() {
    let input = "#pragma omp parallel for";
    let directive = parse_openmp_directive(input, Language::C).unwrap();
    
    // Pattern match on kind
    match directive.kind {
        DirectiveKind::Parallel => println!("This is a parallel directive"),
        DirectiveKind::For => println!("This is a for directive"),
        DirectiveKind::ParallelFor => println!("This is a combined parallel for"),
        DirectiveKind::Target => println!("This is a target directive"),
        _ => println!("Other directive type"),
    }
    
    // Or use helper methods
    if directive.kind.is_parallel() {
        println!("Contains parallel semantics");
    }
    
    if directive.kind.is_worksharing() {
        println!("Is a worksharing construct");
    }
}
```

### Extract Source Location

```rust,ignore
use roup::parser::openmp::parse_openmp_directive;
use roup::lexer::Language;

fn main() {
    let input = "#pragma omp parallel";
    let directive = parse_openmp_directive(input, Language::C).unwrap();
    
    println!("Directive found at:");
    println!("  Line: {}", directive.location.line);
    println!("  Column: {}", directive.location.column);
    println!("  Language: {:?}", directive.language);
}
```

---

## Working with Clauses

### Iterate Over Clauses

```rust,ignore
use roup::parser::openmp::parse_openmp_directive;
use roup::lexer::Language;

fn main() {
    let input = "#pragma omp parallel num_threads(8) default(shared) private(x)";
    let directive = parse_openmp_directive(input, Language::C).unwrap();
    
    println!("Found {} clauses:", directive.clauses.len());
    
    for clause in &directive.clauses {
        println!("  - {:?}", clause);
    }
}
```

### Pattern Match on Clauses

```rust,ignore
use roup::parser::openmp::parse_openmp_directive;
use roup::ir::ClauseData;
use roup::lexer::Language;

fn main() {
    let input = "#pragma omp parallel num_threads(4) default(shared) private(x, y)";
    let directive = parse_openmp_directive(input, Language::C).unwrap();
    
    for clause in &directive.clauses {
        match clause {
            ClauseData::NumThreads(expr) => {
                println!("Thread count: {}", expr.value);
            }
            ClauseData::Default(kind) => {
                println!("Default sharing: {:?}", kind);
            }
            ClauseData::Private { items, .. } => {
                println!("Private variables: {:?}", items);
            }
            ClauseData::Shared { items, .. } => {
                println!("Shared variables: {:?}", items);
            }
            ClauseData::Reduction { operator, items, .. } => {
                println!("Reduction: {:?} on {:?}", operator, items);
            }
            _ => {
                println!("Other clause: {:?}", clause);
            }
        }
    }
}
```

### Find Specific Clauses

```rust,ignore
use roup::parser::openmp::parse_openmp_directive;
use roup::ir::ClauseData;
use roup::lexer::Language;

fn get_thread_count(input: &str) -> Option<String> {
    let directive = parse_openmp_directive(input, Language::C).ok()?;
    
    directive.clauses.iter()
        .find_map(|clause| {
            if let ClauseData::NumThreads(expr) = clause {
                Some(expr.value.to_string())
            } else {
                None
            }
        })
}

fn get_private_vars(input: &str) -> Vec<String> {
    let directive = parse_openmp_directive(input, Language::C)
        .ok()
        .unwrap_or_default();
    
    directive.clauses.iter()
        .filter_map(|clause| {
            if let ClauseData::Private { items, .. } = clause {
                Some(items.iter().map(|s| s.to_string()).collect())
            } else {
                None
            }
        })
        .flatten()
        .collect()
}

fn main() {
    let input = "#pragma omp parallel num_threads(8) private(x, y, z)";
    
    if let Some(count) = get_thread_count(input) {
        println!("Thread count: {}", count);
    }
    
    let vars = get_private_vars(input);
    println!("Private variables: {:?}", vars);
}
```

**Output:**
```text
Thread count: 8
Private variables: ["x", "y", "z"]
```

---

## Advanced Patterns

### Parse Multiple Directives

```rust,ignore
use roup::parser::openmp::parse_openmp_directive;
use roup::ir::DirectiveIR;
use roup::lexer::Language;

fn parse_file_directives(source: &str) -> Vec<DirectiveIR> {
    source.lines()
        .filter(|line| line.trim().starts_with("#pragma omp"))
        .filter_map(|line| {
            parse_openmp_directive(line, Language::C).ok()
        })
        .collect()
}

fn main() {
    let source = r#"
    #pragma omp parallel num_threads(4)
    for (int i = 0; i < n; i++) {
        #pragma omp task
        process(i);
    }
    #pragma omp taskwait
    "#;
    
    let directives = parse_file_directives(source);
    
    println!("Found {} OpenMP directives:", directives.len());
    for (i, dir) in directives.iter().enumerate() {
        println!("  {}. {:?} at line {}", 
                 i + 1, dir.kind, dir.location.line);
    }
}
```

### Directive Analysis

```rust,ignore
use roup::parser::openmp::parse_openmp_directive;
use roup::ir::{DirectiveIR, ClauseData};
use roup::lexer::Language;

struct DirectiveStats {
    total_clauses: usize,
    has_data_sharing: bool,
    has_scheduling: bool,
    has_reduction: bool,
    thread_count: Option<String>,
}

impl DirectiveStats {
    fn analyze(directive: &DirectiveIR) -> Self {
        let total_clauses = directive.clauses.len();
        
        let mut has_data_sharing = false;
        let mut has_scheduling = false;
        let mut has_reduction = false;
        let mut thread_count = None;
        
        for clause in &directive.clauses {
            match clause {
                ClauseData::Private { .. } | 
                ClauseData::Shared { .. } |
                ClauseData::Firstprivate { .. } |
                ClauseData::Lastprivate { .. } => {
                    has_data_sharing = true;
                }
                ClauseData::Schedule { .. } => {
                    has_scheduling = true;
                }
                ClauseData::Reduction { .. } => {
                    has_reduction = true;
                }
                ClauseData::NumThreads(expr) => {
                    thread_count = Some(expr.value.to_string());
                }
                _ => {}
            }
        }
        
        Self {
            total_clauses,
            has_data_sharing,
            has_scheduling,
            has_reduction,
            thread_count,
        }
    }
}

fn main() {
    let input = "#pragma omp parallel for num_threads(4) \
                 schedule(static) private(x) reduction(+:sum)";
    
    let directive = parse_openmp_directive(input, Language::C).unwrap();
    let stats = DirectiveStats::analyze(&directive);
    
    println!("Directive Analysis:");
    println!("  Total clauses: {}", stats.total_clauses);
    println!("  Has data-sharing: {}", stats.has_data_sharing);
    println!("  Has scheduling: {}", stats.has_scheduling);
    println!("  Has reduction: {}", stats.has_reduction);
    if let Some(count) = stats.thread_count {
        println!("  Thread count: {}", count);
    }
}
```

**Output:**
```text
Directive Analysis:
  Total clauses: 4
  Has data-sharing: true
  Has scheduling: true
  Has reduction: true
  Thread count: 4
```

### Building a Directive Validator

```rust,ignore
use roup::parser::openmp::parse_openmp_directive;
use roup::ir::{DirectiveIR, DirectiveKind, ClauseData};
use roup::lexer::Language;

struct ValidationRule {
    name: &'static str,
    check: fn(&DirectiveIR) -> bool,
}

fn validate_directive(directive: &DirectiveIR, rules: &[ValidationRule]) -> Vec<String> {
    rules.iter()
        .filter(|rule| !(rule.check)(directive))
        .map(|rule| rule.name.to_string())
        .collect()
}

fn main() {
    let rules = vec![
        ValidationRule {
            name: "Parallel regions should specify thread count",
            check: |dir| {
                !dir.kind.is_parallel() || 
                dir.clauses.iter().any(|c| matches!(c, ClauseData::NumThreads(_)))
            },
        },
        ValidationRule {
            name: "For loops with reduction should have schedule clause",
            check: |dir| {
                let has_reduction = dir.clauses.iter()
                    .any(|c| matches!(c, ClauseData::Reduction { .. }));
                let has_schedule = dir.clauses.iter()
                    .any(|c| matches!(c, ClauseData::Schedule { .. }));
                
                !has_reduction || has_schedule
            },
        },
    ];
    
    let input = "#pragma omp parallel";  // Missing num_threads
    let directive = parse_openmp_directive(input, Language::C).unwrap();
    
    let violations = validate_directive(&directive, &rules);
    
    if violations.is_empty() {
        println!("✓ All validation rules passed");
    } else {
        println!("✗ Validation warnings:");
        for violation in violations {
            println!("  - {}", violation);
        }
    }
}
```

---

## Testing

### Unit Testing

```rust,ignore
#[cfg(test)]
mod tests {
    use roup::parser::openmp::parse_openmp_directive;
    use roup::ir::DirectiveKind;
    use roup::lexer::Language;
    
    #[test]
    fn test_parse_parallel() {
        let input = "#pragma omp parallel";
        let result = parse_openmp_directive(input, Language::C);
        
        assert!(result.is_ok());
        let directive = result.unwrap();
        assert_eq!(directive.kind, DirectiveKind::Parallel);
        assert_eq!(directive.clauses.len(), 0);
    }
    
    #[test]
    fn test_parse_with_clauses() {
        let input = "#pragma omp parallel num_threads(4)";
        let directive = parse_openmp_directive(input, Language::C).unwrap();
        
        assert_eq!(directive.kind, DirectiveKind::Parallel);
        assert_eq!(directive.clauses.len(), 1);
    }
    
    #[test]
    fn test_invalid_directive() {
        let input = "#pragma omp invalid_directive";
        let result = parse_openmp_directive(input, Language::C);
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_fortran_syntax() {
        let input = "!$omp parallel";
        let result = parse_openmp_directive(input, Language::Fortran);
        
        assert!(result.is_ok());
        let directive = result.unwrap();
        assert_eq!(directive.kind, DirectiveKind::Parallel);
    }
}
```

### Integration Testing

```rust,ignore
#[cfg(test)]
mod integration_tests {
    use roup::parser::openmp::parse_openmp_directive;
    use roup::ir::ClauseData;
    use roup::lexer::Language;
    
    #[test]
    fn test_complete_parsing_pipeline() {
        let inputs = vec![
            "#pragma omp parallel",
            "#pragma omp for schedule(static)",
            "#pragma omp parallel for num_threads(8) private(x)",
            "#pragma omp task depend(in: x) priority(10)",
        ];
        
        for input in inputs {
            let result = parse_openmp_directive(input, Language::C);
            assert!(result.is_ok(), "Failed to parse: {}", input);
            
            let directive = result.unwrap();
            assert!(directive.kind.is_valid());
            
            // Verify round-trip
            let output = directive.to_string();
            assert!(!output.is_empty());
        }
    }
    
    #[test]
    fn test_clause_extraction() {
        let input = "#pragma omp parallel for \
                     num_threads(4) \
                     schedule(dynamic, 100) \
                     private(i, j) \
                     reduction(+:sum)";
        
        let directive = parse_openmp_directive(input, Language::C).unwrap();
        
        // Count clause types
        let mut num_threads_count = 0;
        let mut schedule_count = 0;
        let mut private_count = 0;
        let mut reduction_count = 0;
        
        for clause in &directive.clauses {
            match clause {
                ClauseData::NumThreads(_) => num_threads_count += 1,
                ClauseData::Schedule { .. } => schedule_count += 1,
                ClauseData::Private { .. } => private_count += 1,
                ClauseData::Reduction { .. } => reduction_count += 1,
                _ => {}
            }
        }
        
        assert_eq!(num_threads_count, 1);
        assert_eq!(schedule_count, 1);
        assert_eq!(private_count, 1);
        assert_eq!(reduction_count, 1);
    }
}
```

---

## Best Practices

### 1. Always Handle Errors

```rust,ignore
// ❌ Bad - unwrap can panic
let directive = parse_openmp_directive(input, Language::C).unwrap();

// ✅ Good - explicit error handling
match parse_openmp_directive(input, Language::C) {
    Ok(directive) => { /* use directive */ }
    Err(e) => { /* handle error */ }
}
```

### 2. Use Pattern Matching

```rust,ignore
// ❌ Bad - lots of if-let chains
for clause in &directive.clauses {
    if let ClauseData::NumThreads(expr) = clause {
        // ...
    } else if let ClauseData::Private { items, .. } = clause {
        // ...
    }
}

// ✅ Good - clean match expression
for clause in &directive.clauses {
    match clause {
        ClauseData::NumThreads(expr) => { /* ... */ }
        ClauseData::Private { items, .. } => { /* ... */ }
        _ => {}
    }
}
```

### 3. Leverage Iterator Combinators

```rust,ignore
// ❌ Bad - manual iteration
let mut has_reduction = false;
for clause in &directive.clauses {
    if matches!(clause, ClauseData::Reduction { .. }) {
        has_reduction = true;
        break;
    }
}

// ✅ Good - iterator method
let has_reduction = directive.clauses.iter()
    .any(|c| matches!(c, ClauseData::Reduction { .. }));
```

### 4. Create Helper Functions

```rust,ignore
// Reusable helper
fn has_clause<F>(directive: &DirectiveIR, predicate: F) -> bool
where
    F: Fn(&ClauseData) -> bool,
{
    directive.clauses.iter().any(predicate)
}

// Usage
if has_clause(&directive, |c| matches!(c, ClauseData::NumThreads(_))) {
    println!("Has num_threads clause");
}
```

---

## Next Steps

- **[C Tutorial](./c-tutorial.md)** - Learn the C FFI API
- **[C++ Tutorial](./cpp-tutorial.md)** - Build a real-world application
- **[API Reference](./api-reference.md)** - Complete Rust API documentation
- **Examples** - Check out `tests/` directory for 355+ test cases

---

## Summary

**Key Takeaways:**

1. Use `parse_openmp_directive()` for parsing
2. Handle errors with `Result` types
3. Pattern match on `DirectiveKind` and `ClauseData`
4. Use iterators for clause analysis
5. Write tests for your integration code

**Common Patterns:**

```rust,ignore
// Parse
let directive = parse_openmp_directive(input, Language::C)?;

// Check kind
if directive.kind.is_parallel() { /* ... */ }

// Find clause
let num_threads = directive.clauses.iter()
    .find_map(|c| match c {
        ClauseData::NumThreads(expr) => Some(expr.value.clone()),
        _ => None,
    });

// Analyze all clauses
for clause in &directive.clauses {
    match clause {
        ClauseData::Private { items, .. } => { /* ... */ }
        ClauseData::Shared { items, .. } => { /* ... */ }
        _ => {}
    }
}
```

**Happy parsing!** 🦀
