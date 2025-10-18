# ROUP Documentation Website

The `docs/book` directory contains the mdBook sources for the ROUP
documentation.  CI builds the book together with the generated Rust API docs and
publishes the result to GitHub Pages.

## Local development

Install mdBook and build the content locally:

```bash
cargo install mdbook
mdbook build docs/book
```

For live previews use:

```bash
mdbook serve docs/book --open
```

To include the API reference alongside the book:

```bash
cargo doc --no-deps --all-features
mkdir -p docs/book/book/api
cp -r target/doc/* docs/book/book/api/
```

## Structure

```
docs/book/
├── book.toml        # mdBook configuration
├── README.md        # this file
├── src/             # Markdown sources consumed by mdBook
│   ├── SUMMARY.md   # table of contents
│   ├── intro.md     # landing page
│   └── ...          # tutorials, reference material, FAQ, etc.
└── book/            # Generated output (ignored by git)
```

## CI integration

`.github/workflows/ci.yml` runs the Linux test matrix and triggers a `docs`
job.  That job builds the mdBook, executes mdBook doctests, produces the Rust
API documentation, and combines everything into `docs/book/book/` before
publishing it to the `gh-pages` branch.

## Adding new pages

1. Create a Markdown file in `src/`.
2. Reference it from `src/SUMMARY.md` so mdBook adds it to the navigation.
3. Run `mdbook serve` or `mdbook build` to ensure the page compiles.

When embedding code listings prefer mdBook's `{{#include}}` directive so the
snippets stay in sync with source files that are built and tested elsewhere in
the repository.
