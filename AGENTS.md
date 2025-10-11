# Agent Instructions

## Code Quality

- Consult the latest official OpenMP specification when making changes related to OpenMP parsing or documentation to ensure accuracy.
- Only use safe Rust; adding `unsafe` code anywhere in the repository is prohibited.
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
