use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clap::Parser;
use roup::parser::Parser as OmpParser;
use walkdir::WalkDir;

type DynError = Box<dyn std::error::Error>;

#[derive(Debug, Parser)]
#[command(
    name = "roup-openmp-vv",
    about = "Round-trip OpenMP pragmas from OpenMP_VV using ROUP"
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

    /// Maximum number of individual failures to show in the report (0 = no limit).
    #[arg(long, default_value_t = 20)]
    max_failures: usize,

    /// clang executable used for preprocessing.
    #[arg(long, default_value = "clang")]
    clang: String,

    /// clang-format executable used for canonicalisation.
    #[arg(long, default_value = "clang-format")]
    clang_format: String,
}

#[derive(Debug)]
struct FailureRecord {
    path: PathBuf,
    directive: String,
    detail: String,
}

#[derive(Default, Debug)]
struct Stats {
    files_seen: usize,
    directives_seen: usize,
    successes: usize,
    parse_failures: usize,
    mismatch_failures: usize,
    clang_failures: usize,
}

fn main() -> Result<(), DynError> {
    let args = Args::parse();

    let repo_path = args
        .repo_path
        .clone()
        .unwrap_or_else(|| PathBuf::from("target").join("openmp_vv"));

    ensure_repo(&repo_path, &args)?;

    let tests_root = repo_path.join(&args.tests_dir);
    if !tests_root.is_dir() {
        return Err(format!("tests directory '{}' does not exist", tests_root.display()).into());
    }

    let mut stats = Stats::default();
    let mut failures = Vec::new();
    let parser = OmpParser::default();

    for entry in WalkDir::new(&tests_root) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                eprintln!("failed to read entry: {err}");
                continue;
            }
        };

        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if !is_c_like_source(path) {
            continue;
        }

        stats.files_seen += 1;

        match process_source(path, &args, &parser) {
            Ok(result) => {
                stats.directives_seen += result.directives;
                stats.successes += result.successes;
                stats.parse_failures += result.parse_failures;
                stats.mismatch_failures += result.mismatch_failures;
                stats.clang_failures += result.clang_failures;
                failures.extend(result.failures);
            }
            Err(err) => {
                stats.clang_failures += 1;
                failures.push(FailureRecord {
                    path: path.to_path_buf(),
                    directive: String::new(),
                    detail: format!("failed to process file: {err}"),
                });
            }
        }
    }

    print_report(&stats, &failures, args.max_failures);

    if stats.parse_failures > 0 || stats.mismatch_failures > 0 || stats.clang_failures > 0 {
        std::process::exit(1);
    }

    Ok(())
}

struct FileResult {
    directives: usize,
    successes: usize,
    parse_failures: usize,
    mismatch_failures: usize,
    clang_failures: usize,
    failures: Vec<FailureRecord>,
}

fn process_source(path: &Path, args: &Args, parser: &OmpParser) -> Result<FileResult, DynError> {
    let preprocessed = preprocess_with_clang(path, &args.clang)?;
    let directives = extract_pragmas(&preprocessed);

    let mut file_failures = Vec::new();
    let mut successes = 0usize;
    let mut parse_failures = 0usize;
    let mut mismatch_failures = 0usize;
    let mut clang_failures = 0usize;

    for directive in directives {
        match parser.parse(&directive) {
            Ok((rest, parsed)) => {
                if !rest.trim().is_empty() {
                    let rest = rest.trim().to_string();
                    parse_failures += 1;
                    file_failures.push(FailureRecord {
                        path: path.to_path_buf(),
                        directive,
                        detail: format!("unparsed trailing input: '{rest}'"),
                    });
                    continue;
                }

                let round_tripped = parsed.to_pragma_string();
                match compare_via_clang_format(&directive, &round_tripped, &args.clang_format) {
                    Ok(true) => successes += 1,
                    Ok(false) => {
                        mismatch_failures += 1;
                        file_failures.push(FailureRecord {
                            path: path.to_path_buf(),
                            directive,
                            detail: format!("round-tripped pragma differed: {}", round_tripped),
                        });
                    }
                    Err(err) => {
                        clang_failures += 1;
                        file_failures.push(FailureRecord {
                            path: path.to_path_buf(),
                            directive,
                            detail: format!("clang-format error: {err}"),
                        });
                    }
                }
            }
            Err(err) => {
                let detail = format!("parse error: {err}");
                parse_failures += 1;
                file_failures.push(FailureRecord {
                    path: path.to_path_buf(),
                    directive,
                    detail,
                });
            }
        }
    }

    Ok(FileResult {
        directives: successes + parse_failures + mismatch_failures + clang_failures,
        successes,
        parse_failures,
        mismatch_failures,
        clang_failures,
        failures: file_failures,
    })
}

