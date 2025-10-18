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
    about = "Round-trip OpenMP directives from OpenMP_VV through the ROUP parser"
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

    /// Maximum number of failures to print in the report. Set to 0 for unlimited.
    #[arg(long, default_value_t = 20)]
    max_failures: usize,

    /// clang executable used for preprocessing.
    #[arg(long, default_value = "clang")]
    clang: String,

    /// Extra argument passed to clang during preprocessing (can be repeated).
    #[arg(long = "clang-arg", action = ArgAction::Append)]
    clang_arg: Vec<String>,

    /// clang-format executable used to canonicalize directives before/after parsing.
    #[arg(long, default_value = "clang-format")]
    clang_format: String,

    /// clang-format style (passed as -style=<value>).
    #[arg(long, default_value = "llvm")]
    clang_format_style: String,

    /// Extra argument passed to clang-format (can be repeated).
    #[arg(long = "clang-format-arg", action = ArgAction::Append)]
    clang_format_arg: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
enum FileLanguage {
    C,
    Cxx,
}

impl FileLanguage {
    fn clang_language(self) -> &'static str {
        match self {
            FileLanguage::C => "c",
            FileLanguage::Cxx => "c++",
        }
    }

    fn assume_filename(self) -> &'static str {
        match self {
            FileLanguage::C => "directive.c",
            FileLanguage::Cxx => "directive.cpp",
        }
    }
}

#[derive(Debug)]
struct Failure {
    path: PathBuf,
    line: Option<usize>,
    directive: String,
    kind: FailureKind,
    message: String,
}

impl Failure {
    fn new(
        path: &Path,
        line: Option<usize>,
        directive: String,
        kind: FailureKind,
        message: String,
    ) -> Self {
        Self {
            path: path.to_path_buf(),
            line,
            directive,
            kind,
            message,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum FailureKind {
    Preprocess,
    Format,
    Parse,
    Mismatch,
}

impl FailureKind {
    fn as_str(self) -> &'static str {
        match self {
            FailureKind::Preprocess => "preprocess",
            FailureKind::Format => "clang-format",
            FailureKind::Parse => "parse",
            FailureKind::Mismatch => "mismatch",
        }
    }
}

#[derive(Default)]
struct ProcessResult {
    directives: usize,
    matches: usize,
    failures: Vec<Failure>,
}

#[derive(Default)]
struct Totals {
    directives: usize,
    matches: usize,
    preprocess_failures: usize,
    format_failures: usize,
    parse_failures: usize,
    mismatch_failures: usize,
}

impl Totals {
    fn record_failure(&mut self, kind: FailureKind) {
        match kind {
            FailureKind::Preprocess => self.preprocess_failures += 1,
            FailureKind::Format => self.format_failures += 1,
            FailureKind::Parse => self.parse_failures += 1,
            FailureKind::Mismatch => self.mismatch_failures += 1,
        }
    }

    fn total_failures(&self) -> usize {
        self.preprocess_failures
            + self.format_failures
            + self.parse_failures
            + self.mismatch_failures
    }
}

fn main() -> Result<(), DynError> {
    let args = Args::parse();
    let repo_path = args
        .repo_path
        .clone()
        .unwrap_or_else(|| PathBuf::from("target").join("openmp_vv"));

    ensure_repo(&repo_path, &args)?;

    let tests_root = repo_path.join(&args.tests_dir);
    if !tests_root.exists() {
        return Err(format!("tests directory {:?} does not exist", tests_root).into());
    }

    let parser = OmpParser::default().with_language(Language::C);

    let mut totals = Totals::default();
    let mut failures = Vec::new();

    for entry in WalkDir::new(&tests_root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }

        let Some(lang) = classify_file(entry.path()) else {
            continue;
        };

        let result = process_file(entry.path(), lang, &parser, &args);
        totals.directives += result.directives;
        totals.matches += result.matches;
        for failure in result.failures {
            totals.record_failure(failure.kind);
            failures.push(failure);
        }
    }

    print_summary(&totals, &failures, args.max_failures);

    if totals.total_failures() > 0 {
        return Err(format!(
            "{} directives failed to round-trip through ROUP",
            totals.total_failures()
        )
        .into());
    }

    Ok(())
}

fn ensure_repo(repo_path: &Path, args: &Args) -> Result<(), DynError> {
    if repo_path.exists() {
        return Ok(());
    }

    if args.skip_clone {
        return Err(format!(
            "repository path {:?} does not exist and --skip-clone was requested",
            repo_path
        )
        .into());
    }

    if let Some(parent) = repo_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let status = Command::new("git")
        .arg("clone")
        .arg(&args.repo_url)
        .arg(repo_path)
        .status()?;

    if !status.success() {
        return Err(format!("failed to clone {}", args.repo_url).into());
    }

    Ok(())
}

fn classify_file(path: &Path) -> Option<FileLanguage> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "c" => Some(FileLanguage::C),
        "cc" | "cxx" | "cpp" | "c++" => Some(FileLanguage::Cxx),
        _ => None,
    }
}

fn process_file(path: &Path, lang: FileLanguage, parser: &OmpParser, args: &Args) -> ProcessResult {
    let mut result = ProcessResult::default();

    let preprocessed = match run_clang_preprocess(args, path, lang) {
        Ok(output) => output,
        Err(message) => {
            result.failures.push(Failure::new(
                path,
                None,
                String::new(),
                FailureKind::Preprocess,
                message,
            ));
            return result;
        }
    };

    for (idx, raw_line) in preprocessed.lines().enumerate() {
        let Some(directive) = extract_directive(raw_line) else {
            continue;
        };

        result.directives += 1;
        match validate_directive(&directive, path, idx + 1, lang, parser, args) {
            Ok(()) => result.matches += 1,
            Err(failure) => result.failures.push(failure),
        }
    }

    result
}

