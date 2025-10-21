# Documentation build notes

The `docs/book` directory hosts the mdBook sources deployed to <https://roup.ouankou.com>.

## Local preview

```bash
cargo install mdbook
mdbook serve docs/book --open
```

Use `mdbook build docs/book` for a static build. The generated HTML lives in `docs/book/book/`.

## API docs

Run `cargo doc --no-deps --all-features` to refresh the Rust API reference. Copy the output from `target/doc/` into `docs/book/book/api/` before publishing if you want the API reference bundled with the book.

GitHub Actions builds both outputs on `main` via `docs.yml` and publishes them to GitHub Pages.
