pub use crate::environment::Environment;
pub use crate::Parser::*;
pub use crate::AST::*;
pub use nom::{
    branch::alt, bytes::complete::tag, combinator::map, multi::many0, sequence::preceded,
};
pub use nom_locate::LocatedSpan;

pub type ParseResult = (Option<Module>, Environment);

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

    use crate::Parser::utils::*;

    #[test]
    fn test_parse_whitespace() {
        let input = "";
        let input = LocatedSpan::new(input);
        let result = whitespace(input);
        match result {
            Ok((c, _b)) => assert_eq!(c, LocatedSpan::new("")),
            Err(_) => assert_eq!(1, 0),
        }
    }
}
