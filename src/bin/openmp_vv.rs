use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clap::Parser;
use roup::lexer::Language;
use roup::parser::Parser as OmpParser;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
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

    /// clang executable used for preprocessing OpenMP_VV sources.
    #[arg(long, default_value = "clang")]
    clang: String,

    /// clang-format executable used for normalizing pragma strings.
    #[arg(long, default_value = "clang-format")]
    clang_format: String,
}

struct Context<'a> {
    repo_path: &'a Path,
    clang: &'a str,
    clang_format: &'a str,
}

#[derive(Debug)]
enum FileResult {
    NoPragmas,
    Match {
        directives: usize,
    },
    Mismatch {
        directives: usize,
        expected: String,
        actual: String,
    },
    ParseError {
        line: usize,
        directive: String,
        error: String,
    },
}

#[derive(Debug)]
struct MismatchReport {
    path: PathBuf,
    directives: usize,
    expected: String,
    actual: String,
}

#[derive(Debug)]
struct ParseFailure {
    path: PathBuf,
    line: usize,
    directive: String,
    error: String,
}

#[derive(Debug)]
struct ToolFailure {
    path: PathBuf,
    message: String,
}

type DynResult<T> = Result<T, Box<dyn Error>>;

type ToolResult<T> = Result<T, String>;

fn main() -> DynResult<()> {
    let args = Args::parse();
    let repo_path = resolve_repo(&args)?;
    let tests_root = repo_path.join(&args.tests_dir);

    if !tests_root.is_dir() {
        return Err(format!("{} is not a directory", tests_root.display()).into());
    }

    let context = Context {
        repo_path: &repo_path,
        clang: &args.clang,
        clang_format: &args.clang_format,
    };

    let parser = OmpParser::default().with_language(Language::C);

    let mut files: Vec<PathBuf> = WalkDir::new(&tests_root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| is_c_like_source(path))
        .collect();
    files.sort();

    let mut processed_files = 0usize;
    let mut successful_files = 0usize;
    let mut total_directives = 0usize;

    let mut mismatches = Vec::new();
    let mut parse_failures = Vec::new();
    let mut tool_failures = Vec::new();

    for path in files {
        match process_file(&path, &context, &parser) {
            Ok(FileResult::NoPragmas) => {}
            Ok(FileResult::Match { directives }) => {
                processed_files += 1;
                successful_files += 1;
                total_directives += directives;
            }
            Ok(FileResult::Mismatch {
                directives,
                expected,
                actual,
            }) => {
                processed_files += 1;
                total_directives += directives;
                mismatches.push(MismatchReport {
                    path: path.clone(),
                    directives,
                    expected,
                    actual,
                });
            }
            Ok(FileResult::ParseError {
                line,
                directive,
                error,
            }) => {
                processed_files += 1;
                parse_failures.push(ParseFailure {
                    path: path.clone(),
                    line,
                    directive,
                    error,
                });
            }
            Err(message) => {
                tool_failures.push(ToolFailure {
                    path: path.clone(),
                    message,
                });
            }
        }
    }

    println!("Round-trip validation summary:");
    println!("  Files with OpenMP pragmas: {}", processed_files);
    println!("  Successful round-trips: {}", successful_files);
    println!("  Total directives checked: {}", total_directives);

    if processed_files > 0 {
        let rate = (successful_files as f64 / processed_files as f64) * 100.0;
        println!("  Success rate: {:.1}%", rate);
    }

    if !mismatches.is_empty() {
        println!();
        println!("Round-trip mismatches ({}):", mismatches.len());
        for report in &mismatches {
            println!(
                "- {} ({} directives)",
                report.path.display(),
                report.directives
            );
            println!("    Expected:");
            println!("{}", indent_block(&report.expected));
            println!("    Actual:");
            println!("{}", indent_block(&report.actual));
        }
    }

    if !parse_failures.is_empty() {
        println!();
        println!("Parse failures ({}):", parse_failures.len());
        for failure in &parse_failures {
            println!("- {}:{}", failure.path.display(), failure.line);
            println!("    Directive: {}", failure.directive);
            println!("    Error: {}", failure.error);
        }
    }

    if !tool_failures.is_empty() {
        println!();
        println!("Tool execution failures ({}):", tool_failures.len());
        for failure in &tool_failures {
            println!("- {}", failure.path.display());
            println!("    {}", failure.message);
        }
    }

    if mismatches.is_empty() && parse_failures.is_empty() && tool_failures.is_empty() {
        Ok(())
    } else {
        Err("OpenMP_VV round-trip validation failed".into())
    }
}

