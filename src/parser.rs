use super::lexer;
use nom::{
    bytes::complete::{tag, is_not},
    character::complete::{char, multispace1},
    combinator::{map, opt},
    sequence::{tuple},
    multi::{separated_list0, separated_list1},
    IResult,
};

pub fn parse_directive(input: &str) -> IResult<&str, &str> {
    let (input, _) = tag("parallel")(input)?;

    Ok((input, "parallel"))
}

pub fn parse_clause(input: &str) -> IResult<&str, &str> {
    let (input, _) = tag("private")(input)?;
    let (input, _) = char('(')(input)?;
    let (input, _) = separated_list1(tag(", "), is_not(",)"))(input)?;
    let (input, _) = char(')')(input)?;

    Ok((input, "private"))
}

pub fn parse_clauses(input: &str) -> IResult<&str, Vec<&str>> {
    separated_list0(tag(" "), parse_clause)(input)
}

pub fn parse_omp_directive(input: &str) -> IResult<&str, (&str, Vec<&str>)> {
    let (input, (_, _, _, directive, _, clauses)) = tuple((
        lexer::lex_pragma,
        multispace1,
        lexer::lex_omp,
        multispace1,
        parse_directive,
        opt(map(tuple((multispace1, parse_clauses)), |(_, clauses)| clauses)),
    ))(input)?;
    Ok((input, (directive, clauses.unwrap_or_default())))
}

