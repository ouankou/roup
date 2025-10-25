//! Interactive step-by-step parser debugger
//!
//! This tool allows users to step through the parsing process interactively,
//! seeing exactly what happens at each stage. Useful for education and debugging.

use roup::debugger::{run_interactive_session, run_non_interactive, DebugConfig, DebugSession};
use roup::lexer::Language;
use roup::parser::Dialect;
use std::env;
use std::io::{self, Read};

#[derive(Debug)]
enum InputLanguage {
    C,
    FortranFree,
    FortranFixed,
}

fn detect_language(input: &str) -> Result<(Dialect, InputLanguage), String> {
    let trimmed = input.trim_start();
    let lower = trimmed.to_ascii_lowercase();

    // Try OpenACC detection first
    if let Ok((lang, _)) = detect_openacc_language(input) {
        return Ok((Dialect::OpenAcc, lang));
    }

    // Try OpenMP detection
    if lower.starts_with("#pragma omp") {
        return Ok((Dialect::OpenMp, InputLanguage::C));
    }

    // Fortran OpenMP free-form: !$omp (full form) or !$ (short form)
    if lower.starts_with("!$omp") || lower.starts_with("!$") {
        return Ok((Dialect::OpenMp, InputLanguage::FortranFree));
    }

    // Fortran OpenMP fixed-form: c$omp, *$omp, C$omp or short forms
    if let Some(first) = trimmed.chars().next() {
        let rest = trimmed[first.len_utf8()..].trim_start();
        let rest_lower = rest.to_ascii_lowercase();

        if (rest_lower.starts_with("$omp") || rest_lower.starts_with('$'))
            && matches!(first, 'c' | 'C' | '*')
        {
            return Ok((Dialect::OpenMp, InputLanguage::FortranFixed));
        }
    }

    Err(
        "Unable to detect directive dialect and language. Expected OpenMP or OpenACC directive."
            .to_string(),
    )
}

fn detect_openacc_language(input: &str) -> Result<(InputLanguage, String), String> {
    let trimmed = input.trim_start();
    let lower = trimmed.to_ascii_lowercase();

    // C/C++ pragma form
    if lower.starts_with("#pragma acc") {
        return Ok((InputLanguage::C, "#pragma acc".to_string()));
    }

    // Fortran free-form: !$acc (full form only - !$ alone could be OpenMP)
    if lower.starts_with("!$acc") {
        return Ok((InputLanguage::FortranFree, "!$acc".to_string()));
    }
    // Check for !$ short form ONLY if followed by acc-specific keyword
    if let Some(after_sentinel) = lower.strip_prefix("!$") {
        // Extract the word after !$ to check if it's an OpenACC keyword
        let after_sentinel = after_sentinel.trim_start();
        // Only accept short form if it's clearly OpenACC (starts with OpenACC directive names)
        // For safety, require the full !$acc form to avoid confusion with !$omp
        if after_sentinel.starts_with("acc") {
            return Ok((InputLanguage::FortranFree, "!$".to_string()));
        }
    }

    // Fortran fixed-form: c$acc, *$acc, C$acc or short forms c$, *$, C$
    if let Some(first) = trimmed.chars().next() {
        let rest = trimmed[first.len_utf8()..].trim_start();
        let rest_lower = rest.to_ascii_lowercase();

        // Check for full sentinel ($acc) first
        if rest_lower.starts_with("$acc") {
            let prefix = match first {
                'c' | 'C' => format!("{}$acc", first),
                '*' => "*$acc".to_string(),
                _ => return Err("Unable to detect OpenACC directive prefix".to_string()),
            };
            return Ok((InputLanguage::FortranFixed, prefix));
        }

        // Check for short sentinel ($) ONLY if followed by "acc"
        if let Some(after_dollar) = rest_lower.strip_prefix('$') {
            let after_dollar = after_dollar.trim_start();
            if after_dollar.starts_with("acc") {
                let prefix = match first {
                    'c' | 'C' => format!("{}$", first),
                    '*' => "*$".to_string(),
                    _ => return Err("Unable to detect OpenACC directive prefix".to_string()),
                };
                return Ok((InputLanguage::FortranFixed, prefix));
            }
        }
    }

    Err("Unable to detect OpenACC directive prefix".to_string())
}

