# Agent Instructions

## Project Maturity Level

**IMPORTANT**: ROUP is in **experimental/development stage**. Do NOT use "production-ready" or similar terms.

- **Use**: "experimental", "development", "working prototype", "proof of concept", "beta"
- **Avoid**: "production-ready", "stable", "enterprise-grade", "battle-tested"
- **Status markers**: Use "⚠️ Experimental", "🚧 Under Development", "🧪 Beta"
- **Documentation tone**: Be clear about current limitations and ongoing development

## Documentation Philosophy

**IMPORTANT**: ROUP maintains a **single source of truth** for all documentation. Avoid redundancy.

- **No redundant docs**: If the same information exists in multiple places, consolidate it
- **Single canonical location**: Each piece of information should have ONE authoritative source
- **Cross-reference, don't duplicate**: Link to the canonical source instead of copying content
- **Assume no redundancy**: If you see a doc/README that seems redundant, it likely is - check if it can be deleted

**Documentation Hierarchy**:
1. **mdBook website** (`docs/book/src/`) - Primary user-facing documentation
2. **README.md** - Brief project intro with links to website
3. **API docs** (rustdoc) - Generated from source code
4. **Examples** - Working code in `examples/`
5. **Source comments** - Implementation details in code

**DO NOT**:
- ❌ Create duplicate guides (e.g., both `docs/QUICK_START.md` and `docs/book/src/getting-started.md`)
- ❌ Copy content between files - use links instead
- ❌ Maintain multiple versions of the same tutorial
- ❌ Keep historical/planning docs after completion (delete them)

**DO**:
- ✅ Consolidate overlapping documentation
- ✅ Use `[See Building Guide](./building.md)` instead of duplicating build instructions
- ✅ Delete planning docs, status files, and completed task lists
- ✅ Keep examples up-to-date with current API

## C FFI API Architecture

**IMPORTANT**: ROUP uses a **minimal unsafe pointer-based C API**, NOT a handle-based approach.

- **Current API**: Direct C pointers (`*mut OmpDirective`, `*mut OmpClause`) in `src/c_api.rs`
- **Pattern**: Standard malloc/free pattern familiar to C programmers
- **Functions**: 16 FFI functions (parse, free, query, iterate)
- **Clause Mapping**: Simple integer discriminants (0-11, not 0-91)
- **Safety**: ~60 lines of unsafe code (~0.9% of file), all at FFI boundary

**DO NOT**:
- ❌ Reference or create "handle-based API" with `Handle` types and global registry
- ❌ Use `omp_parse_cstr()`, `OmpStatus`, `INVALID_HANDLE` - these are from an old, deleted API
- ❌ Document enum mappings beyond the 12 clause types in `src/c_api.rs`

**DO**:
- ✅ Use `roup_parse()`, `roup_directive_free()`, `roup_clause_kind()` - the actual pointer API
- ✅ Reference `examples/c/tutorial_basic.c` for correct usage patterns
- ✅ Check `src/c_api.rs` for the source of truth on all C API functions

## Code Quality

- Consult the latest official OpenMP specification when making changes related to OpenMP parsing or documentation to ensure accuracy.
- Unsafe code is permitted ONLY at the FFI boundary in `src/c_api.rs`; all business logic must be safe Rust.
- Run `cargo fmt` (or `rustfmt`) to maintain consistent Rust formatting before submitting changes.
- **Always ensure warning-free builds**: All commits must pass without warnings:
  - `cargo fmt -- --check` - No formatting issues
  - `cargo build` - No compilation warnings
  - `cargo doc --no-deps` - No rustdoc warnings
  - `cargo test` - All tests pass
  - `mdbook build docs/book` - No documentation warnings (if applicable)

## Documentation Maintenance

- **Keep documentation synchronized**: After any code changes or commits:
  - Update relevant README.md files
  - Update docs/ directory content if APIs or features changed
  - Update code examples in documentation to match current API
  - Update RELEASE_NOTES.md for user-facing changes
  - Regenerate rustdoc if public APIs modified
  - Verify all documentation builds successfully
- **Check README.md after every change**: Ensure the main README.md and any sub-project READMEs don't conflict with new changes
  - If API changes, update README examples
  - If features added/removed, update README feature list
  - If build process changes, update README installation/build instructions
  - Keep README in sync with docs/book/src/ website content

