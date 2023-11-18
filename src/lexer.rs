use nom::{
    bytes::complete::tag,
    character::complete::{alphanumeric1},
    IResult,
};

pub fn lex_pragma(input: &str) -> IResult<&str, &str> {
    tag("#pragma")(input)
}

pub fn lex_omp(input: &str) -> IResult<&str, &str> {
    tag("omp")(input)
}

pub fn lex_directive(input: &str) -> IResult<&str, &str> {
    alphanumeric1(input)
}

pub fn lex_clause(input: &str) -> IResult<&str, &str> {
    alphanumeric1(input)
}

