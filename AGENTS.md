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

## Safe Command Execution with Complex Strings

**CRITICAL**: When using `run_in_terminal` with complex strings or special symbols, ALWAYS use file-based approaches to avoid bash hangs or parsing issues.

### The Problem

Direct heredoc or complex string literals in commands can cause:
- ❌ Bash hangs waiting for input
- ❌ Quote escaping nightmares
- ❌ Special character parsing failures
- ❌ Multiline string corruption
- ❌ Tool cancellation by the system

### The Solution: File-Based Approach

**ALWAYS use temporary files for**:
- Long commit messages
- PR descriptions with markdown
- Multi-paragraph text
- Strings with special characters: `$`, `` ` ``, `"`, `'`, `\`, `!`
- JSON or structured data
- Any text over ~200 characters

### Safe Pattern

```bash
# ✅ CORRECT: Use file-based approach
cat > /tmp/message.txt << 'EOF'
Your complex message here
With $pecial characters
And "quotes" and 'apostrophes'
EOF

# Then use the file
git commit -F /tmp/message.txt
gh pr create --body-file /tmp/message.txt
```

```bash
# ❌ WRONG: Direct heredoc in command (will hang!)
git commit -F - << 'EOF'
Message here
EOF
```

```bash
# ❌ WRONG: Complex string in --body argument (will fail!)
gh pr create --body "Very long text with $vars and \"quotes\""
```

### Examples of File-Based Commands

**Commit Messages**:
```bash
cat > /tmp/commit_msg.txt << 'EOFMSG'
feat: your changes

Detailed explanation with special chars: $var, "quotes", etc.
EOFMSG
git commit -F /tmp/commit_msg.txt
```

**PR Descriptions**:
```bash
cat > /tmp/pr_description.md << 'EOFPR'
## Summary
Your markdown here with **bold** and `code`
EOFPR
gh pr create --body-file /tmp/pr_description.md
```

**JSON Data**:
```bash
cat > /tmp/data.json << 'EOFJSON'
{"key": "value with special chars: $, \", etc."}
EOFJSON
curl -d @/tmp/data.json https://api.example.com
```

### Critical Rules

