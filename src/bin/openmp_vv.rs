use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clap::{ArgAction, Parser};
use roup::lexer::Language;
use roup::parser::Parser as OmpParser;
use walkdir::WalkDir;

type DynError = Box<dyn std::error::Error>;

#[derive(Debug, Parser)]
#[command(
    name = "roup-openmp-vv",
    about = "Validate that ROUP round-trips OpenMP pragmas from OpenMP_VV"
)]
struct Args {
    /// Location of the OpenMP_VV repository on disk. Defaults to target/openmp_vv.
    #[arg(long)]
    repo_path: Option<PathBuf>,

    /// Git URL used when cloning the OpenMP_VV repository.
    #[arg(
        long,
        default_value = "https://github.com/OpenMP-Validation-and-Verification/OpenMP_VV"
    )]
    repo_url: String,

    /// Relative path (within the repository) that contains the tests directory.
    #[arg(long, default_value = "tests")]
    tests_dir: PathBuf,

    /// Skip cloning when the repository does not exist yet.
    #[arg(long)]
    skip_clone: bool,

    /// Path to the clang executable used for preprocessing.
    #[arg(long, default_value = "clang")]
    clang: String,

    /// Additional argument passed to clang (can be repeated).
    #[arg(long = "clang-arg", action = ArgAction::Append)]
    clang_arg: Vec<String>,

    /// Path to the clang-format executable used for canonicalizing pragmas.
    #[arg(long, default_value = "clang-format")]
    clang_format: String,

    /// Additional argument passed to clang-format (can be repeated).
    #[arg(long = "clang-format-arg", action = ArgAction::Append)]
    clang_format_arg: Vec<String>,

    /// Extra include directories forwarded to clang during preprocessing.
    #[arg(long = "include", action = ArgAction::Append)]
    include_dirs: Vec<PathBuf>,

    /// Maximum number of failures to print in the report. Set to 0 for unlimited.
    #[arg(long, default_value_t = 20)]
    max_failures: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum SourceLanguage {
    C,
    Cxx,
}

impl SourceLanguage {
    fn label(self) -> &'static str {
        match self {
            SourceLanguage::C => "C",
            SourceLanguage::Cxx => "C++",
        }
    }

    fn assume_filename(self) -> &'static str {
        match self {
            SourceLanguage::C => "file.c",
            SourceLanguage::Cxx => "file.cpp",
        }
    }

    fn clang_language(self) -> &'static str {
        match self {
            SourceLanguage::C => "c",
            SourceLanguage::Cxx => "c++",
        }
    }
}

#[derive(Default, Debug)]
struct LanguageStats {
    directives: usize,
    successes: usize,
}

#[derive(Debug, Clone)]
struct DirectiveRecord {
    path: PathBuf,
    line: usize,
    language: SourceLanguage,
    pragma: String,
}

#[derive(Debug)]
struct FailureRecord {
    path: PathBuf,
    line: usize,
    language: SourceLanguage,
    original: String,
    rewritten: Option<String>,
    error: String,
}

struct ParserCache {
    c: OmpParser,
}

impl ParserCache {
    fn new() -> Self {
        Self {
            c: OmpParser::default().with_language(Language::C),
        }
    }

    fn parser(&self, language: SourceLanguage) -> &OmpParser {
        match language {
            SourceLanguage::C | SourceLanguage::Cxx => &self.c,
        }
    }
}

fn main() -> Result<(), DynError> {
    let args = Args::parse();

    let repo_path = args
        .repo_path
        .clone()
        .unwrap_or_else(|| PathBuf::from("target").join("openmp_vv"));

    ensure_repo(&repo_path, &args.repo_url, args.skip_clone)?;

    let tests_root = repo_path.join(&args.tests_dir);
    if !tests_root.exists() {
        return Err(format!("tests directory {} does not exist", tests_root.display()).into());
    }

    let mut stats: HashMap<SourceLanguage, LanguageStats> = HashMap::new();
    let mut failures = Vec::new();

    let parser_cache = ParserCache::new();

    for entry in WalkDir::new(&tests_root) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let Some(language) = detect_language(entry.path()) else {
            continue;
        };

        let canonical_path = entry.path().canonicalize()?;
        let preprocessed = match preprocess_file(&args, &repo_path, &canonical_path, language) {
            Ok(output) => output,
            Err(err) => {
                failures.push(FailureRecord {
                    path: canonical_path.clone(),
                    line: 0,
                    language,
                    original: String::new(),
                    rewritten: None,
                    error: format!("failed to preprocess with clang: {err}"),
                });
                continue;
            }
        };

        let directives = extract_directives(&preprocessed, &canonical_path, language);
        if directives.is_empty() {
            continue;
        }

        let entry_stats = stats.entry(language).or_default();
        entry_stats.directives += directives.len();

        for directive in directives {
            match validate_directive(&args, &parser_cache, &directive) {
                Ok(_) => entry_stats.successes += 1,
                Err(failure) => failures.push(failure),
            }
        }
    }

    print_summary(&stats, &failures, args.max_failures);

    if failures.is_empty() {
        Ok(())
    } else {
        std::process::exit(1);
    }
}

