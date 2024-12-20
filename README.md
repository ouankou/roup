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