fn print_usage(program: &str) {
    eprintln!("Usage: {} [OPTIONS] [INPUT]", program);
    eprintln!();
    eprintln!("Interactive step-by-step parser debugger for OpenMP and OpenACC directives.");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --omp, -o           Force OpenMP dialect (auto-detected by default)");
    eprintln!("  --acc, -a           Force OpenACC dialect (auto-detected by default)");
    eprintln!("  --non-interactive   Show all steps at once without interaction");
    eprintln!("  --help, -h          Show this help message");
    eprintln!();
    eprintln!("Input:");
    eprintln!("  Provide input as a command-line argument, or via stdin.");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  {} '#pragma omp parallel shared(x)'", program);
    eprintln!("  echo '#pragma omp for' | {}", program);
    eprintln!("  {} --acc '#pragma acc parallel async(1)'", program);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = &args[0];

    // Parse command-line arguments
    let mut forced_dialect: Option<Dialect> = None;
    let mut interactive = true;
    let mut input_arg: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--omp" | "-o" => forced_dialect = Some(Dialect::OpenMp),
            "--acc" | "-a" => forced_dialect = Some(Dialect::OpenAcc),
            "--non-interactive" | "-n" => interactive = false,
            "--help" | "-h" => {
                print_usage(program);
                return;
            }
            arg => {
                if arg.starts_with('-') {
                    eprintln!("Unknown option: {}", arg);
                    eprintln!();
                    print_usage(program);
                    std::process::exit(1);
                } else {
                    input_arg = Some(arg.to_string());
                }
            }
        }
        i += 1;
    }

    // Get input from argument or stdin
    let input = if let Some(arg_input) = input_arg {
        arg_input
    } else {
        let mut stdin_input = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut stdin_input) {
            eprintln!("Failed to read from stdin: {}", e);
            std::process::exit(1);
        }
        stdin_input
    };

    let trimmed = input.trim();
    if trimmed.is_empty() {
        eprintln!("Error: No input provided");
        eprintln!();
        print_usage(program);
        std::process::exit(1);
    }

    // Detect or use forced dialect and language
    let (dialect, input_language) = if let Some(forced) = forced_dialect {
        // Use forced dialect, but still need to detect language
        let lang = match trimmed.chars().next() {
            Some('#') => InputLanguage::C,
            Some('!') => InputLanguage::FortranFree,
            Some('c') | Some('C') | Some('*') => InputLanguage::FortranFixed,
            _ => InputLanguage::C,
        };
        (forced, lang)
    } else {
        match detect_language(trimmed) {
            Ok((d, l)) => (d, l),
            Err(e) => {
                eprintln!("Error: {}", e);
                eprintln!();
                eprintln!("Hint: Use --omp or --acc to force a specific dialect.");
                std::process::exit(1);
            }
        }
    };

    // Map InputLanguage to Language
    let language = match input_language {
        InputLanguage::C => Language::C,
        InputLanguage::FortranFree => Language::FortranFree,
        InputLanguage::FortranFixed => Language::FortranFixed,
    };

    // Create debug configuration
    let config = DebugConfig::new(dialect, language);

    // Create debug session
    let session = match DebugSession::new(trimmed, config) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to create debug session: {}", e);
            std::process::exit(1);
        }
    };

    // Run session
    if interactive {
        if let Err(e) = run_interactive_session(session) {
            eprintln!("Error during interactive session: {}", e);
            std::process::exit(1);
        }
    } else {
        run_non_interactive(&session);
    }
}