fn extract_directive(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with('#') {
        return None;
    }

    let mut tokens = trimmed.split_whitespace();
    let first = tokens.next()?;
    if !first.eq_ignore_ascii_case("#pragma") {
        return None;
    }
    let second = tokens.next()?;
    if !second.eq_ignore_ascii_case("omp") {
        return None;
    }

    Some(trimmed.to_string())
}

fn validate_directive(
    directive: &str,
    path: &Path,
    line: usize,
    lang: FileLanguage,
    parser: &OmpParser,
    args: &Args,
) -> Result<(), Failure> {
    let formatted_input = clang_format_directive(args, lang, directive).map_err(|msg| {
        Failure::new(
            path,
            Some(line),
            directive.to_string(),
            FailureKind::Format,
            msg,
        )
    })?;

    let formatted_trimmed = formatted_input.trim();

    let (rest, parsed) = parser.parse(formatted_trimmed).map_err(|err| {
        Failure::new(
            path,
            Some(line),
            formatted_trimmed.to_string(),
            FailureKind::Parse,
            format!("{err:?}"),
        )
    })?;

    if !rest.trim().is_empty() {
        return Err(Failure::new(
            path,
            Some(line),
            formatted_trimmed.to_string(),
            FailureKind::Parse,
            format!("unparsed trailing input: {rest:?}"),
        ));
    }

    let round_trip = parsed.to_pragma_string();
    let formatted_round_trip = clang_format_directive(args, lang, &round_trip).map_err(|msg| {
        Failure::new(
            path,
            Some(line),
            round_trip.clone(),
            FailureKind::Format,
            msg,
        )
    })?;

    let canonical_input = canonicalize(&formatted_input);
    let canonical_output = canonicalize(&formatted_round_trip);

    if canonical_input == canonical_output {
        Ok(())
    } else {
        Err(Failure::new(
            path,
            Some(line),
            directive.to_string(),
            FailureKind::Mismatch,
            format!(
                "clang-format input `{}` != round-tripped `{}`",
                canonical_input, canonical_output
            ),
        ))
    }
}

fn run_clang_preprocess(args: &Args, path: &Path, lang: FileLanguage) -> Result<String, String> {
    let mut command = Command::new(&args.clang);
    command.arg("-E");
    command.arg("-P");
    command.arg("-fopenmp");
    command.arg("-x");
    command.arg(lang.clang_language());
    for extra in &args.clang_arg {
        command.arg(extra);
    }
    command.arg(path);

    let output = command
        .output()
        .map_err(|err| format!("failed to launch {}: {err}", args.clang))?;

    if !output.status.success() {
        return Err(format!(
            "{} exited with status {}: {}",
            args.clang,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    String::from_utf8(output.stdout)
        .map_err(|err| format!("{} produced invalid UTF-8: {err}", args.clang))
}

fn clang_format_directive(
    args: &Args,
    lang: FileLanguage,
    directive: &str,
) -> Result<String, String> {
    let mut command = Command::new(&args.clang_format);
    command.arg("-style");
    command.arg(&args.clang_format_style);
    for extra in &args.clang_format_arg {
        command.arg(extra);
    }
    command.arg("-assume-filename");
    command.arg(lang.assume_filename());
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|err| format!("failed to launch {}: {err}", args.clang_format))?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| format!("failed to open {} stdin", args.clang_format))?;
        let mut snippet = directive.trim().to_string();
        if !snippet.ends_with('\n') {
            snippet.push('\n');
        }
        stdin
            .write_all(snippet.as_bytes())
            .map_err(|err| format!("failed to write to {} stdin: {err}", args.clang_format))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|err| format!("failed to read {} output: {err}", args.clang_format))?;

    if !output.status.success() {
        return Err(format!(
            "{} exited with status {}: {}",
            args.clang_format,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    String::from_utf8(output.stdout)
        .map_err(|err| format!("{} produced invalid UTF-8: {err}", args.clang_format))
}

fn canonicalize(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn print_summary(totals: &Totals, failures: &[Failure], max_failures: usize) {
    println!(
        "Processed {} OpenMP directives ({} matched, {:.1}% success).",
        totals.directives,
        totals.matches,
        if totals.directives == 0 {
            100.0
        } else {
            100.0 * totals.matches as f64 / totals.directives as f64
        }
    );

    let failure_count = totals.total_failures();
    println!(
        "Failures: {} (preprocess: {}, clang-format: {}, parse: {}, mismatch: {}).",
        failure_count,
        totals.preprocess_failures,
        totals.format_failures,
        totals.parse_failures,
        totals.mismatch_failures
    );

    if failure_count == 0 {
        return;
    }

    println!();
    println!("Sample failures:");
    let limit = if max_failures == 0 {
        failures.len()
    } else {
        failures.len().min(max_failures)
    };

    for failure in failures.iter().take(limit) {
        match failure.line {
            Some(line) => println!(
                "- [{}:{}] {} failure: {}",
                failure.path.display(),
                line,
                failure.kind.as_str(),
                failure.message
            ),
            None => println!(
                "- [{}] {} failure: {}",
                failure.path.display(),
                failure.kind.as_str(),
                failure.message
            ),
        }
        if !failure.directive.is_empty() {
            println!("  directive: {}", failure.directive);
        }
    }

    if max_failures != 0 && failures.len() > limit {
        println!(
            "... {} more failures not shown (use --max-failures 0 to display all).",
            failures.len() - limit
        );
    }
}
