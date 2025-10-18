# Documentation build notes

The `docs/book` directory hosts the mdBook sources for https://roup.ouankou.com.

- Install mdBook with `cargo install mdbook`.
- Run `mdbook serve` for a live preview or `mdbook build` to produce static
  pages.
- Copy `target/doc` into `docs/book/book/api/` after running `cargo doc --no-deps`
  to bundle the Rust API reference alongside the book.

GitHub Actions (`docs.yml`) builds both outputs on `main` and publishes them to
GitHub Pages. The `book.toml` configuration tracks the custom domain and other
mdBook options.