## Pull Request & Git Workflow

- **PR Commit History**: When merging a PR, maintain a clean, logical commit history:
  - **Option 1 (Preferred for small changes)**: Squash all commits into a single logical commit with comprehensive message
  - **Option 2 (For larger features)**: Rewrite history to organize commits into several logical units (e.g., "feat: core implementation", "test: add test suite", "docs: add documentation")
  - **Avoid**: Keeping intermediate "fix typo", "address review comments", "WIP" commits in main branch history
  - Use interactive rebase (`git rebase -i`) to reorganize commits before merging
  - Each final commit should:
    - Be self-contained and functional
    - Have a clear, descriptive commit message following conventional commits format
    - Pass all tests independently
- **Commit Message Format**:
  ```
  <type>: <subject>
  
  <body>
  
  <footer>
  ```
  - Types: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `chore:`
  - Subject: Concise summary (50 chars or less)
  - Body: Detailed explanation of changes, motivation, and impact
  - Footer: Breaking changes, issue references

## Testing Requirements

**IMPORTANT**: ROUP has TWO critical components that BOTH require comprehensive testing:

### 1. Rust Core Library Testing

- **Location**: `tests/*.rs`, `src/*/tests`, inline `#[cfg(test)]` modules
- **Coverage areas**:
  - Lexer: Token parsing, sentinel detection, whitespace handling
  - Parser: Directive/clause parsing, error handling, edge cases
  - IR: Semantic validation, type checking, conversions
  - C API: FFI boundary, NULL handling, memory safety
- **Required tests for new features**:
  - Unit tests for individual functions/modules
  - Integration tests for end-to-end parsing
  - Edge cases: malformed input, boundary conditions, empty/null inputs
  - Regression tests for bug fixes
- **Test organization**:
  - Prefer `tests/*.rs` for integration tests
  - Use inline `#[cfg(test)]` for unit tests near implementation
  - Name tests descriptively: `parses_fortran_parallel_directive`

### 2. ompparser Compatibility Layer Testing

**CRITICAL**: The ompparser compatibility layer (`compat/ompparser/`) is a **first-class feature**, not an afterthought.

- **Location**: `compat/ompparser/tests/*.cpp`
- **Purpose**: Drop-in replacement for existing ompparser users
- **Coverage requirements**:
  - **All Rust features** must have equivalent ompparser compat tests
  - Test ompparser API functions match original ompparser behavior
  - Verify enum mappings (OpenMPDirectiveKind, OpenMPClauseKind)
  - Test memory management (allocation/deallocation)
  - Validate return values and error conditions
- **When adding new features**:
  1. ✅ Add Rust tests (`tests/*.rs`)
  2. ✅ Add ompparser compat tests (`compat/ompparser/tests/*.cpp`)
  3. ✅ Update compat layer implementation if needed (`compat/ompparser/src/compat_impl.cpp`)
  4. ✅ Document compat layer changes in `compat/ompparser/README.md`
- **Test execution**:
  ```bash
  # Rust tests
  cargo test
  
  # ompparser compat tests
  cd compat/ompparser/build
  make test  # or ctest
  ```

### Testing Checklist for New Features

When implementing new features (e.g., Fortran support, new directives):

- [ ] Rust unit tests added
- [ ] Rust integration tests added
- [ ] ompparser compat tests added (if feature exposed via compat layer)
- [ ] All tests pass: `cargo test`
- [ ] All compat tests pass: `cd compat/ompparser/build && make test`
- [ ] Test coverage includes edge cases
- [ ] Tests documented with clear descriptions
- [ ] Regression tests for any bug fixes

**DO NOT**:
- ❌ Implement features only in Rust without compat layer support
- ❌ Skip ompparser compat tests for "Rust-only" features
- ❌ Merge PRs without testing both components
- ❌ Assume compat layer "just works" without explicit testing

**DO**:
- ✅ Treat ompparser compat layer as equal priority to Rust core
- ✅ Test both components for every feature
- ✅ Keep compat layer tests in sync with Rust tests
- ✅ Document compat layer behavior and limitations
