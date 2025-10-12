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

## Pull Request Comment Retrieval

**CRITICAL**: When the user says there are new comments on a PR, THEY EXIST. You MUST use ALL possible methods to find them.

### MANDATORY: USE ALL 15 METHODS EVERY TIME

**NEVER skip any method. ALWAYS run ALL methods. NO EXCEPTIONS.**

When user mentions PR comments, execute ALL of the following methods in order:

#### Method 1: GitHub Pull Request Tools (VSCode Extensions)
```
github-pull-request_activePullRequest
github-pull-request_openPullRequest
```

#### Method 2: GitHub CLI - PR View (Multiple Formats)
```bash
gh pr view <number> --comments
gh pr view <number> --json comments
gh pr view <number> --json reviews
gh pr view <number> --json comments,reviews,latestReviews
```

#### Method 3: GitHub API - Pull Request Comments (WITH PAGINATION)
```bash
# CRITICAL: Use --paginate to get ALL comments, not just first page
gh api /repos/{owner}/{repo}/pulls/<number>/comments --paginate
gh api /repos/{owner}/{repo}/pulls/<number>/comments --paginate --jq '.[]'
gh api /repos/{owner}/{repo}/pulls/<number>/comments --paginate --jq '[.[] | select(.commit_id == "<commit_hash>")]'
```

#### Method 4: GitHub API - Issue Comments (General PR Comments)
```bash
# General comments on the PR (not line-specific code review comments)
gh api /repos/{owner}/{repo}/issues/<number>/comments
gh api /repos/{owner}/{repo}/issues/<number>/comments --jq '.[]'
```

#### Method 5: GitHub API - Review Details
```bash
gh api /repos/{owner}/{repo}/pulls/<number>/reviews
gh api /repos/{owner}/{repo}/pulls/<number>/reviews --jq '.[]'
gh api /repos/{owner}/{repo}/pulls/<number>/reviews --jq '.[] | select(.submitted_at != null)'
```

#### Method 6: GitHub API - Review Comments by Review ID
```bash
# Get review IDs first, then get comments for each review
gh api /repos/{owner}/{repo}/pulls/<number>/reviews --jq '.[].id'
# Then for each review ID:
gh api /repos/{owner}/{repo}/pulls/<number>/reviews/<review_id>/comments
```

#### Method 7: GitHub API - Timeline Events
```bash
gh api /repos/{owner}/{repo}/issues/<number>/timeline
gh api /repos/{owner}/{repo}/issues/<number>/timeline --jq '.[] | select(.event == "reviewed" or .event == "commented")'
```

#### Method 8: GitHub API - Specific Commit Comments
```bash
# For each commit in the PR
gh api /repos/{owner}/{repo}/commits/<commit_sha>/comments
```

#### Method 9: Sort and Filter by Timestamp
```bash
# Get ALL comments and sort by creation time
gh api /repos/{owner}/{repo}/pulls/<number>/comments --paginate --jq '[.[] | {created: .created_at, updated: .updated_at, path: .path, line: .line, body: .body}] | sort_by(.created) | reverse'

# Get only comments after specific timestamp
gh api /repos/{owner}/{repo}/pulls/<number>/comments --paginate --jq '[.[] | select(.created_at > "2025-10-12T19:00:00Z")]'
```

#### Method 10: Check for Suppressed/Low-Confidence Comments
```bash
# Review bodies may mention suppressed comments
gh api /repos/{owner}/{repo}/pulls/<number>/reviews --jq '.[] | .body' | grep -i "suppressed\|low confidence"
```

#### Method 11: GraphQL API (More Comprehensive)
```bash
gh api graphql -f query='
query($owner: String!, $repo: String!, $number: Int!) {
  repository(owner: $owner, name: $repo) {
    pullRequest(number: $number) {
      reviews(first: 100) {
        nodes {
          body
          state
          createdAt
          comments(first: 100) {
            nodes {
              body
              path
              createdAt
            }
          }
        }
      }
      comments(first: 100) {
        nodes {
          body
          createdAt
        }
      }
      reviewThreads(first: 100) {
        nodes {
          comments(first: 100) {
            nodes {
              body
              path
              createdAt
            }
          }
        }
      }
    }
  }
}' -f owner=<owner> -f repo=<repo> -F number=<number>
```

#### Method 12: Web UI Scraping Fallback
```bash
# Open PR in browser to manually check for comments not visible via API
gh pr view <number> --web
```

#### Method 13: Git Fetch and Check Notes
```bash
git fetch origin
git fetch origin refs/pull/<number>/head
git log origin/pr/<number> --oneline
git notes list
```

#### Method 14: Check PR Files for Inline Comments
```bash
gh pr diff <number>
gh pr view <number> --json files --jq '.files[].filename'
```

