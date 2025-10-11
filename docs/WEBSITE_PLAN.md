# ROUP Documentation Website Plan

**Target URL:** `https://roup.ouankou.com`  
**Technology:** GitHub Pages + mdBook (Rust's documentation tool)  
**Automation:** GitHub Actions for auto-deployment

---

## 1. Technology Choice: mdBook

### Why mdBook?
- ✅ **Rust ecosystem standard** (used by Rust book, cargo book, etc.)
- ✅ **Zero config Markdown** - Write pure MD, get beautiful site
- ✅ **Built-in search** - Fast, client-side full-text search
- ✅ **Syntax highlighting** - Rust, C, C++, bash support
- ✅ **Mobile responsive** - Works on all devices
- ✅ **Fast builds** - Static HTML, no JavaScript framework needed
- ✅ **GitHub Pages ready** - One command deployment
- ✅ **Themeable** - Light/dark mode built-in
- ✅ **Code playground** - Can integrate Rust Playground

### Alternatives Considered
- ❌ **Docusaurus** (React-based, overkill for our needs)
- ❌ **Jekyll** (GitHub default, but less Rust-focused)
- ❌ **Sphinx** (Python ecosystem, not idiomatic for Rust)
- ⚠️ **rustdoc** (Great for API docs, but not for guides/tutorials)

**Decision:** mdBook for human docs + rustdoc for API reference

---

## 2. Content Structure

### Site Map

```
roup.ouankou.com/
├── Home                           (Landing page)
├── Getting Started
│   ├── Quick Start               (5-minute setup)
│   ├── Installation              (Rust, C, C++ setup)
│   └── Your First Parse          (Hello World examples)
├── Tutorials
│   ├── Rust Tutorial             (Idiomatic Rust usage)
│   ├── C Tutorial                (From tutorial_basic.c)
│   ├── C++ Tutorial              (From tutorial_basic.cpp)
│   └── Advanced Parsing          (Complex directives)
├── API Reference
│   ├── Rust API Docs             (Link to docs.rs or self-hosted rustdoc)
│   ├── C API Reference           (18 functions, examples)
│   └── C++ RAII Wrappers         (Modern C++ patterns)
├── OpenMP Support
│   ├── Supported Directives      (Feature matrix)
│   ├── Supported Clauses         (Complete list)
│   └── Compatibility Notes       (OpenMP 5.0/6.0)
├── Guides
│   ├── Building from Source      (Detailed build guide)
│   ├── Cross-Platform Building   (Linux/macOS/Windows)
│   ├── FFI Integration           (Calling from other languages)
│   └── Testing Strategy          (355 tests explained)
├── Design & Architecture
│   ├── Why Rust?                 (Safety, performance)
│   ├── Parser Architecture       (nom combinators)
│   ├── Minimal Unsafe Design     (0.9% unsafe code)
│   └── Development History       (Phases 1-3)
├── Contributing
│   ├── Code of Conduct
│   ├── Development Setup
│   ├── Coding Standards          (AGENTS.md)
│   └── Release Process
└── About
    ├── Project Goals
    ├── Roadmap                   (Future features)
    ├── License
    └── Acknowledgments
```

---

## 3. Content Sources

### Existing Documentation to Integrate

| Existing File | Website Section | Transformation Needed |
|---------------|-----------------|------------------------|
| `README.md` | Home + Quick Start | Split into landing + quick start |
| `docs/QUICK_START.md` | Getting Started > Quick Start | Minor formatting |
| `docs/OPENMP_SUPPORT.md` | OpenMP Support > Feature Matrix | Convert tables |
| `docs/C_FFI_STATUS.md` | API Reference > C API | Add examples |
| `docs/TUTORIAL_BUILDING_AND_RUNNING.md` | Guides > Building from Source | Reorganize by platform |
| `docs/IMPLEMENTATION_SUMMARY.md` | Design > Parser Architecture | Simplify for readers |
| `docs/DEVELOPMENT_HISTORY.md` | Design > Development History | Timeline format |
| `docs/MINIMAL_UNSAFE_SUMMARY.md` | Design > Minimal Unsafe | Safety guarantees focus |
| `examples/c/tutorial_basic.c` | Tutorials > C Tutorial | Extract + annotate |
| `examples/cpp/tutorial_basic.cpp` | Tutorials > C++ Tutorial | Extract + annotate |

### New Content to Create

1. **Landing Page** - Hero section, features, quick examples
2. **Rust Tutorial** - Idiomatic Rust usage (currently only in rustdoc)
3. **Advanced Topics** - Metadirectives, error handling, performance
4. **Roadmap** - Future plans (OpenACC, more directives)
5. **FAQ** - Common questions
6. **Benchmarks** - Performance comparison (if available)

---

## 4. How to Generate Content

### Setup Process

```bash
# 1. Install mdBook
cargo install mdbook

# 2. Create book structure
cd /workspaces/roup
mdbook init docs/book
# Answer prompts:
#   What title would you like to give the book? > ROUP Documentation
#   Do you want a .gitignore? > y

# 3. Configure book.toml
# (See configuration section below)

# 4. Organize content
mkdir -p docs/book/src/{getting-started,tutorials,api,openmp,guides,design,contributing}

# 5. Build and preview
mdbook serve docs/book --open
# Opens http://localhost:3000 with live reload

# 6. Build for production
mdbook build docs/book
# Generates docs/book/book/ folder with static HTML
```

### book.toml Configuration

```toml
[book]
title = "ROUP Documentation"
authors = ["Anjia Wang"]
description = "Rust-based OpenMP/OpenACC Unified Parser"
language = "en"
multilingual = false
src = "src"

[build]
build-dir = "book"
create-missing = true

[output.html]
default-theme = "rust"
preferred-dark-theme = "navy"
git-repository-url = "https://github.com/ouankou/roup"
edit-url-template = "https://github.com/ouankou/roup/edit/main/docs/book/{path}"
site-url = "/roup/"  # For GitHub Pages at ouankou.github.io/roup
cname = "roup.ouankou.com"  # For custom domain

[output.html.fold]
enable = true  # Collapsible sections
level = 1

[output.html.search]
enable = true
limit-results = 30
use-boolean-and = true

[output.html.playground]
runnable = false  # No Rust Playground for now

[[output.html.redirect]]
"/index.html" = "getting-started/index.html"
```

---

## 5. Representation & Design

### Visual Design

**Theme:** Professional, clean, Rust-inspired
- **Colors:** Rust orange (#CE422B) for accents, dark mode support
- **Typography:** System fonts (readable, fast)
- **Code blocks:** Syntax highlighted (Rust, C, C++, bash)
- **Tables:** Responsive, sortable feature matrices

### Page Templates

#### Landing Page (Home)
```markdown
# ROUP

**Rust-based OpenMP/OpenACC Unified Parser**

Safe, fast, and extensible directive parser with multi-language support.

[Get Started →](getting-started/quick-start.md) | [View on GitHub](https://github.com/ouankou/roup)

## Features

- ✅ **99.1% Safe Rust** - Minimal unsafe code
- ✅ **Multi-Language** - Rust, C, C++ APIs
- ✅ **OpenMP 5.0+** - 15+ directives, 50+ clauses
- ✅ **355 Tests** - Comprehensive test coverage

## Quick Example

```rust
use roup::parser::parse;

let directive = parse("#pragma omp parallel for num_threads(4)").unwrap();
println!("Clauses: {}", directive.clauses.len());
```

[See more examples →](tutorials/rust-tutorial.md)
```

#### Tutorial Pages
- **Structure:** Concept → Example → Explanation → Exercise
- **Code:** Runnable examples with copy button
- **Navigation:** Previous/Next chapter links

#### API Reference
- **Layout:** Function signature → Parameters → Returns → Example
- **Search:** Full-text search across all functions
- **Cross-links:** Link between related functions

---

## 6. Long-term Maintenance

### Content Updates

| Trigger | Update Type | Frequency |
|---------|-------------|-----------|
| **New release** | Version number, changelog | Per release |
| **API changes** | API reference, examples | Per API change |
| **New directives** | OpenMP support matrix | As implemented |
| **Bug fixes** | Troubleshooting section | As needed |
| **Community feedback** | FAQ, clarifications | Ongoing |

### Documentation Review Schedule

- **Monthly:** Check for broken links, outdated examples
- **Per release:** Update all version references
- **Quarterly:** Review analytics, improve popular pages
- **Annually:** Major content refresh

### Ownership & Contributors

- **Primary maintainer:** You (Anjia Wang)
- **Contributors:** Accept PRs for docs via GitHub
- **Review process:** Same as code (PR → review → merge)

---

## 7. Automatic Updates (CI/CD)

### GitHub Actions Workflow

**File:** `.github/workflows/docs.yml`

```yaml
name: Documentation

on:
  push:
    branches: [main]
    paths:
      - 'docs/book/**'
      - 'README.md'
      - 'examples/**/*.c'
      - 'examples/**/*.cpp'
  workflow_dispatch:  # Manual trigger

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup mdBook
        uses: peaceiris/actions-mdbook@v2
        with:
          mdbook-version: 'latest'
      
      - name: Build book
        run: mdbook build docs/book
      
      - name: Deploy to GitHub Pages
        uses: peaceiris/actions-gh-pages@v4
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./docs/book/book
          cname: roup.ouankou.com
```

**Triggers:**
- ✅ Push to main (docs changes)
- ✅ Manual dispatch (on-demand)
- ⚠️ NOT on every commit (only docs/ changes)

### Rustdoc Automation

Generate Rust API docs alongside mdBook:

```yaml
- name: Build Rust API docs
  run: |
    cargo doc --no-deps --document-private-items
    cp -r target/doc docs/book/book/api/rust/
```

**Result:** `roup.ouankou.com/api/rust/roup/` has full rustdoc

---

## 8. Custom Domain Setup

### DNS Configuration (roup.ouankou.com)

**At your DNS provider (e.g., Cloudflare, Namecheap):**

```
Type    Name    Value                   TTL
CNAME   roup    ouankou.github.io       Auto/3600
```

### GitHub Pages Settings

1. Go to: `github.com/ouankou/roup/settings/pages`
2. Source: `gh-pages` branch, `/ (root)`
3. Custom domain: `roup.ouankou.com`
4. Enforce HTTPS: ✅ (auto via GitHub)

### CNAME File

**Created automatically by actions-gh-pages action** with this config:

```yaml
cname: roup.ouankou.com
```

**Manual alternative:** Create `docs/book/book/CNAME` with content:
```
roup.ouankou.com
```

---

## 9. Discussion Points

### Questions to Decide

#### 1. **API Documentation Hosting**
- **Option A:** Self-host rustdoc on GitHub Pages
  - ✅ All docs in one place
  - ❌ Must rebuild on every commit
- **Option B:** Use docs.rs (official Rust docs)
  - ✅ Auto-built on crates.io publish
  - ❌ Split between two sites
  - ✅ Better SEO for Rust users

**Recommendation:** Both! Link to docs.rs from website, but also self-host for offline use.

#### 2. **Update Frequency**
- **Conservative:** Manual updates per release
  - ✅ Stable, reviewed content
  - ❌ Docs lag behind development
- **Aggressive:** Auto-deploy on every push
  - ✅ Always up-to-date
  - ❌ Risk of incomplete docs going live

**Recommendation:** Auto-deploy only for `docs/book/**` changes, not all commits.

#### 3. **Code Examples Strategy**
- **Option A:** Inline code blocks (current)
  - ✅ Easy to maintain
  - ❌ Can get out of sync
- **Option B:** Include from actual files
  - ✅ Always accurate (tested code)
  - ❌ More complex build process

**Recommendation:** Use mdBook's `{{#include}}` preprocessor to include examples from `examples/` directory.

Example:
```markdown
{{#include ../../examples/c/tutorial_basic.c:1:50}}
```

#### 4. **Versioned Documentation**
- **Option A:** Single "latest" version
  - ✅ Simple to maintain
  - ❌ No docs for old versions
- **Option B:** Version switcher (v0.2.0, v0.3.0, etc.)
  - ✅ Users can see old API
  - ❌ Complex to maintain

**Recommendation:** Start with "latest" only. Add versioning when you reach v1.0.0.

#### 5. **Internationalization (i18n)**
- **Now:** English only
- **Future:** Chinese translation?

**Recommendation:** English for v1, add Chinese when community grows.

#### 6. **Analytics**
- Track page views, popular sections?
- Tools: Google Analytics, Plausible (privacy-friendly)

**Recommendation:** Add lightweight analytics (Plausible or GitHub traffic) to understand usage.

---

## 10. Implementation Timeline

### Phase 1: Foundation (Week 1)
- [ ] Install mdBook
- [ ] Create initial structure (`book.toml`, `SUMMARY.md`)
- [ ] Convert README → Landing page
- [ ] Convert QUICK_START → Getting Started
- [ ] Setup GitHub Actions workflow
- [ ] Configure custom domain DNS
- [ ] Deploy to roup.ouankou.com

### Phase 2: Core Content (Week 2)
- [ ] Extract C tutorial from tutorial_basic.c
- [ ] Extract C++ tutorial from tutorial_basic.cpp
- [ ] Convert OPENMP_SUPPORT → Feature matrix page
- [ ] Convert C_FFI_STATUS → API reference
- [ ] Add Rust API tutorial (new content)
- [ ] Create FAQ page

### Phase 3: Polish (Week 3)
- [ ] Add syntax highlighting for all code blocks
- [ ] Create custom CSS for ROUP branding
- [ ] Add search functionality testing
- [ ] Cross-link all related pages
- [ ] Add "Edit on GitHub" links
- [ ] Proofread and fix typos

### Phase 4: Advanced Features (Week 4)
- [ ] Self-hosted rustdoc integration
- [ ] Code example testing (ensure all examples compile)
- [ ] Add diagrams (parser architecture)
- [ ] Create video tutorials (optional)
- [ ] Setup analytics

---

## 11. Success Metrics

**Launch Goals:**
- ✅ Site loads in <2s (static HTML)
- ✅ Mobile-friendly (responsive design)
- ✅ Search works for all 50+ pages
- ✅ Zero broken links
- ✅ All code examples are runnable

**Long-term:**
- 📈 Page views per month
- 📈 Average time on site
- 📈 Search query analytics (what people look for)
- 📈 External links (docs.rs, GitHub stars)

---

## 12. Next Steps

1. **Review this plan** - Discuss questions in Section 9
2. **Make decisions** - Choose options for each discussion point
3. **Install mdBook** - `cargo install mdbook`
4. **Create prototype** - 3-5 page skeleton to test
5. **Get feedback** - Share early version for review
6. **Full deployment** - Complete all content migration

---

## Resources

- **mdBook Guide:** https://rust-lang.github.io/mdBook/
- **mdBook GitHub:** https://github.com/rust-lang/mdBook
- **Example sites:**
  - Rust Book: https://doc.rust-lang.org/book/
  - Cargo Book: https://doc.rust-lang.org/cargo/
  - Tokio Tutorial: https://tokio.rs/tokio/tutorial

**This is a living document - update as the website evolves!**