fn preprocess_with_clang(path: &Path, clang: &str) -> Result<String, DynError> {
    let output = Command::new(clang)
        .arg("-E")
        .arg("-P")
        .arg("-CC")
        .arg("-fopenmp")
        .arg(path)
        .stderr(Stdio::piped())
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "clang preprocessing failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(String::from_utf8(output.stdout)?)
}

fn extract_pragmas(preprocessed: &str) -> Vec<String> {
    preprocessed
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("#pragma omp") {
                Some(trimmed.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn compare_via_clang_format(
    original: &str,
    round_tripped: &str,
    clang_format: &str,
) -> Result<bool, DynError> {
    let formatted_original = format_with_clang(original, clang_format)?;
    let formatted_round_trip = format_with_clang(round_tripped, clang_format)?;
    Ok(formatted_original == formatted_round_trip)
}

fn format_with_clang(directive: &str, clang_format: &str) -> Result<String, DynError> {
    let mut child = Command::new(clang_format)
        .arg("-style=LLVM")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    {
        let mut stdin = child.stdin.take().expect("child stdin should exist");
        let stub = format!("void __roup_stub() {{\n{directive}\n}}\n");
        stdin.write_all(stub.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(format!(
            "clang-format failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let formatted = String::from_utf8(output.stdout)?;
    let pragma_line = formatted
        .lines()
        .find(|line| line.trim_start().starts_with("#pragma omp"))
        .ok_or_else(|| "clang-format output missing pragma".to_string())?
        .trim()
        .to_string();
    Ok(pragma_line)
}

fn ensure_repo(repo_path: &Path, args: &Args) -> Result<(), DynError> {
    if repo_path.is_dir() {
        return Ok(());
    }

    if args.skip_clone {
        return Err(format!(
            "repository not found at '{}' (pass --skip-clone=false to clone)",
            repo_path.display()
        )
        .into());
    }

    let parent = repo_path
        .parent()
        .ok_or_else(|| "repository path must have a parent directory".to_string())?;
    fs::create_dir_all(parent)?;

    let status = Command::new("git")
        .arg("clone")
        .arg(&args.repo_url)
        .arg(repo_path)
        .status()?;

    if !status.success() {
        return Err("git clone failed".into());
    }

    Ok(())
}

fn is_c_like_source(path: &Path) -> bool {
    match path.extension().and_then(OsStr::to_str) {
        Some(ext) => matches!(
            ext,
            "c" | "cc" | "cp" | "cxx" | "cpp" | "c++" | "C" | "i" | "ii" | "m" | "mm"
        ),
        None => false,
    }
}

fn print_report(stats: &Stats, failures: &[FailureRecord], max_failures: usize) {
    println!("Processed {} files", stats.files_seen);
    println!("Found {} OpenMP pragmas", stats.directives_seen);
    println!("Round-trip successes: {}", stats.successes);
    println!("Parse failures: {}", stats.parse_failures);
    println!("Round-trip mismatches: {}", stats.mismatch_failures);
    println!("clang/clang-format failures: {}", stats.clang_failures);

    if failures.is_empty() {
        return;
    }

    println!("\nFailures:");
    let mut shown = 0usize;
    for failure in failures {
        if max_failures != 0 && shown >= max_failures {
            println!("... {} more failures omitted", failures.len() - shown);
            break;
        }
        println!("- {}", failure.path.display());
        if !failure.directive.is_empty() {
            println!("  directive: {}", failure.directive);
        }
        println!("  detail: {}", failure.detail);
        shown += 1;
    }
}
