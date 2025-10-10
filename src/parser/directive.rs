use super::{Clause};
use std::fmt;

/// Represents a complete OpenMP directive
#[derive(Debug, PartialEq, Eq)]
pub struct Directive<'a> {
    /// The name of the directive (e.g., "parallel", "for")
    pub name: &'a str,
    /// List of clauses attached to this directive
    pub clauses: Vec<Clause<'a>>,
}

impl<'a> fmt::Display for Directive<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#pragma omp {}", self.name)?;
        
        for (idx, clause) in self.clauses.iter().enumerate() {
            if idx == 0 {
                write!(f, " ")?;
            } else {
                write!(f, " ")?;
            }
            write!(f, "{}", clause)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Clause;

    #[test]
    fn creates_directive_with_clauses() {
        let directive = Directive {
            name: "parallel",
            clauses: vec![
                Clause::bare("nowait"),
                Clause::parenthesized("private", "x"),
            ],
        };

        assert_eq!(directive.name, "parallel");
        assert_eq!(directive.clauses.len(), 2);
    }

    #[test]
    fn display_formats_directive() {
        let directive = Directive {
            name: "parallel",
            clauses: vec![
                Clause::parenthesized("private", "x, y"),
                Clause::bare("nowait"),
            ],
        };
        assert_eq!(
            directive.to_string(),
            "#pragma omp parallel private(x, y) nowait"
        );
    }
}