- ✅ **USE FILES**: For any string over 200 chars or with special symbols
- ✅ **USE UNIQUE EOF MARKERS**: `EOFMSG`, `EOFPR`, `EOFJSON` (not just `EOF`)
- ✅ **QUOTE EOF MARKERS**: Use `'EOF'` not `EOF` to prevent variable expansion
- ✅ **VERIFY FILE FIRST**: `cat /tmp/file.txt` before using it
- ✅ **USE /tmp/**: Temporary files are automatically cleaned up

- ❌ **NEVER use `-F -` or `--body` with complex strings**
- ❌ **NEVER embed heredocs directly in command arguments**
- ❌ **NEVER use unquoted EOF markers** (allows `$var` expansion)

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
  - **Update version numbers**: After any release, update all `roup = "X.Y"` version references in documentation to match `Cargo.toml`
  - Update RELEASE_NOTES.md for user-facing changes
  - Regenerate rustdoc if public APIs modified
  - Verify all documentation builds successfully
- **Check README.md after every change**: Ensure the main README.md and any sub-project READMEs don't conflict with new changes
  - If API changes, update README examples
  - If features added/removed, update README feature list
  - If build process changes, update README installation/build instructions
  - Keep README in sync with docs/book/src/ website content

## Pull Request & Git Workflow

**CRITICAL**: Follow this EXACT sequence. NEVER skip steps. NEVER merge before completing ALL verification.

### PR Workflow - MANDATORY STEPS IN ORDER

#### Step 1: Check ALL PR Comments (MANDATORY)

**CRITICAL**: "ALL comments" means EVERYTHING - review comments, issue comments, general discussions, even "hello" messages.

**Comment Retrieval Methods** - Use ALL 15 methods (see "Pull Request Comment Retrieval" section):
1. GitHub Pull Request Tools (VSCode extensions)
2. `gh pr view <number> --comments`
3. `gh api /repos/{owner}/{repo}/pulls/<number>/comments --paginate`
4. `gh api /repos/{owner}/{repo}/issues/<number>/comments` (general PR comments)
5. `gh api /repos/{owner}/{repo}/pulls/<number>/reviews`
6. Review comments by review ID
7. Timeline events
8. Specific commit comments
9. Sort and filter by timestamp
10. Check for suppressed/low-confidence comments
11. GraphQL API (comprehensive)
12. Web UI fallback
13. Git fetch and check notes
14. PR files for inline comments
15. Cross-reference multiple commit hashes

**Comment Types to Check**:
- ✅ Code review comments (inline file comments)
- ✅ General PR comments (issue comments)
- ✅ Review summary comments
- ✅ Discussion threads
- ✅ Questions or clarifications
- ✅ Suggestions or ideas
- ✅ Even casual messages like "LGTM", "thanks", "hello"
- ✅ Bot comments (CI results, analysis tools)
- ✅ Suppressed or low-confidence comments

**Create Comment Checklist**:
```
Comment Tracking Checklist for PR #<number>
============================================

[ ] Comment 1 (from @user, created_at): "Fix the use-after-free bug"
    Status: Not addressed yet
    
[ ] Comment 2 (from @bot, created_at): "CI passed on commit abc123"
    Status: Informational only, no action needed
    
[ ] Comment 3 (from @reviewer, created_at): "Looks good!"
    Status: Acknowledged, no action needed

... (continue for ALL comments)

Total comments found: X
Comments requiring action: Y
Comments addressed: Z
```

#### Step 2: Address ALL Comments (MANDATORY)

**For EACH comment that requires action**:

1. **Read the comment carefully** - understand what's being requested
2. **Implement the fix or change** - make the necessary code changes
3. **Test the fix** - ensure it works correctly
4. **Mark as addressed** - check it off your checklist
5. **Reply if needed** - acknowledge or explain your approach

**NEVER**:
- ❌ Skip any comment, even minor ones
- ❌ Say "will fix later" or "TODO"
- ❌ Assume you can merge with unaddressed comments
- ❌ Commit partial fixes

**ALWAYS**:
- ✅ Address 100% of comments before committing
- ✅ Re-read each comment after making changes to verify full compliance
- ✅ Mark each comment as addressed in your checklist

#### Step 3: Run ALL Quality Checks (MANDATORY)

**Code Formatting**:
```bash
# Rust formatting
cargo fmt
cargo fmt --check  # Verify no diff

# C/C++ formatting (if modified)
clang-format -i compat/ompparser/**/*.cpp
clang-format -i compat/ompparser/**/*.h
clang-format -i examples/c/**/*.c
```

**Build and Compilation**:
```bash
# Clean build
cargo clean
cargo build

# Check for warnings (MUST be zero)
cargo build 2>&1 | grep warning
# Output should be empty!
```

**Tests**:
```bash
# All Rust tests
cargo test

# All ompparser compat tests
cd compat/ompparser/build
make test
cd ../../..
```

**Documentation**:
```bash
# Build API docs
cargo doc --no-deps

# Build mdBook
mdbook build docs/book

# Test code examples in docs
mdbook test docs/book

# Verify examples compile
cargo build --examples
```

**CI Checks**:
```bash
# Check CI status on GitHub
gh pr checks <pr-number>

# View detailed CI results
gh pr view <pr-number> --json statusCheckRollup
```

**Quality Checklist**:
```
Quality Verification Checklist
==============================

Formatting:
[ ] cargo fmt executed
[ ] cargo fmt --check passes (no diff)
[ ] C/C++ code formatted (if applicable)

Build:
[ ] cargo clean && cargo build succeeds
[ ] Zero compiler warnings
[ ] cargo build --examples succeeds

Tests:
[ ] cargo test passes (all tests green)
[ ] compat/ompparser tests pass
[ ] No test failures

Documentation:
[ ] cargo doc --no-deps succeeds
[ ] mdbook build docs/book succeeds
[ ] mdbook test docs/book passes
[ ] All examples compile

