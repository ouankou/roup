# Devcontainer for roup

This folder contains the configuration used by VS Code Remote - Containers and GitHub Codespaces to build a reproducible development environment for this repository.
Key pieces installed by the devcontainer:

- Base: Ubuntu 24.04 (from mcr.microsoft.com/devcontainers/base)
- LLVM / Clang: version 21 (installed via the official llvm.sh helper)
- Rust: 1.90 (installed via rustup, with rust-src, rustfmt and clippy components)

How to use
1. Open the repository in VS Code and choose "Reopen in Container" (Remote - Containers) or create a Codespace. The devcontainer will be built automatically using the files in this folder.
2. The build uses the `Dockerfile` in this folder and accepts two build arguments: `LLVM_VERSION` and `RUST_VERSION`. Defaults are set in `devcontainer.json`.
3. After setup the container runs a simple verification (`clang --version`, `rustc --version`, `cargo --version`) as the `postCreateCommand`.

Overriding versions
If you want a different LLVM or Rust version when building locally, provide build args to docker build or change the values in `.devcontainer/devcontainer.json`:

```bash
# example: build locally with different versions
docker build -t roup-devcontainer -f .devcontainer/Dockerfile --build-arg LLVM_VERSION=21 --build-arg RUST_VERSION=1.90 ..
```

Notes and troubleshooting
- The `llvm.sh` helper adds the appropriate apt repository for the detected Ubuntu codename and installs the requested LLVM packages. The `all` argument installs the full toolchain — this can increase image size.
- If you see empty `LLVM_VERSION` or `RUST_VERSION` in a running shell, rebuild the container with the build args (ARGs are build-time; we export them to `remoteEnv` so runtime shells should see them).
- To shrink the image, consider installing a smaller subset of LLVM packages instead of `all`.

Contact / changes
If you want different defaults (e.g., LLVM 22 or a newer Rust), update `.devcontainer/devcontainer.json` and the `Dockerfile` build args.

---

(This file was updated to reflect the current devcontainer configuration.)
This devcontainer builds an environment for the `roup` repository with:

- Ubuntu 24.04 (base image)
- LLVM/Clang 20 (clang-20, clang++-20, lldb-20, clang-tidy-20)
- Rust 1.90.0 installed via rustup

How to use

1. Open this repository in Codespaces or in VS Code using Remote - Containers.
2. The container will be built from the included `Dockerfile` and selected automatically by `devcontainer.json`.
3. After creation the container runs `clang --version` and `rustc --version` as a quick check.

Notes

- If you prefer a different default Rust toolchain, change the version in the `Dockerfile`.
- The container installs `rustfmt` and `clippy` components for common Rust tooling.
