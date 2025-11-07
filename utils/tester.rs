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
                    ClauseKind::Parenthesized(ref value) => {
                        println!("Clause: {}({})", clause.name, value);
                    }
                    ClauseKind::VariableList(ref variables) => {
                        println!("Clause: {}(vars: {:?})", clause.name, variables);
                    }
                    ClauseKind::CopyinClause { ref modifier, ref variables } => {
                        println!("Clause: {}(modifier: {:?}, vars: {:?})", clause.name, modifier, variables);
                    }
                    ClauseKind::CopyoutClause { ref modifier, ref variables } => {
                        println!("Clause: {}(modifier: {:?}, vars: {:?})", clause.name, modifier, variables);
                    }
                    ClauseKind::CreateClause { ref modifier, ref variables } => {
                        println!("Clause: {}(modifier: {:?}, vars: {:?})", clause.name, modifier, variables);
                    }
                    ClauseKind::ReductionClause { ref operator, ref variables } => {
                        println!("Clause: {}(operator: {:?}, vars: {:?})", clause.name, operator, variables);
                    }
                    ClauseKind::GangClause { ref modifier, ref variables } => {
                        println!("Clause: {}(modifier: {:?}, vars: {:?})", clause.name, modifier, variables);
                    }
                    ClauseKind::WorkerClause { ref modifier, ref variables } => {
                        println!("Clause: {}(modifier: {:?}, vars: {:?})", clause.name, modifier, variables);
                    }
                    ClauseKind::VectorClause { ref modifier, ref variables } => {
                        println!("Clause: {}(modifier: {:?}, vars: {:?})", clause.name, modifier, variables);
                    }
                }
            }
        }
        Err(err) => {
            eprintln!("Error: {err}");
        }
    }
}
