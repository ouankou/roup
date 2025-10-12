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
