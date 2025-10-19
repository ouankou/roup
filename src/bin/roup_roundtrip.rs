use roup::lexer::Language;
use roup::parser::{openacc, openmp};
use std::env;
use std::io::{self, Read};

#[derive(Debug)]
enum Dialect {
    OpenMP,
    OpenACC,
}

#[derive(Debug)]
enum InputLanguage {
    C,
    FortranFree,
    FortranFixed,
}

fn detect_openacc_language(input: &str) -> Result<(InputLanguage, String), String> {
    let trimmed = input.trim_start();
    let lower = trimmed.to_ascii_lowercase();

    // C/C++ pragma form
    if lower.starts_with("#pragma acc") {
        return Ok((InputLanguage::C, "#pragma acc".to_string()));
    }

    // Fortran free-form: !$acc (full form) or !$ (short form)
    // Check full form first to avoid matching short form prematurely
    if lower.starts_with("!$acc") {
        return Ok((InputLanguage::FortranFree, "!$acc".to_string()));
    }
    if lower.starts_with("!$") {
        return Ok((InputLanguage::FortranFree, "!$".to_string()));
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

        // Check for short sentinel ($) - only if not already matched $acc above
        if rest_lower.starts_with('$') {
            let prefix = match first {
                'c' | 'C' => format!("{}$", first),
                '*' => "*$".to_string(),
                _ => return Err("Unable to detect OpenACC directive prefix".to_string()),
            };
            return Ok((InputLanguage::FortranFixed, prefix));
        }
    }

    Err("Unable to detect OpenACC directive prefix".to_string())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Determine dialect from command line args
    let dialect = if args.len() > 1 && (args[1] == "--acc" || args[1] == "-a") {
        Dialect::OpenACC
    } else {
        Dialect::OpenMP
    };

    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        eprintln!("Failed to read stdin: {}", e);
        std::process::exit(1);
    }

    let trimmed = input.trim();
    if trimmed.is_empty() {
        eprintln!("No input provided");
        std::process::exit(1);
    }

    match dialect {
        Dialect::OpenMP => {
            // OpenMP mode (original behavior)
            let parser = openmp::parser();
            match parser.parse(trimmed) {
                Ok((rest, directive)) => {
                    if !rest.trim().is_empty() {
                        eprintln!("Unparsed trailing input: '{}'", rest.trim());
                        std::process::exit(1);
                    }
                    println!("{}", directive.to_pragma_string());
                }
                Err(e) => {
                    eprintln!("Parse error: {:?}", e);
                    std::process::exit(1);
                }
            }
        }
        Dialect::OpenACC => {
            // OpenACC mode with language detection
            let (language, prefix) = match detect_openacc_language(trimmed) {
                Ok(result) => result,
                Err(err) => {
                    eprintln!("{}", err);
                    std::process::exit(1);
                }
            };

            let parser = match language {
                InputLanguage::C => openacc::parser().with_language(Language::C),
                InputLanguage::FortranFree => {
                    openacc::parser().with_language(Language::FortranFree)
                }
                InputLanguage::FortranFixed => {
                    openacc::parser().with_language(Language::FortranFixed)
                }
            };

            match parser.parse(trimmed) {
                Ok((rest, directive)) => {
                    if !rest.trim().is_empty() {
                        eprintln!("Unparsed trailing input: '{}'", rest.trim());
                        std::process::exit(1);
                    }

                    println!("{}", directive.to_pragma_string_with_prefix(&prefix));
                }
                Err(e) => {
                    eprintln!("Parse error: {:?}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
