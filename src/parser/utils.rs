pub use crate::ast::*;
pub use crate::environment::Environment;
use crate::parser::statements::*;
pub use crate::parser::*;
pub use nom::{
    branch::alt, bytes::complete::tag, combinator::map, multi::many0, sequence::preceded,
};
pub use nom_locate::LocatedSpan;

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
