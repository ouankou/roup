# ROUP Step-by-Step Parser Debugger

## Table of Contents

1. [Goal](#goal)
2. [Design Philosophy](#design-philosophy)
3. [Implementation Details](#implementation-details)
4. [User Tutorial](#user-tutorial)
5. [Developer Guide](#developer-guide)
6. [Future Enhancements](#future-enhancements)

---

## Goal

The ROUP debugger is an **interactive educational and debugging tool** that breaks down the parsing of OpenMP and OpenACC directives into discrete, observable steps. It serves three primary purposes:

### 1. **Educational Tool**
- Teaches students and developers how parsers work
- Visualizes the step-by-step tokenization and parsing process
- Shows exactly how directive syntax is decomposed into an Abstract Syntax Tree (AST)
- Demonstrates the relationship between text input and parser output

### 2. **Debugging Tool**
- Diagnoses parsing failures by showing exactly where and why parsing stops
- Helps identify syntax errors in complex directives
- Validates that directives are being parsed as expected
- Assists in parser development and testing

### 3. **Documentation Tool**
- Provides concrete examples of valid directive syntax
- Shows the internal structure of parsed directives
- Documents the parser's behavior through interactive exploration

---

## Design Philosophy

### Core Principles

#### 1. **Minimal Code Duplication**
The debugger achieves ~95% code reuse by:
- Calling existing `parse_omp_directive()` and `parse_acc_directive()` functions
- Using the same `Directive` and `Clause` data structures
- Leveraging existing lexer functions (`lex_pragma()`, `skip_space_and_comments()`, etc.)
- Adding only a thin instrumentation layer to capture intermediate steps

**Result**: ~800 lines of new code, zero changes to core parser.

#### 2. **Zero Maintenance for New Directives**
The debugger is **future-proof** because it:
- Works generically with the `Directive` structure, not specific directive names
- Automatically picks up new directives added to registries
- Handles custom directive parsers without modification
- Requires updates only if core data structures change

#### 3. **Educational First, Performance Second**
The debugger prioritizes:
- **Clarity**: Each step is clearly labeled and explained
- **Completeness**: Shows every parsing operation, even whitespace skipping
- **Accessibility**: Both interactive and batch modes for different learning styles
- **Visualization**: Beautiful AST tree display with box-drawing characters

---

## Implementation Details

### Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    roup_debug Binary                     │
│                  (src/bin/roup_debug.rs)                 │
│  - CLI argument parsing                                  │
│  - Language/dialect detection                            │
│  - Mode selection (interactive/non-interactive)          │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│              Debugger Module (src/debugger/)             │
├─────────────────────────────────────────────────────────┤
│  mod.rs          - Public API, configuration, errors    │
│  stepper.rs      - DebugSession, step capture logic     │
│  ast_display.rs  - Tree visualization                   │
│  ui.rs           - Interactive terminal interface        │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│           Existing Parser (src/parser/)                  │
│  - DirectiveRegistry, ClauseRegistry                     │
│  - parse_omp_directive(), parse_acc_directive()          │
│  - Directive, Clause data structures                     │
└─────────────────────────────────────────────────────────┘
```

### Key Components

#### 1. **DebugSession** (`src/debugger/stepper.rs`)

The core of the debugger, responsible for:
- Parsing the input and capturing all intermediate steps
- Storing the final parsed directive
- Managing navigation state (current step index)

```rust
pub struct DebugSession {
    pub original_input: String,
    pub config: DebugConfig,
    pub steps: Vec<DebugStep>,
    pub final_directive: Option<Directive<'static>>,
    pub error: Option<String>,
    pub current_step_index: usize,
}
```

**Key Methods**:
- `new(input: &str, config: DebugConfig)` - Creates session and parses input
- `next_step()`, `prev_step()` - Navigation
- `current_step()` - Get current step
- `total_steps()` - Get total step count

#### 2. **DebugStep** (`src/debugger/stepper.rs`)

Represents a single parsing step:

```rust
pub struct DebugStep {
    pub step_number: usize,           // 0-indexed sequential number
    pub kind: StepKind,               // Type of step (see below)
    pub description: String,          // Human-readable description
    pub consumed: String,             // Text consumed in this step
    pub remaining: String,            // Unparsed text after this step
    pub position: usize,              // Position in original input
    pub context_stack: Vec<String>,   // Parser call stack
    pub token_info: Option<String>,   // Additional token information
}
```

**StepKind Categories**:
- `SkipWhitespace` - Skipping whitespace or comments
- `PragmaPrefix` - Parsing `#pragma omp` / `#pragma acc` / etc.
- `DirectiveName` - Parsing directive name (`parallel`, `for`, etc.)
- `DirectiveParameter` - Parsing directive parameter (e.g., `exclusive(x)` in `scan`)
- `ClauseName` - Parsing clause name (`shared`, `private`, etc.)
- `ClauseArguments` - Parsing clause arguments (content inside parentheses)
- `Complete` - Parsing successfully finished
- `Error` - Parsing failed

#### 3. **Parsing Flow**

The `parse_step_by_step()` method orchestrates the stepping:

```
Input: "#pragma omp parallel shared(x, y) private(z)"

Step 1: Collapse line continuations (if any)
Step 2: Skip leading whitespace
Step 3: Parse pragma prefix → "#pragma"
Step 4: Skip whitespace
Step 5: Call parser.parse() to get full Directive
Step 6: Decompose Directive into granular steps:
        - Directive name → "parallel"
        - Clause name → "shared"
        - Clause arguments → "(x, y)"
        - Clause name → "private"
        - Clause arguments → "(z)"
Step 7: Complete
```

**Critical Design Decision**: We call the real parser (`parser.parse()`) to get the authoritative `Directive` structure, then **retroactively decompose it** into steps. This ensures:
- The debugger always shows what the real parser does
- Zero risk of divergence between debugger and parser
- Custom parsers work automatically

#### 4. **AST Visualization** (`src/debugger/ast_display.rs`)

Renders the parsed directive as a tree:

```
Directive
├─ name: "parallel"
├─ parameter: None
└─ clauses: [2]
   ├─ Clause
   │  ├─ name: "shared"
   │  └─ kind: Parenthesized("x, y")
   └─ Clause
      ├─ name: "private"
      └─ kind: Parenthesized("z")
```

Uses Unicode box-drawing characters:
- `├─` for intermediate nodes
- `└─` for final nodes
- `│` for continuation lines

#### 5. **Interactive UI** (`src/debugger/ui.rs`)

Provides keyboard-driven navigation:

```rust
pub enum UserCommand {
    Next,              // Move to next step
    Previous,          // Move to previous step
    GoToStep(usize),   // Jump to specific step
    First,             // Jump to first step
    Last,              // Jump to last step
    ShowAst,           // Display complete AST
    ShowStep,          // Re-display current step
    ShowHistory,       // Show all steps
    ShowInput,         // Show original input
    Help,              // Show help
    Quit,              // Exit
}
```

### Lifetime Management

**Challenge**: The parser returns `Directive<'a>` where `'a` is tied to the input string's lifetime. The debugger needs to store this in a struct with `'static` lifetime.

**Solution**: Convert all borrowed data to owned:

```rust
let owned_clauses: Vec<Clause<'static>> = directive
    .clauses
    .iter()
    .map(|c| Clause {
        name: Cow::Owned(c.name.to_string()),
        kind: match &c.kind {
            ClauseKind::Bare => ClauseKind::Bare,
            ClauseKind::Parenthesized(s) => {
                ClauseKind::Parenthesized(Cow::Owned(s.to_string()))
            }
        },
    })
    .collect();
```

This creates a fully-owned copy suitable for long-term storage.

### Error Handling

The debugger gracefully handles errors:

```rust
pub enum DebugError {
    ParseError(String),    // Parser couldn't parse input
    IoError(std::io::Error), // I/O failure (stdin, terminal)
    InvalidInput(String),  // Invalid CLI arguments
}
```

When parsing fails:
1. The error is captured in `DebugSession.error`
2. A final step with `StepKind::Error` is added
3. The error message is displayed to the user
4. The program exits gracefully with exit code 1

---

## User Tutorial

### Installation

The debugger is built as part of roup:

```bash
cargo build --release
# The binary is at: target/release/roup_debug
```

Or build just the debugger:

```bash
cargo build --release --bin roup_debug
```

### Basic Usage

#### 1. **Non-Interactive Mode** (Quick Inspection)

Show all parsing steps at once:

```bash
# Via stdin
echo '#pragma omp parallel shared(x)' | roup_debug --non-interactive

# Via command-line argument
roup_debug '#pragma omp parallel shared(x)' --non-interactive

# Output:
# ═════════════════════════════════════════════════════════════
#                     All Parsing Steps
# ═════════════════════════════════════════════════════════════
#
# 1. Pragma Prefix - Parse pragma prefix
#    Consumed: #pragma
#    Prefix: "#pragma"
#
# 2. Skip Whitespace - Skip whitespace before directive
#
# 3. Directive Name - Parse directive name 'parallel'
#    Consumed: parallel
#    Directive: "parallel"
#
# [... more steps ...]
#
# Final AST:
# Directive
# ├─ name: "parallel"
# ├─ parameter: None
# └─ clauses: [1]
#    └─ Clause
#       ├─ name: "shared"
#       └─ kind: Parenthesized("x")
```

**Use cases**:
- Quick verification of parsing behavior
- Generating documentation
- CI/CD validation
- Batch processing

#### 2. **Interactive Mode** (Step-by-Step Exploration)

Navigate through parsing steps interactively:

```bash
roup_debug '#pragma omp parallel for collapse(2) reduction(+:sum)'

# You'll see:
# ═════════════════════════════════════════════════════════════
# ROUP Parser Debugger - Step 1/8 - Pragma Prefix
# ═════════════════════════════════════════════════════════════
# Input: #pragma omp parallel for collapse(2) reduction(+:sum)
#        ^
#
# Step: Parse pragma prefix
#
# Token: Prefix: "#pragma"
# Consumed: #pragma
# Remaining: "omp parallel for collapse(2) reduction(+:sum)"
#
# Parser context: Parser::parse → lex_pragma
#
# ═════════════════════════════════════════════════════════════
# [n]ext [p]rev [a]st [h]istory [?]help [q]uit >
```

**Interactive Commands**:

| Command | Action | Example |
|---------|--------|---------|
| `n`, `next`, `→` | Move to next step | Press `n` |
| `p`, `prev`, `←` | Move to previous step | Press `p` |
| `g <num>` | Jump to step number | Type `g 5` |
| `0`, `first` | Jump to first step | Type `0` |
| `$`, `last` | Jump to last step | Type `$` |
| `a`, `ast` | Show complete AST | Press `a` |
| `s`, `step` | Re-display current step | Press `s` |
| `h`, `history` | Show all steps summary | Press `h` |
| `i`, `input` | Show original input | Press `i` |
| `?`, `help` | Show help message | Press `?` |
| `q`, `quit`, `exit` | Quit debugger | Press `q` |

**Use cases**:
- Learning how the parser works
- Debugging complex directives
- Teaching parser concepts
- Exploring unfamiliar syntax

### Examples

#### Example 1: Understanding Reduction Syntax

```bash
roup_debug '#pragma omp parallel reduction(+:sum)' --non-interactive
```

**Learning outcome**: See how the reduction operator `+` and variable `sum` are parsed together as clause arguments.

#### Example 2: Debugging a Metadirective

```bash
roup_debug '#pragma omp metadirective when(device={kind(gpu)}:parallel) default(serial)'
```

**Navigate interactively**:
1. Press `n` repeatedly to see how nested braces are handled
2. Press `a` to see the final AST structure
3. Verify the `when` clause captures the entire device selector

#### Example 3: Comparing OpenMP vs OpenACC

```bash
# OpenMP
roup_debug '#pragma omp parallel' --non-interactive

# OpenACC
roup_debug '#pragma acc parallel' --non-interactive
```

**Learning outcome**: Observe that both use the same parsing structure, just different registries.

#### Example 4: Custom Parser - Scan Directive

```bash
roup_debug '#pragma omp scan exclusive(x, y)' --non-interactive
```

**Learning outcome**: See that `scan` has a **parameter** (`exclusive(x, y)`) rather than a clause, showcasing custom parser behavior.

#### Example 5: Complex Real-World Directive

```bash
roup_debug '#pragma omp target teams distribute parallel for simd \
             map(to: a[0:N]) map(from: b[0:N]) \
             num_teams(4) thread_limit(256) \
             collapse(2) reduction(+:sum)' --non-interactive
```

**Learning outcome**: See how complex directives with many clauses are parsed systematically, one clause at a time.

### Dialect Selection

By default, the debugger auto-detects the dialect from the pragma prefix:

```bash
# Auto-detects OpenMP
roup_debug '#pragma omp parallel'

# Auto-detects OpenACC
roup_debug '#pragma acc parallel'
```

Force a specific dialect:

```bash
# Force OpenMP (useful if testing unusual syntax)
roup_debug --omp '#pragma parallel'

# Force OpenACC
roup_debug --acc '#pragma parallel'
```

### Input Methods

#### Method 1: Command-line argument

```bash
roup_debug '#pragma omp parallel shared(x)'
```

**Pros**: Quick, scriptable, good for short directives

#### Method 2: Stdin

```bash
echo '#pragma omp parallel shared(x)' | roup_debug
```

**Pros**: Works with pipes, can use heredocs for multiline

#### Method 3: File input

```bash
cat my_directive.txt | roup_debug
```

**Pros**: Good for complex directives stored in files

### Tips and Tricks

#### Tip 1: Use Non-Interactive for Quick Checks

```bash
# Quickly verify directive is valid
roup_debug '#pragma omp for' --non-interactive > /dev/null && echo "Valid"
```

#### Tip 2: Capture Output for Documentation

```bash
# Generate parsing documentation
roup_debug '#pragma omp parallel' --non-interactive > parallel_parsing.txt
```

#### Tip 3: Compare Multiple Directives

```bash
# Create a comparison script
for dir in "parallel" "for" "sections"; do
    echo "=== $dir ==="
    roup_debug "#pragma omp $dir" --non-interactive
done
```

#### Tip 4: Debug Parse Failures

```bash
# See exactly where parsing fails
roup_debug '#pragma omp INVALID_DIRECTIVE' --non-interactive
# The error step shows exactly what went wrong
```

#### Tip 5: Interactive Learning Session

```bash
# Start interactive mode, then:
# 1. Press 'n' repeatedly to understand full flow
# 2. Press 'a' to see final AST
# 3. Press 'h' to see all steps summarized
# 4. Press 'p' to review previous steps
roup_debug '#pragma omp parallel for collapse(2)'
```

---

## Developer Guide

### Adding New Features to the Debugger

#### When You DON'T Need to Update the Debugger

✅ **Adding new OpenMP directives** to `src/parser/openmp.rs`
- Just add to `directive_registry()`
- Debugger automatically picks it up

✅ **Adding new OpenACC directives** to `src/parser/openacc.rs`
- Just add to `directive_registry()`
- Debugger automatically picks it up

✅ **Adding new clauses**
- Just add to clause registries
- Debugger displays them by name

✅ **Custom directive parsers**
- Debugger sees the final `Directive` structure
- No changes needed

#### When You MIGHT Need to Update the Debugger

⚠️ **Adding new `ClauseKind` variants**
- Update `src/debugger/stepper.rs` lines 200-212 (Directive conversion)
- Update `src/debugger/ast_display.rs` lines 30-40 (AST display)

⚠️ **Adding new fields to `Directive` or `Clause` structs**
- Update conversion logic in `stepper.rs`
- Update display logic in `ast_display.rs`

⚠️ **Adding Fortran support to debugger**
- Update `parse_pragma_prefix_static()` to handle `!$omp`, `!$acc`, etc.
- Add language detection in prefix parsing

### Testing Guidelines

When adding new directives/clauses, verify the debugger works:

```bash
# Test your new directive
cargo test --test debugger_integration

# Add a specific test (optional but recommended)
# Edit tests/debugger_integration.rs and add:
#[test]
fn test_my_new_directive() {
    let input = "#pragma omp my_new_directive my_clause(arg)";
    let config = DebugConfig::openmp();
    let session = DebugSession::new(input, config).expect("Failed to parse");

    let directive = session.final_directive.as_ref().unwrap();
    assert_eq!(directive.name, "my_new_directive");
    assert_eq!(directive.clauses.len(), 1);
}
```

### Code Organization

```
src/debugger/
├── mod.rs          # Public API
│   ├── DebugConfig    - Configuration (dialect, language)
│   ├── DebugError     - Error types
│   └── DebugResult<T> - Result type alias
│
├── stepper.rs      # Core stepping logic
│   ├── DebugSession   - Main orchestrator
│   ├── DebugStep      - Individual step data
│   └── StepKind       - Step categories
│
├── ast_display.rs  # Visualization
│   ├── display_ast_tree()     - Tree rendering
│   ├── display_compact()      - One-line summary
│   ├── display_step_info()    - Step details
│   ├── display_help()         - Help message
│   └── display_all_steps()    - Step history
│
└── ui.rs           # Interactive interface
    ├── UserCommand            - Command enum
    ├── run_interactive_session() - Interactive mode
    └── run_non_interactive()     - Batch mode
```

### Performance Considerations

The debugger is designed for **educational/debugging use**, not production:

- **Memory**: Stores all steps in memory (acceptable for single directives)
- **Speed**: Clones input string and directive structure (negligible for typical usage)
- **Optimization**: Not a concern - clarity and correctness are prioritized

For production parsing, use the standard parser API directly.

---

## Future Enhancements

### Planned Features

#### 1. **Fortran Support in Debugger**
Currently, the debugger's step-by-step instrumentation only handles C-style `#pragma` prefixes. Fortran sentinels (`!$omp`, `c$omp`, etc.) work in the main parser but not in the debugger's prefix parsing step.

**Implementation**:
- Extend `parse_pragma_prefix_static()` to detect and handle Fortran sentinels
- Add language-specific step descriptions

#### 2. **Web-Based UI**
Export debug sessions as JSON for web visualization:

```rust
pub fn export_session_json(&self) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&self)
}
```

**Benefits**:
- Interactive visualization in browser
- Syntax highlighting
- Step-through animations
- Shareable session URLs

#### 3. **Colorized Output**
Add terminal colors to highlight:
- Step kinds (different colors for pragma, directive, clause, etc.)
- Consumed text (green)
- Remaining text (dim)
- Errors (red)

**Implementation**: Use `termcolor` or `colored` crate

#### 4. **Breakpoints**
Set breakpoints on specific step kinds:

```bash
roup_debug --break-on clause '#pragma omp parallel shared(x)'
# Automatically pause when encountering clause steps
```

#### 5. **Diff Mode**
Compare two parsing sessions side-by-side:

```bash
roup_debug --diff '#pragma omp parallel' '#pragma acc parallel'
```

#### 6. **Performance Profiling**
Show time spent in each parsing step:

```bash
roup_debug --profile '#pragma omp parallel for collapse(2)'
# Output:
# Step 1 (Pragma Prefix): 12μs
# Step 2 (Directive Name): 8μs
# ...
```

#### 7. **Export Formats**
Support multiple export formats:
- JSON (for web UI)
- Markdown (for documentation)
- GraphViz DOT (for diagram generation)
- CSV (for analysis)

#### 8. **Replay Mode**
Save and replay debug sessions:

```bash
# Save session
roup_debug '#pragma omp parallel' --save session.roup

# Replay later
roup_debug --replay session.roup
```

### Contributing

Contributions are welcome! Areas where help is needed:

1. **Fortran support** - Add Fortran sentinel handling
2. **Web UI** - Build interactive web interface
3. **Documentation** - More examples and tutorials
4. **Tests** - Add tests for edge cases
5. **Performance** - Optimize for very large directives (uncommon but possible)

See the [main ROUP repository](https://github.com/ouankou/roup) for contribution guidelines.

---

## Appendix: Debugging Common Issues

### Issue 1: "Failed to parse pragma prefix"

**Symptom**: Error mentioning pragma prefix parsing failure

**Cause**: Input doesn't start with `#pragma omp` or `#pragma acc`

**Solution**:
```bash
# Ensure input has proper pragma prefix
roup_debug '#pragma omp parallel'  # ✓ Correct
roup_debug 'omp parallel'           # ✗ Missing #pragma
```

### Issue 2: Invalid directive name

**Symptom**: Parse error saying directive is unknown

**Cause**: Typo in directive name, or directive not supported by roup

**Solution**:
```bash
# Check directive name spelling
roup_debug '#pragma omp parallell'  # ✗ Typo
roup_debug '#pragma omp parallel'   # ✓ Correct

# Verify directive is supported
# See docs/OPENMP_SUPPORT.md or docs/OPENACC_SUPPORT.md
```

### Issue 3: Interactive mode not responding

**Symptom**: After entering command, nothing happens

**Cause**: Input not terminated with Enter key

**Solution**: Press Enter after typing command

### Issue 4: AST shows unexpected structure

**Symptom**: AST doesn't match expected structure

**Cause**: Parsing might be correct, but your expectation differs

**Solution**:
1. Use interactive mode to step through parsing
2. Check which step produces unexpected result
3. Verify against OpenMP/OpenACC specification
4. If parser is wrong, file an issue

### Issue 5: Too many/too few steps

**Symptom**: Expected different number of steps

**Cause**: Whitespace steps can vary based on input formatting

**Solution**: Focus on semantic steps (Directive, Clause) rather than total count

---

## Appendix: Sample Output

### Full Session Example

```bash
$ roup_debug '#pragma omp parallel shared(x, y) private(z)' --non-interactive
```

**Output**:

```
═════════════════════════════════════════════════════════════
                    All Parsing Steps
═════════════════════════════════════════════════════════════

1. Pragma Prefix - Parse pragma prefix
   Consumed: #pragma
   Prefix: "#pragma"

2. Skip Whitespace - Skip whitespace before directive

3. Directive Name - Parse directive name 'parallel'
   Consumed: parallel
   Directive: "parallel"

4. Skip Whitespace - Skip whitespace before clause

5. Clause Name - Parse clause name 'shared'
   Consumed: shared
   Clause: "shared"

6. Clause Arguments - Parse clause arguments 'x, y'
   Consumed: (x, y)
   Arguments: "x, y"

7. Skip Whitespace - Skip whitespace before clause

8. Clause Name - Parse clause name 'private'
   Consumed: private
   Clause: "private"

9. Clause Arguments - Parse clause arguments 'z'
   Consumed: (z)
   Arguments: "z"

10. Complete - Parsing complete
   Successfully parsed directive

═════════════════════════════════════════════════════════════


Final AST:
Directive
├─ name: "parallel"
├─ parameter: None
└─ clauses: [2]
   ├─ Clause
   │  ├─ name: "shared"
   │  └─ kind: Parenthesized("x, y")
   └─ Clause
      ├─ name: "private"
      └─ kind: Parenthesized("z")


Pragma string:
#pragma omp parallel shared(x, y) private(z)
```

---

## References

- [ROUP Main Documentation](../README.md)
- [OpenMP Support Matrix](OPENMP_SUPPORT.md)
- [OpenACC Support Matrix](OPENACC_SUPPORT.md)
- [Parser Architecture](../src/parser/README.md) (if exists)
- [OpenMP 5.2 Specification](https://www.openmp.org/specifications/)
- [OpenACC 3.3 Specification](https://www.openacc.org/specification)

---

## License

The ROUP debugger is part of the ROUP project and is licensed under the BSD-3-Clause license. See [LICENSE](../LICENSE) for details.

---

**Last Updated**: 2025-01-XX
**Author**: ROUP Development Team
**Contact**: https://github.com/ouankou/roup/issues
