# Documentation build notes

The `docs/book` directory hosts the mdBook sources for https://roup.ouankou.com.

- Install mdBook with `cargo install mdbook`.
- Run `mdbook serve docs/book` for a live preview or `mdbook build docs/book`
  to produce static pages.
- Copy `target/doc` into `docs/book/book/api/` after running
  `cargo doc --locked --workspace --no-deps`
  to bundle the Rust API reference alongside the book.

The documentation job in `.github/workflows/ci.yml` tests both outputs and
combines them on every CI run. A push to `main` additionally configures Pages
and uploads the combined tree as the `github-pages` artifact; the dedicated
deployment job publishes that artifact through the protected `github-pages`
environment. CI never writes a generated `gh-pages` branch. The `book.toml`
configuration tracks the custom domain and other mdBook options.
