use std::env;
use std::io::{self, Read};

use roup::lexer::Language;
use roup::parser::{openacc, Directive};

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let mut language = Language::C;

    let mut idx = 1;
    while idx < args.len() {
        match args[idx].as_str() {
            "--lang" => {
                idx += 1;
                if idx >= args.len() {
                    return Err("--lang requires a value".into());
                }
                language = parse_language(&args[idx])?;
            }
            other => {
                return Err(format!("Unknown argument: {other}"));
            }
        }
        idx += 1;
    }

    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|err| format!("Failed to read stdin: {err}"))?;

    let raw_input = input.trim_end();
    if raw_input.trim().is_empty() {
        return Err("No input provided".into());
    }

    let parser = openacc::parser().with_language(language);

    let parse_input = match language {
        Language::C => raw_input.trim_start(),
        Language::FortranFree | Language::FortranFixed => raw_input,
    };

    let (rest, directive) = parser
        .parse(parse_input)
        .map_err(|err| format!("Parse error: {err:?}"))?;

    if !rest.trim().is_empty() {
        return Err(format!("Unparsed trailing input: '{}'", rest.trim()));
    }

    let prefix = detect_prefix(raw_input, language);
    let canonical = directive.to_pragma_string_with_prefix(&prefix);

    verify_roundtrip(&canonical, language, &directive)?;

    println!("{canonical}");

    Ok(())
}

fn parse_language(value: &str) -> Result<Language, String> {
    match value.to_ascii_lowercase().as_str() {
        "c" | "c++" | "cpp" => Ok(Language::C),
        "fortran-free" | "fortran" | "f90" => Ok(Language::FortranFree),
        "fortran-fixed" | "fixed" | "f" => Ok(Language::FortranFixed),
        other => Err(format!("Unsupported language '{other}'")),
    }
}

fn detect_prefix(input: &str, language: Language) -> String {
    let first_line = input.lines().next().unwrap_or(input);
    let indent_len = first_line
        .char_indices()
        .find(|(_, ch)| !ch.is_whitespace())
        .map(|(idx, _)| idx)
        .unwrap_or(first_line.len());
    let indent = &first_line[..indent_len];

    match language {
        Language::C => format!("{indent}#pragma acc"),
        Language::FortranFree | Language::FortranFixed => {
            let rest = &first_line[indent_len..];
            let sentinel_end = rest
                .char_indices()
                .find(|(_, ch)| ch.is_whitespace() || *ch == '&')
                .map(|(idx, _)| idx)
                .unwrap_or(rest.len());
            let sentinel = &rest[..sentinel_end];
            let sentinel = if sentinel.is_empty() {
                "!$acc"
            } else {
                sentinel
            };
            format!("{indent}{sentinel}")
        }
    }
}

fn verify_roundtrip(
    canonical: &str,
    language: Language,
    original: &Directive<'_>,
) -> Result<(), String> {
    let parser = openacc::parser().with_language(language);
    let parse_input = match language {
        Language::C => canonical.trim_start(),
        Language::FortranFree | Language::FortranFixed => canonical,
    };

    let (rest, reparsed) = parser
        .parse(parse_input)
        .map_err(|err| format!("Parse error during roundtrip validation: {err:?}"))?;

    if !rest.trim().is_empty() {
        return Err(format!(
            "Unparsed trailing input after roundtrip: '{}'",
            rest.trim()
        ));
    }

    if *original != reparsed {
        return Err("Roundtrip mismatch between original and reparsed directives".into());
    }

    Ok(())
}
