use crate::parser::utils::*;

pub fn parse_modifiers(i: Span) -> nom::IResult<Span, Vec<Modifier>> {
    many0(nom::sequence::terminated(
        parse_modifier,
        nom::character::complete::space0,
    ))(i)
}

pub fn parse_modifier(i: Span) -> nom::IResult<Span, Modifier> {
    alt((public, visible))(i)
}

fn public(i: Span) -> nom::IResult<Span, Modifier> {
    let (i, _) = tag("public")(i)?;
    Ok((i, Modifier::Public))
}

fn visible(i: Span) -> nom::IResult<Span, Modifier> {
    let (i, _) = tag("visible")(i)?;
    Ok((i, Modifier::Visible))
}
