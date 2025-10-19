use roup::lexer::Language;
use roup::parser::{openacc, openmp, Dialect};
use std::env;
use std::io::{self, Read};

fn detect_language(input: &str) -> Language {
    let trimmed = input.trim_start();

    if trimmed.starts_with('#') {
        Language::C
    } else if trimmed.len() >= 2
        && matches!(
            &trimmed.as_bytes()[..2],
            [b'c', b'$'] | [b'C', b'$'] | [b'*', b'$'] | [b'!', b'$']
        )
    {
        // Fixed-form allows C$ACC / *$ACC / !$ACC sentinels in columns 1-6
        // Free-form directives also start with !$ACC once whitespace is trimmed.
        if trimmed.to_ascii_lowercase().starts_with('c') || trimmed.starts_with('*') {
            Language::FortranFixed
        } else {
            Language::FortranFree
        }
    } else {
        Language::C
    }
}

fn parse_language_from_env(value: Option<String>) -> Result<Option<Language>, String> {
    match value {
        Some(raw) => match raw.to_ascii_lowercase().as_str() {
            "c" => Ok(Some(Language::C)),
            "fortran-free" => Ok(Some(Language::FortranFree)),
            "fortran-fixed" => Ok(Some(Language::FortranFixed)),
            other => Err(format!("Unsupported ROUP_LANGUAGE value: {}", other)),
        },
        None => Ok(None),
    }
}

fn parse_dialect_from_env(value: Option<String>) -> Result<Dialect, String> {
    match value {
        Some(raw) => match raw.to_ascii_lowercase().as_str() {
            "openmp" => Ok(Dialect::OpenMp),
            "openacc" => Ok(Dialect::OpenAcc),
            other => Err(format!("Unsupported ROUP_DIALECT value: {}", other)),
        },
        None => Ok(Dialect::OpenMp),
    }
}

fn determine_prefix(input: &str, language: Language) -> String {
    let trimmed = input.trim_start();

    match language {
        Language::C => {
            let mut parts = trimmed.split_whitespace();
            let first = parts.next().unwrap_or("#pragma");
            let second = parts.next().unwrap_or("acc");

            if first.eq_ignore_ascii_case("#pragma") {
                format!("{} {}", first, second)
            } else {
                first.to_string()
            }
        }
        Language::FortranFree | Language::FortranFixed => trimmed
            .split_whitespace()
            .next()
            .unwrap_or("!$acc")
            .to_string(),
    }
}

fn main() {
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

    let dialect = match parse_dialect_from_env(env::var("ROUP_DIALECT").ok()) {
        Ok(dialect) => dialect,
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };

    let language = match parse_language_from_env(env::var("ROUP_LANGUAGE").ok()) {
        Ok(Some(language)) => language,
        Ok(None) => detect_language(trimmed),
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    };

    let parser = match dialect {
        Dialect::OpenMp => openmp::parser(),
        Dialect::OpenAcc => openacc::parser(),
    }
    .with_language(language);

    match parser.parse(trimmed) {
        Ok((rest, directive)) => {
            if !rest.trim().is_empty() {
                eprintln!("Unparsed trailing input: '{}'", rest.trim());
                std::process::exit(1);
            }
            let output = match dialect {
                Dialect::OpenMp => directive.to_pragma_string(),
                Dialect::OpenAcc => {
                    let prefix = determine_prefix(trimmed, language);
                    directive.to_pragma_string_with_prefix(&prefix)
                }
            };
            println!("{}", output);
        }
        Err(e) => {
            eprintln!("Parse error: {:?}", e);
            std::process::exit(1);
        }
    }
}
