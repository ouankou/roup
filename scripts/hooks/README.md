# Git hooks

`pre-push` invokes the same fail-fast `test.sh` gate used by Linux CI. Install
the repository hook with `scripts/install-git-hooks.sh` only when all required
Rust and native toolchains are available locally.

The gate never initializes submodules, installs dependencies, or skips a test
category. A missing prerequisite stops the push with an error.