fn ensure_repo(repo_path: &Path, repo_url: &str, skip_clone: bool) -> Result<(), DynError> {
    if repo_path.exists() {
        return Ok(());
    }

    if skip_clone {
        return Err(format!(
            "repository {} does not exist and --skip-clone was provided",
            repo_path.display()
        )
        .into());
    }

    if let Some(parent) = repo_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let status = Command::new("git")
        .arg("clone")
        .arg(repo_url)
        .arg(repo_path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err("failed to clone OpenMP_VV".into())
    }
}

fn detect_language(path: &Path) -> Option<SourceLanguage> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "c" | "i" => Some(SourceLanguage::C),
        "cc" | "cpp" | "cxx" | "c++" => Some(SourceLanguage::Cxx),
        _ => None,
    }
}

fn preprocess_file(
    args: &Args,
    repo_root: &Path,
    path: &Path,
    language: SourceLanguage,
) -> Result<String, DynError> {
    let mut command = Command::new(&args.clang);
    command.current_dir(repo_root);
    command.arg("-E");
    command.arg("-fopenmp");
    command.arg("-CC");
    command.arg("-x");
    command.arg(language.clang_language());

    for dir in &args.include_dirs {
        command.arg("-I");
        command.arg(dir);
    }
    if let Some(parent) = path.parent() {
        command.arg("-I");
        command.arg(parent);
    }

    for extra in &args.clang_arg {
        command.arg(extra);
    }

    command.arg(path);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("clang failed: {stderr}").into());
    }

    Ok(String::from_utf8(output.stdout)?)
}

fn extract_directives(
    preprocessed: &str,
    original_path: &Path,
    language: SourceLanguage,
) -> Vec<DirectiveRecord> {
    let mut current_path = original_path.to_path_buf();
    let mut current_line: usize = 0;
    let mut directives = Vec::new();

    for raw_line in preprocessed.lines() {
        let line = raw_line.trim_end();

        if let Some(rest) = line.strip_prefix('#') {
            if let Some((line_no, path)) = parse_line_marker(rest, original_path.parent()) {
                current_line = line_no.saturating_sub(1);
                if let Some(path) = path {
                    current_path = path;
                }
            }
            continue;
        }

        current_line = current_line.saturating_add(1);

        let needle = match language {
            SourceLanguage::C | SourceLanguage::Cxx => "#pragma omp",
        };

        if let Some(idx) = line.find(needle) {
            let pragma = line[idx..].trim().to_string();
            if current_path == original_path {
                directives.push(DirectiveRecord {
                    path: current_path.clone(),
                    line: current_line,
                    language,
                    pragma,
                });
            }
        }
    }

    directives
}

fn parse_line_marker(rest: &str, base_dir: Option<&Path>) -> Option<(usize, Option<PathBuf>)> {
    let trimmed = rest.trim_start();
    let trimmed = if let Some(stripped) = trimmed.strip_prefix("line") {
        stripped.trim_start()
    } else {
        trimmed
    };

    let mut parts = trimmed.split_whitespace();
    let line_number = parts.next()?.parse().ok()?;
    let file = parts
        .next()
        .map(|token| token.trim_matches('"'))
        .and_then(|token| resolve_marker_path(token, base_dir));
    Some((line_number, file))
}

fn resolve_marker_path(token: &str, base_dir: Option<&Path>) -> Option<PathBuf> {
    if token.starts_with('<') {
        return None;
    }

    let raw_path = PathBuf::from(token);
    let resolved = if raw_path.is_relative() {
        base_dir.map(|dir| dir.join(&raw_path)).unwrap_or(raw_path)
    } else {
        raw_path
    };

    match resolved.canonicalize() {
        Ok(canonical) => Some(canonical),
        Err(_) => Some(resolved),
    }
}

