use crate::ast::*;
use crate::environment::Environment;
use crate::parser::operators::{left_brace, parse_comment, right_brace};
use crate::parser::statements::*;
use nom::{branch::alt, multi::many0};
use nom_locate::LocatedSpan;

pub type ParseResult = Result<(Module, Environment), String>;

pub type Span<'a> = LocatedSpan<&'a str>;

pub fn parse_code_block(i: Span) -> nom::IResult<Span, Vec<Statement>> {
    let (i, _) = left_brace(i)?;
    let (i, _) = multi_whitespace(i)?;
    let (i, statements) = parse_statements(i)?;
    let (i, _) = multi_whitespace(i)?;
    let (i, _) = right_brace(i)?;

    Ok((i, statements))
}

pub fn whitespace(i: Span) -> nom::IResult<Span, Span> {
    let (i, _) = many0(alt((
        nom::character::complete::space1,
        nom::character::complete::line_ending,
        parse_comment,
    )))(i)?;
    Ok((i, LocatedSpan::new("")))
}

pub fn multi_whitespace(i: Span) -> nom::IResult<Span, Span> {
    let (i, _) = many0(alt((nom::character::complete::multispace1, parse_comment)))(i)?;
    Ok((i, LocatedSpan::new("")))
}

#[cfg(test)]
mod test {

    use crate::parser::utils::*;
    use nom_locate::LocatedSpan;

    #[test]
    fn test_parse_whitespace() {
        let input = LocatedSpan::new("");
        let (rest, result) = whitespace(input).expect("Error parsing whitespace");
        assert_eq!(rest, LocatedSpan::new(""));
        assert_eq!(result, LocatedSpan::new(""));

        let input = LocatedSpan::new("   ");
        let (_rest, result) = whitespace(input).expect("Error parsing whitespace");
        assert_eq!(result, LocatedSpan::new(""));
    }
}