#### Method 15: Cross-Reference Multiple Commit Hashes
```bash
# Get all commits in the PR
gh api /repos/{owner}/{repo}/pulls/<number>/commits --jq '.[].sha'

# For EACH commit SHA, check for comments
for sha in $(gh api /repos/{owner}/{repo}/pulls/<number>/commits --jq '.[].sha'); do
  echo "=== Checking commit $sha ==="
  gh api /repos/{owner}/{repo}/pulls/<number>/comments --paginate --jq ".[] | select(.commit_id == \"$sha\")"
done
```

### Execution Protocol

**YOU MUST RUN ALL 15 METHODS. NO SHORTCUTS.**

1. **Run methods 1-15 in sequence** - Do not skip any
2. **Collect ALL output** - Merge results from all methods
3. **Deduplicate comments** - Use comment ID or body hash to identify unique comments
4. **Sort by timestamp** - Most recent first
5. **Count total unique comments** - Report the number found
6. **Report per-method results** - Show what each method found

### Critical Rules

- ✅ **USE ALL 15 METHODS**: Run every single method, every single time
- ✅ **NEVER SKIP**: Even if method 1 finds comments, run methods 2-15
- ✅ **PAGINATE EVERYTHING**: Always use `--paginate` for API calls
- ✅ **CHECK ALL COMMIT HASHES**: Different commits may have different comments
- ✅ **VERIFY TIMESTAMPS**: Ensure you're seeing the latest comments
- ✅ **USER IS ALWAYS RIGHT**: If they say comments exist, keep searching
- ✅ **NEVER SAY "NO COMMENTS"**: Until ALL 15 methods complete

**DO NOT**:
- ❌ Stop after one method finds something
- ❌ Skip methods because "we have enough"
- ❌ Assume pagination isn't needed
- ❌ Only check the latest commit
- ❌ Ignore suppressed/low-confidence comments
- ❌ Give up before running all 15 methods

**DO**:
- ✅ Execute all 15 methods systematically
- ✅ Paginate all API calls
- ✅ Check every commit in the PR
- ✅ Merge and deduplicate results
- ✅ Report total count and per-method breakdown
- ✅ Persist until ALL methods complete

### Addressing Review Comments - MANDATORY Verification

**CRITICAL**: Before committing changes to address review comments, you MUST verify EVERY SINGLE comment is fully addressed.

**Pre-Commit Review Comment Checklist**:

1. **List ALL review comments** - Create a complete checklist:
   - Extract every review comment found (from all 5 methods above)
   - Number each comment for tracking
   - Include both code review comments AND general issue comments

2. **Address EVERY comment** - No exceptions:
   - Go through the list one by one
   - Implement the requested change or provide clear explanation why not
   - Mark each as addressed in your checklist

3. **Double-check completeness** - BEFORE committing:
   - Review your checklist: Are ALL items checked off?
   - Re-read each original comment: Did you actually do what was requested?
   - Verify no comments were missed or partially addressed

4. **Commit only when 100% complete**:
   - ✅ Every single feedback item addressed
   - ✅ All changes tested and working
   - ✅ No "TODO" or "will fix later" items remaining

**DO NOT**:
- ❌ Commit partial fixes with some comments unaddressed
- ❌ Skip "minor" comments thinking they don't matter
- ❌ Assume you addressed everything without explicit verification
- ❌ Leave any feedback item for "later" or "next commit"

**DO**:
- ✅ Create explicit checklist of ALL comments before starting
- ✅ Verify each item is addressed before marking complete
- ✅ Re-read original comments to ensure full compliance
- ✅ Commit only when 100% of feedback is addressed

## Code Quality

**CRITICAL PRE-COMMIT REQUIREMENTS - MUST BE DONE EVERY TIME**:

### Before EVERY Commit (No Exceptions)

**1. Code Formatting - MANDATORY**:
```bash
# Rust code - ALWAYS run before committing
cargo fmt

# Verify formatting is correct
cargo fmt --check

# C/C++ code (if modified) - use clang-format
clang-format -i compat/ompparser/**/*.cpp
clang-format -i compat/ompparser/**/*.h
clang-format -i examples/c/**/*.c

# Fortran code (if modified) - use fprettify or similar
# (if available in your environment)
```

**2. Run All Tests - MANDATORY**:
```bash
# Run all Rust tests
cargo test

# Run all ompparser compat tests (if modified)
cd compat/ompparser/build && make test
```

**3. Address ALL Warnings and Failures - MANDATORY**:
- ✅ **FIX IMMEDIATELY**: Never commit with warnings or failures
- ✅ **FIX PROPERLY**: No hardcoded fixes, no workarounds
- ✅ **NO DEFERRING**: Don't say "TODO", "pending", "later"
- ✅ **RESOLVE RIGHT AWAY**: Fix the root cause, not symptoms

