use roup::parser;

fn main() {
    let input = "#pragma omp parallel private(a, b) private(c)";
    match parser::parse_omp_directive(input) {
        Ok((_, (directive, clauses))) => {
            println!("Directive: {}", directive);
            for clause in clauses {
                println!("Clause: {}", clause);
            }
        }
        Err(err) => {
            eprintln!("Error: {}", err);
        }
    }
}

