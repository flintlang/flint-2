use crate::Parser::utils::*;

pub fn parse_modifiers(i: Span) -> nom::IResult<Span, Vec<std::string::String>> {
    many0(nom::sequence::terminated(
        parse_modifier,
        nom::character::complete::space0,
    ))(i)
}

fn parse_modifier(i: Span) -> nom::IResult<Span, std::string::String> {
    alt((public, visible))(i)
}

fn public(i: Span) -> nom::IResult<Span, std::string::String> {
    let (i, public) = tag("public")(i)?;
    Ok((i, public.to_string()))
}

fn visible(i: Span) -> nom::IResult<Span, std::string::String> {
    let (i, visible) = tag("visible")(i)?;
    Ok((i, visible.to_string()))
}
