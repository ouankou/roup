# Language-Aware OpenMP Clause Parsing

ROUP provides a **language-aware frontend** that parses C/C++/Fortran constructs
in OpenMP clauses with just enough understanding to extract semantic information
without implementing full language parsers.

## Overview

The language-aware parser enables ROUP to:
- Parse OpenMP array sections (`arr[0:N:2]` in C/C++, `array(1:n, :)` in Fortran)
- Extract structured variable information with array dimensions
- Handle C++ templates (`std::map<int, float>`)
- Respect quote strings and avoid splitting on commas inside them
- Support mapper syntax in map clauses (`mapper(custom), to: arr`)
- Properly detect colons at the top level (avoiding ternary operators)

## Key Capabilities

### C/C++ Support
- **Array sections**: `arr[lower:length:stride]` syntax
- **Nested sections**: `matrix[0:N][i:j]` with multiple dimensions
- **Templates**: `std::vector<int, std::allocator<int>>` treated as single item
- **Namespaces**: `ns::value` preserved in identifiers
- **Quote strings**: `"a,b,c"` not split at commas

### Fortran Support
- **Array sections**: `array(lower:upper:stride)` with parentheses
- **Multi-dimensional**: `field(1:n, :, 2:m:2)` with rank separation
- **All-elements notation**: `:` becomes empty array section
- **Case insensitivity**: Names normalized according to Fortran rules

### Advanced Features
- **Mapper syntax**: `mapper(identifier)` prefix in map clauses
- **Ternary operators**: `a ? b : c` doesn't confuse colon detection
- **Nested structures**: Proper depth tracking for `()`, `[]`, `{}`
- **Expression preservation**: Complex expressions stored as strings

## Configuration

Language semantics are **enabled by default** and controlled via `ParserConfig`:

```rust
use roup::ir::{ParserConfig, Language};

// Enable language-aware parsing (default)
let config = ParserConfig::with_parsing(Language::C);

// Disable for legacy string-only behavior
let config = ParserConfig::with_parsing(Language::C)
    .with_language_semantics(false);

// Check if enabled
if config.language_semantics_enabled() {
    // Use structured parsing
}
```

### When to Disable

You might want to disable language semantics when:
- **Debugging**: Compare with legacy behavior
- **Performance**: Skip parsing for simple cases
- **Custom parsing**: Handle clause payloads independently
- **Testing**: Verify fallback behavior

## Architecture

### Module Organization

```
src/ir/
├── lang/
│   └── mod.rs          # Language-aware parsing helpers
├── expression.rs       # ParserConfig with language_semantics toggle
├── convert.rs          # IR conversion using language helpers
└── error.rs            # Shared ConversionError types
```

### Key Functions

**In `src/ir/lang/mod.rs`:**
- `parse_clause_item_list()` - Parse comma-separated clause items
- `split_once_top_level()` - Find first top-level delimiter
- `rsplit_once_top_level()` - Find last top-level delimiter
- `split_top_level()` - Split respecting quotes, templates, nesting

**In `src/ir/convert.rs`:**
- `parse_identifier_list()` - Public API for clause item parsing
- `parse_map_clause()` - Map clause with mapper support
- `parse_linear_clause()` - Linear clause with step detection
- `extract_parenthesized()` - Extract content from `()`

## Implementation Details

### Quote and Template Handling

The parser tracks quote state and detects C++ templates:

```rust
// These are treated as single items:
"std::map<int, float>"          // C++ template
"str = \"a,b,c\""               // Quoted string
"a ? b : c"                     // Ternary operator
```

### Delimiter Detection

Top-level delimiters are found by tracking nesting depth:

```rust
split_once_top_level("map(to: arr)", ':')  // Finds colon outside parens
rsplit_once_top_level("i: j: k", ':')      // Finds last colon
```

### Array Section Parsing

Array sections are parsed per language:

```rust
// C/C++: brackets with length semantics
arr[lower:length:stride]  → ArraySection { lower, length, stride }

// Fortran: parentheses with upper bound semantics
array(lower:upper:stride) → ArraySection { lower, upper, stride }
```

## Examples

### Basic Usage

```rust
use roup::ir::{convert::parse_identifier_list, ParserConfig, Language};

let config = ParserConfig::with_parsing(Language::C);

// Simple identifiers
let items = parse_identifier_list("x, y, z", &config).unwrap();
assert_eq!(items.len(), 3);

// Array sections
let items = parse_identifier_list("arr[0:N], matrix[i][j]", &config).unwrap();
assert_eq!(items.len(), 2);
```

### Full Directive Parsing

```rust
use roup::parser::parse_omp_directive;
use roup::ir::{convert::convert_directive, Language, ParserConfig, SourceLocation};

let input = "#pragma omp target map(to: arr[0:N:2])";
let (_, directive) = parse_omp_directive(input).unwrap();

let config = ParserConfig::with_parsing(Language::C);
let ir = convert_directive(&directive, SourceLocation::start(), Language::C, &config).unwrap();

// Access structured clause data
for clause in ir.clauses() {
    if let ClauseData::Map { items, .. } = clause {
        // items[0] is a Variable with array_sections
    }
}
```

## Testing

Comprehensive tests verify:
- `src/ir/lang/mod.rs` - Unit tests for parsing helpers
- `src/ir/convert.rs` - Integration tests for clause conversion
- `tests/language_parsing_integration.rs` - End-to-end directive tests
- `tests/ir_roundtrip.rs` - Round-trip verification

Run tests with:
```bash
cargo test                          # All tests
cargo test language_parsing         # Language parsing tests only
cargo test --test language_parsing_integration  # Integration tests
```

## Limitations

- **Mapper modifiers**: Full `mapper(...)` syntax not yet complete
- **Linear modifiers**: `linear(modifier(list): step)` not supported
- **Fortran display**: Variable printer uses C-style `[]` notation
- **Member access**: `array(i)%field` treated as single identifier

## Performance

The language-aware parser is designed for efficiency:
- **Zero-copy parsing**: Returns string slices when possible
- **Lazy evaluation**: Only parses when language semantics enabled
- **Minimal allocations**: Uses stack-based depth tracking
- **Short-circuit**: Fast path for simple identifiers

## Future Enhancements

Potential improvements:
- Full mapper syntax support
- Linear clause modifiers
- Fortran-specific printer
- Expression AST parsing
- Performance optimizations
