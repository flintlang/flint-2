use crate::ast::BinOp;
use crate::parser::utils::*;
use nom::branch::alt;
use nom::bytes::complete::tag;

pub fn get_operator_precedence(op: &BinOp) -> i32 {
    match op {
        BinOp::Plus => 20,
        BinOp::OverflowingPlus => 20,
        BinOp::Minus => 20,
        BinOp::OverflowingMinus => 20,
        BinOp::Times => 30,
        BinOp::OverflowingTimes => 30,
        BinOp::Power => 31,
        BinOp::Divide => 30,
        BinOp::Percent => 30,
        BinOp::Dot => 40,
        BinOp::Equal => 10,
        BinOp::PlusEqual => 10,
        BinOp::MinusEqual => 10,
        BinOp::TimesEqual => 10,
        BinOp::DivideEqual => 10,
        BinOp::DoubleEqual => 15,
        BinOp::NotEqual => 15,
        BinOp::LessThan => 15,
        BinOp::LessThanOrEqual => 15,
        BinOp::GreaterThan => 15,
        BinOp::GreaterThanOrEqual => 15,
        BinOp::Or => 11,
        BinOp::And => 12,
    }
}

pub fn parse_binary_op(i: Span) -> nom::IResult<Span, BinOp> {
    alt((
        double_equal_operator,
        not_equal_operator,
        plus_equal_operator,
        minus_equal_operator,
        times_equal_operator,
        divide_equal_operator,
        greater_than_equal_operator,
        less_than_equal_operator,
        plus_operator,
        minus_operator,
        power_operator,
        times_operator,
        divide_operator,
        dot_operator,
        equal_operator,
        less_than_operator,
        greater_than_operator,
        and_operator,
        or_operator,
    ))(i)
}

pub fn greater_than_equal_operator(i: Span) -> nom::IResult<Span, BinOp> {
    let (i, _) = tag(">=")(i)?;
    Ok((i, BinOp::GreaterThanOrEqual))
}

pub fn less_than_equal_operator(i: Span) -> nom::IResult<Span, BinOp> {
    let (i, _) = tag("<=")(i)?;
    Ok((i, BinOp::LessThanOrEqual))
}

pub fn power_operator(i: Span) -> nom::IResult<Span, BinOp> {
    let (i, _) = tag("**")(i)?;
    Ok((i, BinOp::Power))
}

pub fn times_operator(i: Span) -> nom::IResult<Span, BinOp> {
    let (i, _) = tag("*")(i)?;
    Ok((i, BinOp::Times))
}

pub fn divide_operator(i: Span) -> nom::IResult<Span, BinOp> {
    let (i, _) = tag("/")(i)?;
    Ok((i, BinOp::Divide))
}

pub fn and_operator(i: Span) -> nom::IResult<Span, BinOp> {
    let (i, _) = tag("&&")(i)?;
    Ok((i, BinOp::And))
}

pub fn or_operator(i: Span) -> nom::IResult<Span, BinOp> {
    let (i, _) = tag("||")(i)?;
    Ok((i, BinOp::Or))
}

pub fn double_equal_operator(i: Span) -> nom::IResult<Span, BinOp> {
    let (i, _) = tag("==")(i)?;
    Ok((i, BinOp::DoubleEqual))
}

pub fn not_equal_operator(i: Span) -> nom::IResult<Span, BinOp> {
    let (i, _) = tag("!=")(i)?;
    Ok((i, BinOp::NotEqual))
}

pub fn plus_operator(i: Span) -> nom::IResult<Span, BinOp> {
    let (i, _) = tag("+")(i)?;
    Ok((i, BinOp::Plus))
}

pub fn parse_comment(i: Span) -> nom::IResult<Span, Span> {
    let (i, _) = tag("//")(i)?;
    let (i, _) = nom::combinator::opt(nom::bytes::complete::is_not("\n"))(i)?;
    let (i, _) = tag("\n")(i)?;
    Ok((i, i))
}

pub fn minus_operator(i: Span) -> nom::IResult<Span, BinOp> {
    let (i, _) = tag("-")(i)?;
    Ok((i, BinOp::Minus))
}

