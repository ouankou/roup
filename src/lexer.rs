use nom::{
    bytes::complete::{tag, take_while1},
    IResult,
};

pub fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

// Skip whitespace and C-style (/* */) and C++-style (//) comments - zero or more
pub fn skip_space_and_comments(input: &str) -> nom::IResult<&str, &str> {
    let mut i = 0;
    let bytes = input.as_bytes();
    let len = bytes.len();

    while i < len {
        if bytes[i].is_ascii_whitespace() {
            // consume whitespace
            let ch = input[i..].chars().next().unwrap();
            i += ch.len_utf8();
            continue;
        }

        if i + 1 < len && &input[i..i + 2] == "/*" {
            // find closing */
            if let Some(end) = input[i + 2..].find("*/") {
                i += 2 + end + 2; // advance past */
                continue;
            } else {
                // unterminated comment: consume to end
                i = len;
                break;
            }
        }

        if i + 1 < len && &input[i..i + 2] == "//" {
            // consume to end of line
            if let Some(end) = input[i + 2..].find('\n') {
                i += 2 + end + 1;
            } else {
                i = len;
            }
            continue;
        }

        break;
    }

    Ok((&input[i..], &input[..0]))
}

// Same as above but require at least one whitespace or comment
pub fn skip_space1_and_comments(input: &str) -> nom::IResult<&str, &str> {
    let (rest, _) = skip_space_and_comments(input)?;
    if rest.len() == input.len() {
        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Space,
        )))
    } else {
        Ok((rest, &input[..0]))
    }
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
