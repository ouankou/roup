use roup::parser::openmp;
use std::io::{self, Read};

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