pub fn plus_equal_operator(i: Span) -> nom::IResult<Span, BinOp> {
    let (i, _) = tag("+=")(i)?;
    Ok((i, BinOp::PlusEqual))
}

pub fn minus_equal_operator(i: Span) -> nom::IResult<Span, BinOp> {
    let (i, _) = tag("-=")(i)?;
    Ok((i, BinOp::MinusEqual))
}

pub fn times_equal_operator(i: Span) -> nom::IResult<Span, BinOp> {
    let (i, _) = tag("*=")(i)?;
    Ok((i, BinOp::TimesEqual))
}

pub fn divide_equal_operator(i: Span) -> nom::IResult<Span, BinOp> {
    let (i, _) = tag("/=")(i)?;
    Ok((i, BinOp::DivideEqual))
}

pub fn equal_operator(i: Span) -> nom::IResult<Span, BinOp> {
    let (i, _) = tag("=")(i)?;
    Ok((i, BinOp::Equal))
}

pub fn dot_operator(i: Span) -> nom::IResult<Span, BinOp> {
    let (i, _) = tag(".")(i)?;
    Ok((i, BinOp::Dot))
}

pub fn less_than_operator(i: Span) -> nom::IResult<Span, BinOp> {
    let (i, _) = tag("<")(i)?;
    Ok((i, BinOp::LessThan))
}

pub fn greater_than_operator(i: Span) -> nom::IResult<Span, BinOp> {
    let (i, _) = tag(">")(i)?;
    Ok((i, BinOp::GreaterThan))
}

pub fn left_brace(i: Span) -> nom::IResult<Span, Span> {
    tag("{")(i)
}

pub fn right_brace(i: Span) -> nom::IResult<Span, Span> {
    tag("}")(i)
}

pub fn left_square_bracket(i: Span) -> nom::IResult<Span, Span> {
    tag("[")(i)
}

pub fn right_square_bracket(i: Span) -> nom::IResult<Span, Span> {
    tag("]")(i)
}

pub fn colon(i: Span) -> nom::IResult<Span, Span> {
    tag(":")(i)
}

pub fn double_colon(i: Span) -> nom::IResult<Span, Span> {
    tag("::")(i)
}

pub fn left_parens(i: Span) -> nom::IResult<Span, Span> {
    tag("(")(i)
}

pub fn right_parens(i: Span) -> nom::IResult<Span, Span> {
    tag(")")(i)
}

pub fn at(i: Span) -> nom::IResult<Span, Span> {
    tag("@")(i)
}

pub fn right_arrow(i: Span) -> nom::IResult<Span, Span> {
    tag("->")(i)
}

pub fn left_arrow(i: Span) -> nom::IResult<Span, Span> {
    tag("<-")(i)
}

#[allow(dead_code)]
pub fn comma(i: Span) -> nom::IResult<Span, Span> {
    tag(",")(i)
}

#[allow(dead_code)]
pub fn semi_colon(i: Span) -> nom::IResult<Span, Span> {
    tag(";")(i)
}

#[allow(dead_code)]
pub fn double_slash(i: Span) -> nom::IResult<Span, Span> {
    tag("//")(i)
}

#[allow(dead_code)]
pub fn percent(i: Span) -> nom::IResult<Span, Span> {
    tag("//")(i)
}

#[allow(dead_code)]
pub fn double_dot(i: Span) -> nom::IResult<Span, Span> {
    tag("..")(i)
}

#[allow(dead_code)]
pub fn ampersand(i: Span) -> nom::IResult<Span, Span> {
    tag("&")(i)
}

#[allow(dead_code)]
pub fn bang(i: Span) -> nom::IResult<Span, Span> {
    tag("!")(i)
}

#[allow(dead_code)]
pub fn question(i: Span) -> nom::IResult<Span, Span> {
    tag("?")(i)
}

pub fn half_open_range(i: Span) -> nom::IResult<Span, Span> {
    tag("..<")(i)
}

pub fn closed_range(i: Span) -> nom::IResult<Span, Span> {
    tag("...")(i)
}
