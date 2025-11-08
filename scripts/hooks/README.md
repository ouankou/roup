# Git Hooks for ROUP Development

This directory contains git hooks that help maintain code quality and prevent broken code from being pushed.

## Available Hooks

### pre-push

Runs the comprehensive test suite (`./test.sh`) before allowing a push to proceed.

- **What it does**: Executes all 23 test categories including formatting, builds, tests, examples, documentation, and round-trip validation
- **Why it matters**: Ensures CI won't fail due to preventable issues
- **Duration**: Typically 2-5 minutes depending on system
- **Failure handling**: If any tests fail, the push is aborted and you'll see which tests failed

## Installation

To install the git hooks on your local repository:

```bash
./scripts/install-git-hooks.sh
```

This only needs to be done once per clone.

## Manual Installation

If you prefer manual installation:

```bash
cp scripts/hooks/pre-push .git/hooks/pre-push
chmod +x .git/hooks/pre-push
```

## Bypassing Hooks (Not Recommended)

In rare cases where you need to push without running tests (not recommended):

```bash
git push --no-verify
```

**Warning**: Only use this if you're absolutely sure the tests will pass on CI.

## Adding New Hooks

1. Create the hook script in `scripts/hooks/`
2. Make it executable: `chmod +x scripts/hooks/your-hook`
3. Update `install-git-hooks.sh` to install it
4. Document it in this README