**Why This Matters**:
- CI will fail if formatting is incorrect
- Tests catch regressions before they reach production
- Warnings indicate potential bugs or code smells
- Clean commits make reviews easier and faster

### Pre-Commit Checklist (Every Single Commit)

- [ ] `cargo fmt` executed successfully
- [ ] `cargo fmt --check` passes (no diff)
- [ ] `cargo test` passes (all tests green)
- [ ] `cargo build` completes with zero warnings
- [ ] All modified C/C++/Fortran code formatted
- [ ] No compiler warnings of any kind
- [ ] No test failures of any kind
- [ ] All issues resolved (not deferred)

### General Code Quality Guidelines

- Consult the latest official OpenMP specification when making changes related to OpenMP parsing or documentation to ensure accuracy.
- Unsafe code is permitted ONLY at the FFI boundary in `src/c_api.rs`; all business logic must be safe Rust.
- **Always ensure warning-free builds**: All commits must pass without warnings:
  - `cargo fmt -- --check` - No formatting issues
  - `cargo build` - No compilation warnings
  - `cargo doc --no-deps` - No rustdoc warnings
  - `cargo test` - All tests pass
  - `mdbook build docs/book` - No mdbook documentation warnings
  - `mdbook test docs/book` - All code examples in documentation work

## Documentation Generation & Testing

**IMPORTANT**: Documentation is a first-class deliverable. All documentation must build cleanly and examples must work.

### Required Documentation Tests

Before committing documentation changes, run ALL of these:

```bash
# 1. Build API documentation (rustdoc)
cargo doc --no-deps
# Check for warnings - output should be clean

# 2. Build mdBook documentation
mdbook build docs/book
# Check for warnings - should complete without errors

# 3. Test code examples in mdBook
mdbook test docs/book
# All Rust code examples in markdown must compile and run

# 4. Verify examples compile
cargo build --examples
# All example programs must build successfully

# 5. Check documentation links
# Manually verify cross-references work in generated docs
```

### Documentation Quality Checklist

When adding or modifying documentation:

- [ ] All code examples tested and working
- [ ] rustdoc builds without warnings (`cargo doc --no-deps`)
- [ ] mdBook builds without warnings (`mdbook build docs/book`)
- [ ] Code examples in tutorials pass (`mdbook test docs/book`)
- [ ] All examples/ programs compile (`cargo build --examples`)
- [ ] Cross-references and links verified
- [ ] API changes reflected in tutorials
- [ ] SUMMARY.md updated if new pages added
- [ ] Experimental status markers added where appropriate

### Common Documentation Issues

**Broken Code Examples**:
- ❌ Code in markdown that doesn't compile
- ❌ Using outdated API in examples
- ❌ Missing imports in code snippets
- ✅ Test all examples with `mdbook test`
- ✅ Use `rust,ignore` for pseudo-code only

**Stale Documentation**:
- ❌ Tutorial shows old API that was changed
- ❌ README examples use deprecated functions
- ✅ Update docs/ when changing public APIs
- ✅ Keep examples/ in sync with current API

**Missing Markdown Formatting**:
- ❌ Broken links to other pages
- ❌ Incorrect code fence language tags
- ✅ Use correct relative paths: `./building.md` not `building.md`
- ✅ Tag code blocks: ```rust, ```c, ```fortran, ```bash

### mdBook Configuration

The documentation is in `docs/book/`:
- `book.toml` - mdBook configuration
- `src/SUMMARY.md` - Table of contents (update when adding pages)
- `src/*.md` - Documentation pages

### Tools Available in Devcontainer

The devcontainer includes:
- `mdbook` - Documentation generator (auto-installed)
- `cargo doc` - Rust API documentation
- `rustdoc` - Documentation testing for Rust code

All documentation tools are pre-installed and ready to use.

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
- **Pre-Merge Documentation Audit**: Before final merge, check for documentation redundancy:
  - **Scan for duplicate content**: Review all documentation files for overlapping information
  - **Consolidate or remove**: Merge duplicate content into canonical locations or delete redundant files
  - **Check specific areas**:
    - Multiple README files with similar content
    - Duplicate tutorials or guides (e.g., `docs/QUICK_START.md` vs `docs/book/src/getting-started.md`)
    - Planning/status documents that should be deleted after completion
    - Old summary files (e.g., `FORTRAN_SUPPORT_SUMMARY.md`) that duplicate information in other docs
  - **Apply documentation hierarchy**: Ensure content lives in the right place per the Documentation Philosophy section
  - **Cross-reference instead of duplicate**: Replace duplicated content with links to canonical source
  - **Clean up temporary files**: Remove implementation summaries, status files, and planning documents after merging

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
