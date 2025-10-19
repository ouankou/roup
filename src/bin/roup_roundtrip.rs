use roup::lexer::Language;
use roup::parser::{openacc, openmp, Dialect};
use std::io::{self, Read};

#[derive(Debug)]
struct InputConfig {
    language: Language,
    dialect: Dialect,
    prefix: String,
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

    let config = match detect_config(trimmed) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };

    let sanitized = sanitize_input(trimmed, &config);
    if sanitized.trim().is_empty() {
        eprintln!("No directive content after sanitization");
        std::process::exit(1);
    }

    let parser = match config.dialect {
        Dialect::OpenMp => openmp::parser(),
        Dialect::OpenAcc => openacc::parser(),
    }
    .with_language(config.language);

    match parser.parse(&sanitized) {
        Ok((rest, directive)) => {
            if !rest.trim().is_empty() {
                eprintln!("Unparsed trailing input: '{}'", rest.trim());
                std::process::exit(1);
            }
            println!("{}", format_output(&directive, &config));
        }
        Err(e) => {
            eprintln!("Parse error: {:?}", e);
            std::process::exit(1);
        }
    }
}

fn detect_config(input: &str) -> Result<InputConfig, String> {
    let trimmed = input.trim_start();

    if trimmed.len() >= 7 && trimmed[..7].eq_ignore_ascii_case("#pragma") {
        let after_pragma = &trimmed[7..];
        let after_pragma_trimmed = after_pragma.trim_start();
        if after_pragma_trimmed.len() < 3 {
            return Err("Directive missing dialect keyword after #pragma".into());
        }

        let dialect_keyword = &after_pragma_trimmed[..3];
        let whitespace_len = after_pragma.len() - after_pragma_trimmed.len();
        let prefix_len = 7 + whitespace_len + 3;
        let prefix = trimmed[..prefix_len].to_string();

        if dialect_keyword.eq_ignore_ascii_case("omp") {
            return Ok(InputConfig {
                language: Language::C,
                dialect: Dialect::OpenMp,
                prefix,
            });
        } else if dialect_keyword.eq_ignore_ascii_case("acc") {
            return Ok(InputConfig {
                language: Language::C,
                dialect: Dialect::OpenAcc,
                prefix,
            });
        }

        return Err(format!(
            "Unsupported pragma dialect: '{}'. Expected omp or acc.",
            dialect_keyword
        ));
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("!$omp") {
        return Ok(InputConfig {
            language: Language::FortranFree,
            dialect: Dialect::OpenMp,
            prefix: trimmed[..5].to_string(),
        });
    }
    if lower.starts_with("!$acc") {
        return Ok(InputConfig {
            language: Language::FortranFree,
            dialect: Dialect::OpenAcc,
            prefix: trimmed[..5].to_string(),
        });
    }
    if lower.starts_with("c$omp") || lower.starts_with("*$omp") {
        return Ok(InputConfig {
            language: Language::FortranFixed,
            dialect: Dialect::OpenMp,
            prefix: trimmed[..5].to_string(),
        });
    }
    if lower.starts_with("c$acc") || lower.starts_with("*$acc") {
        return Ok(InputConfig {
            language: Language::FortranFixed,
            dialect: Dialect::OpenAcc,
            prefix: trimmed[..5].to_string(),
        });
    }

    Err("Unable to detect directive dialect (expected omp/acc pragma or sentinel)".into())
}

fn sanitize_input(input: &str, config: &InputConfig) -> String {
    match config.language {
        Language::C => sanitize_c_like(input),
        Language::FortranFree | Language::FortranFixed => {
            sanitize_fortran_like(input, config.prefix.len())
        }
    }
}

fn sanitize_c_like(input: &str) -> String {
    input
        .lines()
        .map(|line| {
            let trimmed = line.trim_end();
            if let Some(idx) = trimmed.find("//") {
                trimmed[..idx].trim_end()
            } else {
                trimmed
            }
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn sanitize_fortran_like(input: &str, sentinel_len: usize) -> String {
    input
        .lines()
        .enumerate()
        .map(|(idx, line)| {
            let trimmed = line.trim_end();
            let skip = if idx == 0 {
                sentinel_len.min(trimmed.len())
            } else {
                0
            };

            let mut comment_idx = None;
            for (pos, ch) in trimmed.char_indices() {
                if pos < skip {
                    continue;
                }
                if ch == '!' {
                    comment_idx = Some(pos);
                    break;
                }
            }

            let without_comment = comment_idx
                .map(|idx| trimmed[..idx].trim_end())
                .unwrap_or(trimmed);
            without_comment
        })
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_output(directive: &roup::parser::Directive<'_>, config: &InputConfig) -> String {
    match config.language {
        Language::C => directive.to_pragma_string_with_prefix(match config.dialect {
            Dialect::OpenMp => "#pragma omp",
            Dialect::OpenAcc => "#pragma acc",
        }),
        Language::FortranFree | Language::FortranFixed => {
            let prefix = match config.dialect {
                Dialect::OpenMp => "!$omp",
                Dialect::OpenAcc => "!$acc",
            };
            directive.to_pragma_string_with_prefix(prefix)
        }
    }
}
