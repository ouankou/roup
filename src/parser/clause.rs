use std::fmt;

/// Represents the different kinds of clauses in OpenMP directives
#[derive(Debug, PartialEq, Eq)]
pub enum ClauseKind<'a> {
    /// A clause without parameters, e.g., "nowait"
    Bare,
    /// A clause with parenthesized content, e.g., "private(a, b)"
    Parenthesized(&'a str),
}

/// Represents a single clause in an OpenMP directive
#[derive(Debug, PartialEq, Eq)]
pub struct Clause<'a> {
    /// The name of the clause (e.g., "private", "nowait")
    pub name: &'a str,
    /// The kind/type of this clause
    pub kind: ClauseKind<'a>,
}

impl<'a> Clause<'a> {
    /// Creates a new bare clause (no parameters)
    pub fn bare(name: &'a str) -> Self {
        Clause {
            name,
            kind: ClauseKind::Bare,
        }
    }

    /// Creates a new parenthesized clause
    pub fn parenthesized(name: &'a str, value: &'a str) -> Self {
        Clause {
            name,
            kind: ClauseKind::Parenthesized(value),
        }
    }
}

impl<'a> fmt::Display for Clause<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ClauseKind::Bare => write!(f, "{}", self.name),
            ClauseKind::Parenthesized(value) => write!(f, "{}({})", self.name, value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_bare_clause() {
        let clause = Clause::bare("nowait");
        assert_eq!(clause.name, "nowait");
        assert_eq!(clause.kind, ClauseKind::Bare);
    }

    #[test]
    fn creates_parenthesized_clause() {
        let clause = Clause::parenthesized("private", "a, b");
        assert_eq!(clause.name, "private");
        if let ClauseKind::Parenthesized(value) = clause.kind {
            assert_eq!(value, "a, b");
        } else {
            panic!("Expected Parenthesized clause");
        }
    }

    #[test]
    fn display_formats_clauses() {
        assert_eq!(Clause::bare("nowait").to_string(), "nowait");
        assert_eq!(
            Clause::parenthesized("private", "x, y").to_string(),
            "private(x, y)"
        );
    }
}