fn validate_directive(
    args: &Args,
    parser_cache: &ParserCache,
    directive: &DirectiveRecord,
) -> Result<(), FailureRecord> {
    let formatted_input = match clang_format(&args.clang_format, &args.clang_format_arg, directive)
    {
        Ok(output) => output,
        Err(err) => {
            return Err(FailureRecord {
                path: directive.path.clone(),
                line: directive.line,
                language: directive.language,
                original: directive.pragma.clone(),
                rewritten: None,
                error: format!("clang-format failed: {err}"),
            })
        }
    };

    let parser = parser_cache.parser(directive.language);
    let trimmed_input = formatted_input.trim().to_string();
    match parser.parse(&trimmed_input) {
        Ok((rest, parsed)) => {
            if !rest.trim().is_empty() {
                return Err(FailureRecord {
                    path: directive.path.clone(),
                    line: directive.line,
                    language: directive.language,
                    original: formatted_input.clone(),
                    rewritten: None,
                    error: format!("unexpected trailing input: {rest:?}"),
                });
            }

            let rewritten = parsed.to_pragma_string();
            let formatted_rewritten = match clang_format(
                &args.clang_format,
                &args.clang_format_arg,
                &DirectiveRecord {
                    pragma: rewritten,
                    ..directive.clone()
                },
            ) {
                Ok(text) => text,
                Err(err) => {
                    return Err(FailureRecord {
                        path: directive.path.clone(),
                        line: directive.line,
                        language: directive.language,
                        original: formatted_input.clone(),
                        rewritten: None,
                        error: format!("clang-format failed for rewritten pragma: {err}"),
                    });
                }
            };

            if normalize(&formatted_input) == normalize(&formatted_rewritten) {
                Ok(())
            } else {
                Err(FailureRecord {
                    path: directive.path.clone(),
                    line: directive.line,
                    language: directive.language,
                    original: formatted_input,
                    rewritten: Some(formatted_rewritten),
                    error: "formatted pragma mismatch".into(),
                })
            }
        }
        Err(err) => Err(FailureRecord {
            path: directive.path.clone(),
            line: directive.line,
            language: directive.language,
            original: formatted_input.clone(),
            rewritten: None,
            error: format!("parser error: {err}"),
        }),
    }
}

fn clang_format(
    clang_format: &str,
    extra_args: &[String],
    directive: &DirectiveRecord,
) -> Result<String, DynError> {
    let mut command = Command::new(clang_format);
    command.arg("--assume-filename");
    command.arg(directive.language.assume_filename());
    for extra in extra_args {
        command.arg(extra);
    }
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let mut child = command.spawn()?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or("failed to open clang-format stdin")?;
        let mut payload = directive.pragma.clone();
        if !payload.ends_with('\n') {
            payload.push('\n');
        }
        stdin.write_all(payload.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("clang-format failed: {stderr}").into());
    }

    Ok(String::from_utf8(output.stdout)?)
}

fn normalize(text: &str) -> String {
    text.replace('\r', "").trim_end().to_string()
}

fn print_summary(
    stats: &HashMap<SourceLanguage, LanguageStats>,
    failures: &[FailureRecord],
    max: usize,
) {
    println!("Round-trip validation summary:\n");

    let mut totals = LanguageStats::default();
    for language in [SourceLanguage::C, SourceLanguage::Cxx] {
        if let Some(stat) = stats.get(&language) {
            println!(
                "  {:<8} - {} / {} directives matched",
                language.label(),
                stat.successes,
                stat.directives
            );
            totals.directives += stat.directives;
            totals.successes += stat.successes;
        }
    }

    println!();
    println!(
        "  Overall  - {} / {} directives matched",
        totals.successes, totals.directives
    );

    if failures.is_empty() {
        println!("\nAll directives matched after round-tripping.");
        return;
    }

    println!("\nFailures (showing up to {max}):");
    let to_show = if max == 0 {
        failures.len()
    } else {
        failures.len().min(max)
    };
    for failure in failures.iter().take(to_show) {
        println!(
            "\n{}:{} ({}) - {}",
            failure.path.display(),
            failure.line,
            failure.language.label(),
            failure.error
        );
        if !failure.original.is_empty() {
            println!("  original : {}", failure.original.trim_end());
        }
        if let Some(ref rewritten) = failure.rewritten {
            println!("  rewritten: {}", rewritten.trim_end());
        }
    }

    if failures.len() > to_show {
        println!(
            "\n... {} additional failures omitted (use --max-failures 0 to show all)",
            failures.len() - to_show
        );
    }
}
