# Contributing Guide

Thank you for your interest in contributing to ROUP! This guide will help you get started.

---

## Quick Links

- **Source Code**: [github.com/ouankou/roup](https://github.com/ouankou/roup)
- **Issue Tracker**: [GitHub Issues](https://github.com/ouankou/roup/issues)
- **Discussions**: [GitHub Discussions](https://github.com/ouankou/roup/discussions)
- **Documentation**: [roup.ouankou.com](https://roup.ouankou.com)

---

## Ways to Contribute

### 1. Report Bugs

Found a bug? Please [open an issue](https://github.com/ouankou/roup/issues/new) with:

- **Clear title**: What's wrong?
- **Input**: The OpenMP directive that caused the issue
- **Expected**: What should happen?
- **Actual**: What actually happened?
- **Environment**: OS, Rust version, ROUP version

**Example:**
```text
Title: Parser fails on `collapse` clause with variable

Input: #pragma omp for collapse(n)
Expected: Parse successfully
Actual: Parse error: "Expected integer literal"
Environment: Ubuntu 22.04, Rust 1.75, ROUP 0.1.0
```text

### 2. Suggest Features

Have an idea? [Start a discussion](https://github.com/ouankou/roup/discussions) or open an issue with:

- **Use case**: Why is this needed?
- **Proposed API**: How would it work?
- **Alternatives**: Other ways to solve the problem?

### 3. Improve Documentation

Documentation improvements are always welcome:

- Fix typos or unclear explanations
- Add examples
- Improve error messages
- Translate documentation
- Write tutorials or blog posts

See [Documentation Updates](#documentation-updates) below.

### 4. Submit Code

Ready to code? See [Development Setup](#development-setup) and [Pull Request Process](#pull-request-process).

---

## Development Setup

### Prerequisites

- **Rust 1.85+** - [Install Rust](https://rustup.rs/)
- **Git** - Version control
- **mdBook** (optional) - For documentation: `cargo install mdbook`

### Clone and Build

```bash
# Clone repository
git clone https://github.com/ouankou/roup.git
cd roup

# Build library
cargo build

# Run tests
cargo test

# Build documentation
cargo doc --no-deps --open
```text

### Development Tools

**Recommended VS Code Extensions:**
- rust-analyzer
- Even Better TOML
- Error Lens

**Recommended CLI Tools:**
```bash
# Code formatter
rustup component add rustfmt

# Linter
rustup component add clippy

# Documentation builder
cargo install mdbook
```text

---

## Code Quality Standards

### Rust Code

#### 1. Use Safe Rust

**Rule**: Unsafe code is permitted ONLY at the FFI boundary in `src/c_api.rs`.

```rust
// ✅ GOOD: Safe Rust in parser
pub fn parse(input: &str) -> Result<DirectiveIR, ParseError> {
    // All safe code
}

// ❌ BAD: Unsafe in parser
pub fn parse(input: &str) -> Result<DirectiveIR, ParseError> {
    unsafe {  // ← Not allowed outside c_api.rs!
        // ...
    }
}
```text

#### 2. Format Your Code

```bash
# Format all code
cargo fmt

# Check formatting (CI uses this)
cargo fmt -- --check
```text

#### 3. Pass Clippy

```bash
# Run linter
cargo clippy

# CI requires no warnings
cargo clippy -- -D warnings
```text

#### 4. Write Tests

Every new feature or bug fix should include tests.

```rust
#[test]
fn test_new_feature() {
    let result = parse("#pragma omp my_new_directive");
    assert!(result.is_ok());
    // More assertions...
}
```text

#### 5. Document Public APIs

```rust
/// Parse an OpenMP directive from a string.
///
/// # Arguments
///
/// * `input` - The OpenMP directive text
///
/// # Returns
///
/// * `Ok(DirectiveIR)` - Parsed directive
/// * `Err(ParseError)` - Parse failure with location
///
/// # Examples
///
/// ```
/// use roup::parser::parse;
///
/// let directive = parse("#pragma omp parallel").unwrap();
/// assert_eq!(directive.clauses.len(), 0);
/// ```
pub fn parse(input: &str) -> Result<DirectiveIR, ParseError> {
    // ...
}
```text

### C API Code

If modifying `src/c_api.rs`:

- **Minimize unsafe blocks**: Only what's absolutely necessary
- **NULL checks**: Before every pointer dereference
- **Document safety contracts**: Explain caller obligations
- **Test thoroughly**: Including NULL inputs and edge cases

```rust
/// # Safety
///
/// Caller must ensure:
/// - `input` points to a valid null-terminated C string
/// - The string remains valid for the duration of this call
/// - The string is valid UTF-8
#[no_mangle]
pub extern "C" fn roup_parse(input: *const c_char) -> *mut OmpDirective {
    if input.is_null() {
        return std::ptr::null_mut();
    }
    
    // ... minimal unsafe code ...
}
```text

---

## Testing Guidelines

### Running Tests

```bash
# All tests
cargo test

# Specific test
cargo test test_parallel_directive

# Tests with output
cargo test -- --nocapture

# Tests in specific module
cargo test parser::
```text

### Test Categories

#### Unit Tests

Test individual functions in isolation.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_identifier() {
        let tokens = tokenize("parallel");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Identifier("parallel"));
    }
}
```text

#### Integration Tests

Test complete parsing workflows in `tests/`.

```rust
// tests/openmp_parallel.rs
#[test]
fn test_parallel_with_clauses() {
    let input = "#pragma omp parallel num_threads(4) private(x)";
    let result = roup::parser::parse(input);
    
    assert!(result.is_ok());
    let directive = result.unwrap();
    assert_eq!(directive.clauses.len(), 2);
}
```text

#### FFI Tests

Test C API safety and correctness.

```rust
#[test]
fn test_null_safety() {
    let dir = roup_parse(std::ptr::null());
    assert!(dir.is_null());
}
```text

### Test Coverage

Aim for:
- **90%+ coverage** for parser code
- **100% coverage** for FFI boundary code
- **All error paths** tested

---

## Documentation Updates

### mdBook Website

The main documentation is in `docs/book/src/`:

```text
docs/book/src/
├── SUMMARY.md           # Navigation (table of contents)
├── intro.md             # Homepage
├── getting-started.md   # Quick start guide
├── rust-tutorial.md     # Rust API tutorial
├── c-tutorial.md        # C API tutorial
├── cpp-tutorial.md      # C++ API tutorial
├── building.md          # Build instructions
├── api-reference.md     # API reference
├── architecture.md      # Internal design
├── openmp-support.md    # OpenMP support matrix
├── openacc-support.md   # OpenACC support matrix
├── contributing.md      # This file
└── faq.md              # Frequently asked questions
```text

#### Building Documentation

```bash
# Build website
cd docs/book
mdbook build

# Serve locally (with live reload)
mdbook serve --open

# View at http://localhost:3000
```text

#### Adding New Pages

1. Create `.md` file in `docs/book/src/`
2. Add to `SUMMARY.md`:
   ```markdown
   - [My New Page](./my-new-page.md)
   ```
3. Build and verify: `mdbook build`

### Rustdoc

API documentation is generated from source code:

```bash
# Generate API docs
cargo doc --no-deps --open

# With private items (for development)
cargo doc --no-deps --document-private-items --open
```text

### README.md

**IMPORTANT**: After any change, check that `README.md` stays in sync:

- API changes → Update README examples
- Feature changes → Update README feature list
- Build changes → Update README installation instructions

The README should match website content in `docs/book/src/`.

---

## Pull Request Process

### 1. Fork and Branch

```bash
# Fork on GitHub, then clone your fork
git clone https://github.com/YOUR_USERNAME/roup.git
cd roup

# Create feature branch
git checkout -b feature/my-awesome-feature
```text

### 2. Make Changes

- Write code
- Write tests
- Update documentation
- Format code: `cargo fmt`
- Run tests: `cargo test`
- Check lints: `cargo clippy`

### 3. Commit

Use clear, descriptive commit messages:

```bash
git commit -m "feat: add support for OpenMP 6.0 loop directive"
git commit -m "fix: handle null pointers in roup_parse"
git commit -m "docs: add examples for metadirective"
git commit -m "test: add tests for error recovery"
```text

**Commit Message Format:**
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation only
- `test:` - Tests only
- `refactor:` - Code refactoring
- `perf:` - Performance improvement
- `chore:` - Maintenance tasks

### 4. Pre-PR Checklist

Before opening a PR, ensure:

- [ ] `cargo fmt -- --check` passes (no formatting issues)
- [ ] `cargo build` passes (no compilation warnings)
- [ ] `cargo clippy` passes (no linter warnings)
- [ ] `cargo test` passes (all tests green)
- [ ] `cargo doc --no-deps` passes (no rustdoc warnings)
- [ ] `mdbook build docs/book` passes (if docs changed)
- [ ] README.md is in sync with changes
- [ ] New features have tests
- [ ] New features have documentation

### 5. Push and Open PR

```bash
# Push to your fork
git push origin feature/my-awesome-feature

# Open PR on GitHub
# Go to https://github.com/ouankou/roup and click "New Pull Request"
```text

### 6. PR Description

Include:

**What**: What does this PR do?

**Why**: Why is this change needed?

**How**: How does it work?

**Testing**: How was it tested?

**Example:**
```markdown
## What
Adds support for the OpenMP 6.0 `loop` directive.

## Why
OpenMP 6.0 introduced a new `loop` directive as a more generic alternative to `for`.

## How
- Added `Loop` variant to `DirectiveKind` enum
- Added parsing logic in `directive.rs`
- Updated OpenMP/OpenACC support matrices where applicable

## Testing
- Added 15 new test cases covering various `loop` directive forms
- All existing tests still pass
- Manually tested with real-world code
```text

### 7. Code Review

Maintainers will review your PR and may:

- Request changes
- Ask questions
- Suggest improvements

**Be patient and responsive!** Code review is a collaborative process.

### 8. Merge

Once approved, maintainers will merge your PR. Congratulations! 🎉

---

## OpenMP Specification Compliance

When adding support for new OpenMP features:

### 1. Consult Official Specs

- **OpenMP 6.0**: [Latest specification](https://www.openmp.org/specifications/)
- **Archive**: [Older versions](https://www.openmp.org/specifications/)

### 2. Check Syntax Carefully

OpenMP syntax can be subtle. Double-check:

- Required vs optional clauses
- Clause argument types
- Directive applicability (C/C++ vs Fortran)
- Version introduced

### 3. Update Support Matrix

After adding a directive/clause, update `docs/book/src/openmp-support.md`:

```markdown
| Directive | OpenMP Version | Status | Notes |
|-----------|----------------|--------|-------|
| `loop` | 5.0 | ✅ Supported | New in OpenMP 5.0 |
```text

### 4. Add Examples

Include examples in documentation showing correct usage.

---

## Performance Considerations

### Benchmarking

If your change affects performance:

```bash
# Run benchmarks (if available)
cargo bench

# Profile with flamegraph
cargo install flamegraph
cargo flamegraph --bin roup
```text

### Performance Guidelines

- Avoid unnecessary allocations
- Prefer zero-copy when possible
- Use `&str` instead of `String` where appropriate
- Benchmark before/after for significant changes

---

## Security

### Reporting Security Issues

Please report security vulnerabilities by opening a GitHub issue at:
[https://github.com/ouankou/roup/issues](https://github.com/ouankou/roup/issues)

Include:
- Description of vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Security Best Practices

When writing code:

- Validate all inputs
- Check for integer overflow
- Avoid buffer overruns
- Be careful with unsafe code
- Use safe defaults

---

## Release Process

(For maintainers)

### Version Numbering

ROUP follows [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking API changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

### Release Checklist

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Run full test suite
4. Build documentation
5. Create git tag: `git tag v0.2.0`
6. Push tag: `git push origin v0.2.0`
7. Publish to crates.io: `cargo publish`
8. Create GitHub release with notes

---

## Getting Help

### Stuck?

- **Documentation**: Read [roup.ouankou.com](https://roup.ouankou.com)
- **Discussions**: Ask on [GitHub Discussions](https://github.com/ouankou/roup/discussions)
- **Issues**: Search [existing issues](https://github.com/ouankou/roup/issues)
- **Examples**: Check `examples/` directory

### Communication Guidelines

- Be respectful and professional
- Provide context for questions
- Include minimal reproducible examples
- Search before asking (avoid duplicates)

---

## Code of Conduct

### Our Standards

- **Be respectful**: Treat everyone with respect
- **Be constructive**: Provide helpful feedback
- **Be patient**: Remember that everyone is learning
- **Be inclusive**: Welcome newcomers

### Unacceptable Behavior

- Harassment or discrimination
- Trolling or insulting comments
- Personal attacks
- Publishing others' private information

### Reporting

Report unacceptable behavior to: [conduct@ouankou.com](mailto:conduct@ouankou.com)

---

## Recognition

Contributors will be:

- Listed in `CONTRIBUTORS.md`
- Mentioned in release notes
- Credited in commit messages

Significant contributions may result in:

- Maintainer status
- Commit access
- Decision-making authority

---

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (see `LICENSE` file).

---

## Questions?

Still have questions? Open a [discussion](https://github.com/ouankou/roup/discussions) or reach out to the maintainers.

**Thank you for contributing to ROUP!** 🚀