fn resolve_repo(args: &Args) -> DynResult<PathBuf> {
    let repo_path = args
        .repo_path
        .clone()
        .unwrap_or_else(|| PathBuf::from("target").join("openmp_vv"));

    if repo_path.exists() {
        if !repo_path.is_dir() {
            return Err(format!("{} exists but is not a directory", repo_path.display()).into());
        }
        return Ok(repo_path);
    }

    if args.skip_clone {
        return Err(format!(
            "{} does not exist; run without --skip-clone or pass --repo-path",
            repo_path.display()
        )
        .into());
    }

    if let Some(parent) = repo_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let status = Command::new("git")
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg(&args.repo_url)
        .arg(&repo_path)
        .status()?;

    if !status.success() {
        return Err("failed to clone OpenMP_VV repository".into());
    }

    Ok(repo_path)
}

fn process_file(path: &Path, ctx: &Context<'_>, parser: &OmpParser) -> ToolResult<FileResult> {
    let preprocessed = clang_preprocess(path, ctx)?;
    let directives: Vec<String> = preprocessed
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("#pragma omp") {
                Some(trimmed.to_string())
            } else {
                None
            }
        })
        .collect();

    if directives.is_empty() {
        return Ok(FileResult::NoPragmas);
    }

    // Preserve deterministic order by running clang-format after preprocessing.
    let canonical_input = clang_format(&directives.join("\n"), ctx)?
        .trim()
        .to_string();

    if canonical_input.is_empty() {
        return Ok(FileResult::NoPragmas);
    }

    let mut round_tripped = Vec::with_capacity(directives.len());

    for (idx, directive) in directives.iter().enumerate() {
        let parse_result = parser.parse(directive.as_str());
        let (rest, parsed) = match parse_result {
            Ok(value) => value,
            Err(err) => {
                return Ok(FileResult::ParseError {
                    line: idx + 1,
                    directive: directive.clone(),
                    error: format!("{err:?}"),
                });
            }
        };

        if !rest.trim().is_empty() {
            return Ok(FileResult::ParseError {
                line: idx + 1,
                directive: directive.clone(),
                error: format!("unexpected trailing input: {rest}"),
            });
        }

        round_tripped.push(parsed.to_pragma_string());
    }

    let canonical_output = clang_format(&round_tripped.join("\n"), ctx)?
        .trim()
        .to_string();

    if canonical_input == canonical_output {
        Ok(FileResult::Match {
            directives: round_tripped.len(),
        })
    } else {
        Ok(FileResult::Mismatch {
            directives: round_tripped.len(),
            expected: canonical_input,
            actual: canonical_output,
        })
    }
}

fn clang_preprocess(path: &Path, ctx: &Context<'_>) -> ToolResult<String> {
    let mut command = Command::new(ctx.clang);
    command
        .current_dir(ctx.repo_path)
        .arg("-E")
        .arg("-P")
        .arg("-CC")
        .arg("-fopenmp");

    if let Some(parent) = path.parent() {
        command.arg("-I").arg(parent);
    }

    command.arg(path);

    let output = command
        .output()
        .map_err(|err| format!("failed to run clang: {err}"))?;

    if !output.status.success() {
        return Err(format!(
            "clang returned {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    String::from_utf8(output.stdout)
        .map_err(|err| format!("clang output was not valid UTF-8: {err}"))
}

fn clang_format(input: &str, ctx: &Context<'_>) -> ToolResult<String> {
    let mut child = Command::new(ctx.clang_format)
        .arg("-style")
        .arg("LLVM")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("failed to run clang-format: {err}"))?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(input.as_bytes())
            .map_err(|err| format!("failed to write to clang-format stdin: {err}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|err| format!("failed to read clang-format output: {err}"))?;

    if !output.status.success() {
        return Err(format!(
            "clang-format returned {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    String::from_utf8(output.stdout)
        .map_err(|err| format!("clang-format output was not valid UTF-8: {err}"))
}

fn is_c_like_source(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            ext.eq_ignore_ascii_case("c")
                || ext.eq_ignore_ascii_case("cc")
                || ext.eq_ignore_ascii_case("cp")
                || ext.eq_ignore_ascii_case("cxx")
                || ext.eq_ignore_ascii_case("cpp")
                || ext.eq_ignore_ascii_case("c++")
                || ext.eq_ignore_ascii_case("i")
        })
        .unwrap_or(false)
}

fn indent_block(text: &str) -> String {
    text.lines()
        .map(|line| format!("      {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}
