# ROUP Documentation Website

This directory contains the source for the ROUP documentation website at **https://roup.ouankou.com**

## Architecture

The website uses a **hybrid documentation system**:
- **mdBook** - Human-focused documentation (tutorials, guides)
- **rustdoc** - Auto-generated API reference from source code
- Both combined into a single site deployed via GitHub Pages

## Local Development

### Prerequisites
```bash
cargo install mdbook
```

### Build Documentation
```bash
# Build mdBook
mdbook build

# Build rustdoc
cd ../..
cargo doc --no-deps --all-features

# Combine both
mkdir -p book/api
cp -r ../../target/doc/* book/api/
```

### Live Preview
```bash
mdbook serve --open
# Opens http://localhost:3000 with live reload
```

## Structure

```
docs/book/
├── book.toml           # mdBook configuration
├── src/                # Source markdown files
│   ├── SUMMARY.md      # Table of contents
│   ├── intro.md        # Introduction/landing page
│   ├── cpp-tutorial.md # Detailed C++ tutorial
│   └── api-reference.md # API reference (links to rustdoc)
└── book/               # Generated output (git-ignored)
    ├── *.html          # mdBook HTML files
    └── api/            # rustdoc output
        └── roup/       # Rust API docs
```

## Deployment

The documentation is automatically deployed via GitHub Actions (`.github/workflows/docs.yml`):

1. **On PR**: Validates that docs build successfully (no deployment)
2. **On merge to main**: Builds and deploys to `gh-pages` branch
3. **GitHub Pages**: Serves from `gh-pages` at roup.ouankou.com

### Manual Deployment

If you need to deploy manually:

```bash
# Build everything
mdbook build
cd ../..
cargo doc --no-deps --all-features
cd docs/book
mkdir -p book/api
cp -r ../../target/doc/* book/api/

# Deploy to gh-pages branch
# (Normally handled by GitHub Actions)
```

## Adding Content

### New Page

1. Create markdown file in `src/`:
   ```bash
   touch src/my-new-page.md
   ```

2. Add to `SUMMARY.md`:
   ```markdown
   - [My New Page](./my-new-page.md)
   ```

3. Test locally:
   ```bash
   mdbook serve
   ```

### Including Code Examples

Use mdBook's `{{#include}}` preprocessor to include code from the repo:

```markdown
{{#include ../../examples/c/tutorial_basic.c:1:50}}
```

This ensures examples stay in sync with actual tested code.

### Linking to rustdoc

Link to the rustdoc API from mdBook pages:

```markdown
See the [Rust API Documentation](./api/roup/index.html) for details.
```

## Configuration

Key settings in `book.toml`:

- **title**: "ROUP Documentation"
- **cname**: "roup.ouankou.com" (custom domain)
- **site-url**: "/" (root path)
- **theme**: "rust" (Rust book style)
- **search**: Enabled (full-text search)

## Custom Domain Setup

To use `roup.ouankou.com`:

1. **DNS Configuration** (at your DNS provider):
   ```
   Type: CNAME
   Name: roup
   Value: ouankou.github.io
   TTL: Auto/3600
   ```

2. **GitHub Pages Settings**:
   - Repository → Settings → Pages
   - Source: `gh-pages` branch
   - Custom domain: `roup.ouankou.com`
   - Enforce HTTPS: ✅

3. **CNAME file**: Auto-created by workflow from `book.toml`

## Troubleshooting

### mdBook not found
```bash
cargo install mdbook
```

### Build fails
```bash
mdbook clean
mdbook build
```

### Links to rustdoc broken
Make sure rustdoc is copied to `book/api/`:
```bash
cargo doc --no-deps --all-features
cp -r ../../target/doc/* book/api/
```

### Live reload not working
Try:
```bash
mdbook serve --hostname 0.0.0.0
```

## Resources

- **mdBook Guide**: https://rust-lang.github.io/mdBook/
- **rustdoc Guide**: https://doc.rust-lang.org/rustdoc/
- **GitHub Pages**: https://docs.github.com/en/pages

## License

Same as ROUP: MIT License
