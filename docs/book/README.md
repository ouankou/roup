# Documentation build notes

The `docs/book` directory hosts the mdBook sources for https://roup.ouankou.com.

- Install mdBook with `cargo install mdbook`.
- Run `mdbook serve docs/book` for a live preview or `mdbook build docs/book`
  to produce static pages.
- Copy `target/doc` into `docs/book/book/api/` after running
  `cargo doc --locked --workspace --no-deps`
  to bundle the Rust API reference alongside the book.

The documentation job in `.github/workflows/ci.yml` tests both outputs, combines
them, and publishes the resulting artifact from `main`. The `book.toml`
configuration tracks the custom domain and other mdBook options.
