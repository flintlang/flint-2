use crate::parser::expressions::*;
use crate::parser::operators::*;
use crate::parser::utils::*;

pub fn parse_literal(i: Span) -> nom::IResult<Span, Literal> {
    alt((
        address_literal,
        parse_boolean_literal,
        integer,
        float,
        string_literal,
    ))(i)
}

fn address_literal(i: Span) -> nom::IResult<Span, Literal> {
    let (i, _) = tag("0x")(i)?;
    let (i, address) = nom::character::complete::hex_digit1(i)?;
    let string = format!("0x{}", address.to_string());
    Ok((i, Literal::AddressLiteral(string)))
}

fn parse_boolean_literal(i: Span) -> nom::IResult<Span, Literal> {
    alt((true_literal, false_literal))(i)
}

fn true_literal(i: Span) -> nom::IResult<Span, Literal> {
    let (i, _) = tag("true")(i)?;
    Ok((i, Literal::BooleanLiteral(true)))
}

fn false_literal(i: Span) -> nom::IResult<Span, Literal> {
    let (i, _) = tag("false")(i)?;
    Ok((i, Literal::BooleanLiteral(false)))
}

pub fn integer(input: Span) -> nom::IResult<Span, Literal> {
    let (i, int) = nom::combinator::map_res(nom::character::complete::digit1, |s: Span| {
        s.fragment().parse::<u64>()
    })(input)?;
    Ok((i, Literal::IntLiteral(int)))
}

fn float(input: Span) -> nom::IResult<Span, Literal> {
    let (i, float) = nom::combinator::map_res(
        nom::combinator::recognize(nom::sequence::delimited(
            nom::character::complete::digit1,
            tag("."),
            nom::character::complete::digit1,
        )),
        |s: Span| s.fragment().parse::<f64>(),
    )(input)?;
    Ok((i, Literal::FloatLiteral(float)))
}

fn string_literal(i: Span) -> nom::IResult<Span, Literal> {
    let (i, _) = tag("\"")(i)?;
    let (i, string) = nom::bytes::complete::take_until("\"")(i)?;
    let (i, _) = tag("\"")(i)?;
    Ok((i, Literal::StringLiteral(string.to_string())))
}

pub fn parse_dictionary_empty_literal(i: Span) -> nom::IResult<Span, DictionaryLiteral> {
    let (i, _) = left_square_bracket(i)?;
    let (i, _) = colon(i)?;
    let (i, _) = right_square_bracket(i)?;
    Ok((i, DictionaryLiteral { elements: vec![] }))
}

pub fn parse_dictionary_literal(i: Span) -> nom::IResult<Span, DictionaryLiteral> {
    let (i, elements) = nom::multi::separated_nonempty_list(
        tag(","),
        nom::sequence::terminated(
            preceded(nom::character::complete::space0, parse_dictionary_element),
            nom::character::complete::space0,
        ),
    )(i)?;
    Ok((i, DictionaryLiteral { elements }))
}

fn parse_dictionary_element(i: Span) -> nom::IResult<Span, (Expression, Expression)> {
    let (i, expression1) = parse_expression_left(i)?;
    let (i, _) = colon(i)?;
    let (i, expression2) = parse_expression(i)?;
    Ok((i, (expression1, expression2)))
}

pub fn parse_array_literal(i: Span) -> nom::IResult<Span, ArrayLiteral> {
    let (i, _) = left_square_bracket(i)?;
    let (i, expressions) = nom::multi::separated_list(
        tag(","),
        nom::sequence::terminated(
            preceded(nom::character::complete::space0, parse_expression),
            nom::character::complete::space0,
        ),
    )(i)?;
    let (i, _) = right_square_bracket(i)?;
    let array_literal = ArrayLiteral {
        elements: expressions,
    };
    Ok((i, array_literal))
}
