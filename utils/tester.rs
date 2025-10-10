use roup::parser::{ClauseKind, Parser};

fn main() {
    let input = "#pragma omp parallel private(a, b) private(c)";
    let parser = Parser::default();
    match parser.parse(input) {
        Ok((_, directive)) => {
            println!("Directive: {}", directive.name);
            for clause in directive.clauses {
                match clause.kind {
                    ClauseKind::Bare => println!("Clause: {}", clause.name),
                    ClauseKind::Parenthesized(value) => {
                        println!("Clause: {}({})", clause.name, value);
                    }
                }
            }
        }
        Err(err) => {
            eprintln!("Error: {}", err);
        }
    }
}
