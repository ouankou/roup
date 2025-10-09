use nom::{
    bytes::complete::{tag, take_while1},
    IResult,
};

pub fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn lex_identifier(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| is_identifier_char(c))(input)
}

pub fn lex_pragma(input: &str) -> IResult<&str, &str> {
    tag("#pragma")(input)
}

pub fn lex_omp(input: &str) -> IResult<&str, &str> {
    tag("omp")(input)
}

pub fn lex_directive(input: &str) -> IResult<&str, &str> {
    lex_identifier(input)
}

pub fn lex_clause(input: &str) -> IResult<&str, &str> {
    lex_identifier(input)
}

pub fn lex_identifier_token(input: &str) -> IResult<&str, &str> {
    lex_identifier(input)
}