CI:
[ ] All GitHub Actions checks pass
[ ] No failing workflows
```

#### Step 4: Pre-Merge Documentation Audit (MANDATORY)

**CRITICAL**: Check for documentation redundancy and ensure single source of truth BEFORE rewriting commits.

**Documentation checks may reveal needed changes - if so, make changes and START OVER from Step 1.**

**Check for documentation redundancy**:

- [ ] Scan for duplicate content in multiple files
- [ ] Consolidate overlapping documentation
- [ ] Delete planning/status docs after completion
- [ ] Check README.md synced with docs/book/src/
- [ ] Verify no duplicate tutorials or guides
- [ ] Remove temporary implementation summaries
- [ ] Ensure single source of truth per Documentation Philosophy
- [ ] Update version numbers in docs to match Cargo.toml
- [ ] Verify API examples in docs match current API

**If documentation issues found**: Fix them, commit, push, then START OVER from Step 1.

**If no issues found**: Proceed to Step 5.

#### Step 5: Rewrite PR Commit History (MANDATORY)

**CRITICAL**: After documentation audit, ALWAYS rewrite PR commits into clean, logical commits. Choose ONE of two approaches:

---

**CHOICE 1: Single Feature Commit** (if all changes belong to one logical feature)

Use when the PR implements a single cohesive feature or fix.

```bash
# 1. On PR branch, squash all commits into one
git checkout <pr-branch-name>
git reset --soft main

# 2. Create ONE clean commit with comprehensive message
cat > /tmp/commit_msg.txt << 'EOFMSG'
feat: descriptive title (50 chars max)

Detailed explanation of what this accomplishes and why.

**Changes**:
- Specific change 1
- Specific change 2
- Specific change 3

