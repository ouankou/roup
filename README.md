# Rust-based OpenMP/OpenACC Unified Parser (ROUP) 
ROUP is a standalone, unified parser for OpenMP and OpenACC, developed using Rust. It is designed as an extensible framework that can be expanded to support additional directive-based programming interfaces. For each language, ROUP provides a unique set of grammars and intermediate representations (IRs).

## Build and Run
```bash
cargo build
cargo run
```

## Output Example
```bash
(ssh)ouankou@dehya [ roup@main ] $ cargo build
    Updating crates.io index
     Locking 4 packages to latest compatible versions
   Compiling memchr v2.7.4
   Compiling minimal-lexical v0.2.1
   Compiling nom v7.1.3
   Compiling roup v0.1.0 (/home/ouankou/Projects/roup)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.98s

(ssh)ouankou@dehya [ roup@main ] $ cargo run
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
     Running `target/debug/tester`
Directive:
Clause: private
Clause: private
```

## Devcontainer

This repository includes a devcontainer configuration under `.devcontainer/` which provides a reproducible development environment.

- The devcontainer image installs LLVM and, by default, the Rust toolchain via `rustup`.
- If you want the container to always include Rust (recommended), rebuild the container so the `.devcontainer/Dockerfile` runs the rustup installer.

To rebuild the devcontainer in VS Code: open the Command Palette and run "Dev Containers: Rebuild Container".

If you need to opt-out of installing Rust (not recommended), the Dockerfile respects the build argument `INSTALL_RUST` (defaults to `true`). To build without Rust set `INSTALL_RUST=false` when building the image.

The Dockerfile sets the default Rust toolchain to 1.90. You can override this by changing the `RUST_VERSION` build argument in `.devcontainer/devcontainer.json` or when building.

