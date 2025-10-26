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

fn detect_language(input: &str, dialect_prefix: &str) -> Result<(InputLanguage, String), String> {
    let trimmed = input.trim_start();
    let pragma_prefix = format!("#pragma {}", dialect_prefix);

    // C/C++ pragma form (case-insensitive)
    if trimmed.len() >= pragma_prefix.len()
        && trimmed[..pragma_prefix.len()].eq_ignore_ascii_case(&pragma_prefix)
    {
        return Ok((InputLanguage::C, pragma_prefix));
    }

    // Fortran free-form: !$<dialect> (full form) or !$ (short form)
    let full_sentinel = format!("!${}", dialect_prefix);
    // Check full form first to avoid matching short form prematurely (case-insensitive)
    if trimmed.len() >= full_sentinel.len()
        && trimmed[..full_sentinel.len()].eq_ignore_ascii_case(&full_sentinel)
    {
        // Preserve the actual case from input for the prefix
        return Ok((
            InputLanguage::FortranFree,
            trimmed[..full_sentinel.len()].to_string(),
        ));
    }
    // Check short form: !$
    if trimmed.len() >= 2 && trimmed[..2].eq_ignore_ascii_case("!$") {
        return Ok((InputLanguage::FortranFree, trimmed[..2].to_string()));
    }

    // Fortran fixed-form: c$<dialect>, *$<dialect>, C$<dialect> or short forms c$, *$, C$
    if let Some(first) = trimmed.chars().next() {
        let rest = trimmed[first.len_utf8()..].trim_start();

        // Check for full sentinel ($<dialect>) first (case-insensitive)
        let full_fixed_sentinel = format!("${}", dialect_prefix);
        if rest.len() >= full_fixed_sentinel.len()
            && rest[..full_fixed_sentinel.len()].eq_ignore_ascii_case(&full_fixed_sentinel)
        {
            let prefix = match first {
                'c' | 'C' | '*' => format!("{}{}", first, &rest[..full_fixed_sentinel.len()]),
                _ => {
                    return Err(format!(
                        "Unable to detect {} directive prefix",
                        dialect_prefix
                    ))
                }
            };
            return Ok((InputLanguage::FortranFixed, prefix));
        }

        // Check for short sentinel ($) - only if not already matched $<dialect> above
        if !rest.is_empty() && rest[..1].eq_ignore_ascii_case("$") {
            let prefix = match first {
                'c' | 'C' | '*' => format!("{}{}", first, &rest[..1]),
                _ => {
                    return Err(format!(
                        "Unable to detect {} directive prefix",
                        dialect_prefix
                    ))
                }
            };
            return Ok((InputLanguage::FortranFixed, prefix));
        }
    }

    Err(format!(
        "Unable to detect {} directive prefix",
        dialect_prefix
    ))
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

    // Detect language from input
    let dialect_prefix = match dialect {
        Dialect::OpenMP => "omp",
        Dialect::OpenACC => "acc",
    };

    let (language, prefix) = match detect_language(trimmed, dialect_prefix) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };

    // Create parser with appropriate language settings
    let parser = match dialect {
        Dialect::OpenMP => {
            let base = openmp::parser();
            match language {
                InputLanguage::C => base.with_language(Language::C),
                InputLanguage::FortranFree => base.with_language(Language::FortranFree),
                InputLanguage::FortranFixed => base.with_language(Language::FortranFixed),
            }
        }
        Dialect::OpenACC => {
            let base = openacc::parser();
            match language {
                InputLanguage::C => base.with_language(Language::C),
                InputLanguage::FortranFree => base.with_language(Language::FortranFree),
                InputLanguage::FortranFixed => base.with_language(Language::FortranFixed),
            }
        }
    };

    // Parse and output
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