**Comments Addressed**:
- Fixed use-after-free bug (Comment #1 from @reviewer)
- Updated documentation (Comment #3 from @reviewer)

**Test Results**:
- ✅ All tests passing
- ✅ All CI checks green
- ✅ Zero warnings

Summary of impact and rationale.
EOFMSG

git add -A
git commit -F /tmp/commit_msg.txt

# 3. ⚠️ CRITICAL: Force push to update PR on GitHub!
#    WITHOUT THIS STEP, PR will still show old history on web!
git push --force-with-lease origin <pr-branch-name>

# 4. VERIFY the new history is visible on GitHub web UI
gh pr view <pr-number> --json commits --jq '.commits[] | "\(.oid[0:7]) \(.messageHeadline)"'
# Also check PR page on GitHub to confirm clean history
```

---

**CHOICE 2: Multiple Logical Commits** (if PR has multiple independent components)

Use when the PR contains distinct logical changes that should be separate commits.

```bash
# 1. On PR branch, reset to start fresh
git checkout <pr-branch-name>
git reset --soft main

# 2. Stage and commit each logical component separately

# First logical component
git add src/feature1.rs tests/test_feature1.rs
cat > /tmp/commit1_msg.txt << 'EOFMSG'
feat: implement feature 1

Detailed explanation of feature 1.

**Changes**:
- Added feature1.rs
- Added tests for feature 1

**Addresses**: Comment #1 from @reviewer
EOFMSG
git commit -F /tmp/commit1_msg.txt

# Second logical component
git add src/feature2.rs tests/test_feature2.rs
cat > /tmp/commit2_msg.txt << 'EOFMSG'
fix: resolve bug in feature 2

Detailed explanation of the fix.

**Changes**:
- Fixed bug in feature2.rs
- Updated tests

**Addresses**: Comment #2 from @reviewer
EOFMSG
git commit -F /tmp/commit2_msg.txt

# Third logical component (documentation)
git add docs/ README.md
cat > /tmp/commit3_msg.txt << 'EOFMSG'
docs: update documentation for new features

Updated all relevant documentation.

**Changes**:
- Updated API docs
- Updated README examples

**Addresses**: Comment #3 from @reviewer
EOFMSG
git commit -F /tmp/commit3_msg.txt

# 3. ⚠️ CRITICAL: Force push to update PR on GitHub!
#    WITHOUT THIS STEP, PR will still show old history on web!
git push --force-with-lease origin <pr-branch-name>

# 4. VERIFY the new history is visible on GitHub web UI
gh pr view <pr-number> --json commits --jq '.commits[] | "\(.oid[0:7]) \(.messageHeadline)"'
# Also check PR page on GitHub to confirm clean history
```

---

**Commit Message Format** (for both choices):
```
<type>: <subject (50 chars max)>

<body - detailed explanation>

**Changes**:
- Change 1
- Change 2

**Comments Addressed**:
- Comment summary (from @user)

**Test Results**:
- ✅ cargo test passing
- ✅ CI checks green

<footer - breaking changes, issue refs>
```

**Commit Types**: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `chore:`

**Critical Rules**:
- ✅ **ALWAYS** rewrite commits before merge (no exceptions)
- ✅ **ALWAYS** use file-based commit messages (`-F /tmp/msg.txt`)
- ✅ **NEVER** merge without clean commit history
- ✅ **NEVER** use interactive rebase (`git rebase -i` - will hang!)
- ✅ Each commit must be self-contained and buildable
- ✅ Commit messages must explain WHY, not just WHAT

#### Step 6: Run EVERY SINGLE TEST (MANDATORY - NO EXCEPTIONS)

**CRITICAL**: After documentation audit and commit rewriting, run EVERY test possible. Do NOT skip ANY test. Do NOT make changes - ONLY run tests.

**If ANY test fails, fix it and START OVER from Step 1 (not just Step 6).**

```bash
# ===================================================================
# RUN ALL THESE TESTS - DO NOT SKIP A SINGLE ONE
# ===================================================================

# 1. Code Formatting Check
echo "=== 1. Formatting Check ==="
cargo fmt --check
# MUST pass with no diff

# 2. Rust Build (all targets)
echo "=== 2. Rust Build ==="
cargo build --all-targets
cargo build --release --all-targets
# MUST complete with ZERO warnings (ignoring harmless build.rs info)

# 3. Rust Unit Tests
echo "=== 3. Rust Unit Tests ==="
cargo test --lib
# ALL unit tests MUST pass

# 4. Rust Integration Tests
echo "=== 4. Rust Integration Tests ==="
cargo test --tests
# ALL integration tests MUST pass

# 5. Rust Doc Tests
echo "=== 5. Rust Doc Tests ==="
cargo test --doc
# ALL doc tests MUST pass

# 6. All Rust Tests Together
echo "=== 6. All Rust Tests ==="
cargo test --all-targets
# Everything together MUST pass

# 7. Rust Examples Build
echo "=== 7. Examples Build ==="
cargo build --examples
# ALL examples MUST compile

# 8. Rust Documentation Build
echo "=== 8. Rust Documentation ==="
cargo doc --no-deps
# MUST build with ZERO warnings

# 9. ompparser Compatibility Tests
echo "=== 9. ompparser Compat Tests ==="
cd compat/ompparser/build
make test
# or: ctest
cd ../../..
# ALL compat tests MUST pass

# 10. mdBook Documentation Build
echo "=== 10. mdBook Documentation ==="
mdbook build docs/book
# MUST build with ZERO warnings

# 11. mdBook Code Examples Test
echo "=== 11. mdBook Code Examples ==="
mdbook test docs/book
# ALL code examples in docs MUST work

# 12. C Examples Build (if modified)
echo "=== 12. C Examples ==="
cd examples/c
make clean && make
cd ../..
# ALL C examples MUST compile

# 13. Fortran Examples Build (if modified)
echo "=== 13. Fortran Examples ==="
cd examples/fortran
make clean && make
cd ../..
# ALL Fortran examples MUST compile

# 14. Check for ANY compiler warnings
echo "=== 14. Warning Check ==="
cargo build 2>&1 | grep -i warning | grep -v "build.rs"
# Output MUST be empty (no warnings except build.rs)

# 15. Verify CI Status on GitHub
echo "=== 15. CI Status Check ==="
gh pr checks <pr-number>
# ALL CI checks MUST be GREEN (successful)

# ===================================================================
# SUMMARY CHECK
# ===================================================================
echo ""
echo "=== FINAL VERIFICATION ==="
echo "Did ALL 15 test categories pass? [YES/NO]"
# You MUST answer YES before proceeding to Step 7
```

**Test Results Checklist**:
```
Required Test Results (ALL must pass)
======================================

Formatting & Build:
[ ] cargo fmt --check (no diff)
[ ] cargo build --all-targets (zero warnings)
[ ] cargo build --release --all-targets (zero warnings)

Rust Tests:
[ ] cargo test --lib (all unit tests pass)
[ ] cargo test --tests (all integration tests pass)
[ ] cargo test --doc (all doc tests pass)
[ ] cargo test --all-targets (everything passes)
[ ] cargo build --examples (all examples compile)

Documentation:
[ ] cargo doc --no-deps (zero warnings)
[ ] mdbook build docs/book (zero warnings)
[ ] mdbook test docs/book (all examples work)

Compatibility:
[ ] compat/ompparser tests (all pass)
[ ] examples/c compile (if applicable)
[ ] examples/fortran compile (if applicable)

CI:
[ ] gh pr checks - ALL GREEN
[ ] No failing workflows
[ ] Latest commit has all checks passing
```

**CRITICAL RULES**:
- ✅ Run ALL 15 test categories - NO EXCEPTIONS
- ✅ If ANY test fails, fix it and START OVER from Step 1
- ✅ Do NOT skip any test category
- ✅ Do NOT proceed to Step 7 unless ALL tests pass
- ✅ Tests must pass on the rewritten PR branch BEFORE merge

#### Step 7: Rebase Merge (MANDATORY - ONLY After All Tests Pass)

**CRITICAL**: This is the FINAL step. Do NOT execute until Steps 4, 5, and 6 are 100% complete.

**Pre-Merge Verification**:
1. ✅ Documentation audited, no redundancy (Step 4 completed)
2. ✅ PR commits rewritten into clean logical commits (Step 5 completed)
3. ✅ ALL 15 test categories passed (Step 6 completed)
4. ✅ User explicitly approves merge

```bash
# DO NOT run this command until:
# 1. Step 4 complete: Documentation audit done
# 2. Step 5 complete: PR commits rewritten (Choice 1 or 2)
# 3. Step 6 complete: ALL tests passing (all 15 categories)
# 4. User says "yes" to proceed

# Ask user: "Docs audited. All commits rewritten. All tests passing. Ready to rebase merge PR #<number>. Proceed? (yes/no)"
# ONLY after user says "yes":

gh pr merge <pr-number> --rebase --delete-branch
```

**What this command does**:
1. Rebases the clean PR commits onto main (linear history)
2. Fast-forwards main to include the rebased commits
3. Closes the PR with proper merge tracking
4. Deletes the remote and local PR branch
5. Switches to main and updates it

**This is the ONLY correct merge method.**

**CRITICAL RULES**:
- ✅ **ONLY** use `gh pr merge --rebase --delete-branch`
- ✅ **NEVER** commit directly to main
- ✅ **NEVER** use regular merge (`git merge`)
- ✅ **NEVER** use GitHub UI buttons ("Squash and merge", "Merge commit")
- ✅ **ONLY** rebase merge from PR after Steps 4, 5, and 6 complete

### The Complete 4-Step Final Workflow

**Summary of the ONLY correct approach**:

1. **Step 4: Documentation Audit**
   - Check for redundancy and ensure single source of truth
   - If issues found → fix and START OVER from Step 1
   - Verify documentation is clean

2. **Step 5: Rewrite Commits** 
   - Choose: Single commit (Choice 1) OR Multiple logical commits (Choice 2)
   - Force push rewritten branch
   - Verify clean commit history

3. **Step 6: Run EVERY Test**
   - Execute ALL 15 test categories
   - If ANY fails → fix and START OVER from Step 1
   - Verify ALL tests pass

4. **Step 7: Rebase Merge**
   - Get user confirmation
   - Execute: `gh pr merge --rebase --delete-branch`
   - **ONLY** method allowed - no alternatives

**Result**: Main branch has clean, logical, linear history with all tests passing.

### Forbidden Merge Methods

**❌ NEVER USE THESE - ONLY USE Step 7 (`gh pr merge --rebase --delete-branch`)**:

- ❌ `git commit` directly to main branch
- ❌ `git merge <pr-branch>` then push to main
- ❌ `git rebase -i` (interactive rebase - will hang!)
- ❌ GitHub UI "Squash and merge" button
- ❌ GitHub UI "Merge commit" button  
- ❌ `git cherry-pick` to main then close PR
- ❌ Any method other than: **Step 4 → Step 5 → Step 6 → Step 7**

**Why these are WRONG**:
- Don't properly close/track PR
- Create messy commit history
- Skip the 3-step verification process
- Risk duplicate commits or conflicts
- Manual cleanup required



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
