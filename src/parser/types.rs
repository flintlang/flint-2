use std::collections::HashSet;

use crate::parser::identifiers::parse_identifier;
use crate::parser::literals::*;
use crate::parser::operators::*;
use crate::parser::utils::*;
use crate::ast::{Type, SolidityType, FixedSizedArrayType, Literal, InoutType, ArrayType, DictionaryType, TypeAnnotation};
use nom::bytes::complete::tag;
use nom::combinator::map;
use nom::branch::alt;
use nom::sequence::preceded;

pub fn parse_type_annotation(i: Span) -> nom::IResult<Span, TypeAnnotation> {
    let (i, colon) = colon(i)?;
    let (i, _) = whitespace(i)?;
    let (i, type_assigned) = preceded(whitespace, parse_type)(i)?;
    let type_annotation = TypeAnnotation {
        type_assigned,
        colon: colon.to_string(),
    };
    Ok((i, type_annotation))
}

pub fn parse_type(i: Span) -> nom::IResult<Span, Type> {
    alt((
        parse_fixed_array_type,
        parse_array_type,
        parse_dictionary_type,
        parse_self_type,
        parse_basic_type,
        parse_inout_type,
        parse_solidity_type,
        parse_identifier_type,
    ))(i)
}

pub fn parse_identifier_type(i: Span) -> nom::IResult<Span, Type> {
    let (i, identifier) = parse_identifier(i)?;
    if is_basic_type(identifier.token.as_str()) {
        let basic_type = match identifier.token.as_str() {
            "Int" => Type::Int,
            "Address" => Type::Address,
            "Bool" => Type::Bool,
            _ => Type::Address,
        };
        return Ok((i, basic_type));
    }
    Ok((i, Type::UserDefinedType(identifier)))
}

fn is_basic_type(basic_type: &str) -> bool {
    let basic_types: HashSet<&'static str> = ["Address", "Int", "String", "Void", "Bool", "Event"]
        .iter()
        .cloned()
        .collect();
    basic_types.contains(basic_type)
}

fn parse_self_type(i: Span) -> nom::IResult<Span, Type> {
    let (i, _) = tag("Self")(i)?;
    Ok((i, Type::SelfType))
}

fn parse_solidity_type(i: Span) -> nom::IResult<Span, Type> {
    alt((
        parse_solidity_type_first_part,
        parse_solidity_type_second_part,
        parse_solidity_type_third_part,
        parse_solidity_type_fourth_part,
        parse_solidity_type_fifth_part,
        map(tag("address"), |_| Type::Solidity(SolidityType::ADDRESS)),
        map(tag("string"), |_| Type::Solidity(SolidityType::STRING)),
        map(tag("bool"), |_| Type::Solidity(SolidityType::BOOL)),
    ))(i)
}

fn parse_solidity_type_first_part(i: Span) -> nom::IResult<Span, Type> {
    alt((
        map(tag("int8"), |_| Type::Solidity(SolidityType::INT8)),
        map(tag("int16"), |_| Type::Solidity(SolidityType::INT16)),
        map(tag("int24"), |_| Type::Solidity(SolidityType::INT24)),
        map(tag("int32"), |_| Type::Solidity(SolidityType::INT32)),
        map(tag("int40"), |_| Type::Solidity(SolidityType::INT40)),
        map(tag("int48"), |_| Type::Solidity(SolidityType::INT48)),
        map(tag("int56"), |_| Type::Solidity(SolidityType::INT56)),
        map(tag("int64"), |_| Type::Solidity(SolidityType::INT64)),
        map(tag("int72"), |_| Type::Solidity(SolidityType::INT72)),
        map(tag("int80"), |_| Type::Solidity(SolidityType::INT80)),
        map(tag("int88"), |_| Type::Solidity(SolidityType::INT88)),
        map(tag("int96"), |_| Type::Solidity(SolidityType::INT96)),
        map(tag("int104"), |_| Type::Solidity(SolidityType::INT104)),
        map(tag("int112"), |_| Type::Solidity(SolidityType::INT112)),
        map(tag("int120"), |_| Type::Solidity(SolidityType::INT120)),
    ))(i)
}

fn parse_solidity_type_second_part(i: Span) -> nom::IResult<Span, Type> {
    alt((
        map(tag("int128"), |_| Type::Solidity(SolidityType::INT128)),
        map(tag("int136"), |_| Type::Solidity(SolidityType::INT136)),
        map(tag("int144"), |_| Type::Solidity(SolidityType::INT144)),
        map(tag("int152"), |_| Type::Solidity(SolidityType::INT152)),
        map(tag("int160"), |_| Type::Solidity(SolidityType::INT160)),
        map(tag("int168"), |_| Type::Solidity(SolidityType::INT168)),
        map(tag("int176"), |_| Type::Solidity(SolidityType::INT176)),
        map(tag("int184"), |_| Type::Solidity(SolidityType::INT184)),
        map(tag("int192"), |_| Type::Solidity(SolidityType::INT192)),
        map(tag("int200"), |_| Type::Solidity(SolidityType::INT200)),
        map(tag("int208"), |_| Type::Solidity(SolidityType::INT208)),
        map(tag("int216"), |_| Type::Solidity(SolidityType::INT216)),
        map(tag("int224"), |_| Type::Solidity(SolidityType::INT224)),
        map(tag("int232"), |_| Type::Solidity(SolidityType::INT232)),
    ))(i)
}

fn parse_solidity_type_third_part(i: Span) -> nom::IResult<Span, Type> {
    alt((
        map(tag("int240"), |_| Type::Solidity(SolidityType::INT240)),
        map(tag("int248"), |_| Type::Solidity(SolidityType::INT248)),
        map(tag("int256"), |_| Type::Solidity(SolidityType::INT256)),
        map(tag("uint8"), |_| Type::Solidity(SolidityType::UINT8)),
        map(tag("uint16"), |_| Type::Solidity(SolidityType::UINT16)),
        map(tag("uint24"), |_| Type::Solidity(SolidityType::UINT24)),
        map(tag("uint32"), |_| Type::Solidity(SolidityType::UINT32)),
        map(tag("uint40"), |_| Type::Solidity(SolidityType::UINT40)),
        map(tag("uint48"), |_| Type::Solidity(SolidityType::UINT48)),
        map(tag("uint56"), |_| Type::Solidity(SolidityType::UINT56)),
        map(tag("uint64"), |_| Type::Solidity(SolidityType::UINT64)),
        map(tag("uint72"), |_| Type::Solidity(SolidityType::UINT72)),
        map(tag("uint80"), |_| Type::Solidity(SolidityType::UINT80)),
        map(tag("uint88"), |_| Type::Solidity(SolidityType::UINT88)),
    ))(i)
}

fn parse_solidity_type_fourth_part(i: Span) -> nom::IResult<Span, Type> {
    alt((
        map(tag("uint96"), |_| Type::Solidity(SolidityType::UINT96)),
        map(tag("uint104"), |_| Type::Solidity(SolidityType::UINT104)),
        map(tag("uint112"), |_| Type::Solidity(SolidityType::UINT112)),
        map(tag("uint120"), |_| Type::Solidity(SolidityType::UINT120)),
        map(tag("uint128"), |_| Type::Solidity(SolidityType::UINT128)),
        map(tag("uint136"), |_| Type::Solidity(SolidityType::UINT136)),
        map(tag("uint144"), |_| Type::Solidity(SolidityType::UINT144)),
        map(tag("uint152"), |_| Type::Solidity(SolidityType::UINT152)),
        map(tag("uint160"), |_| Type::Solidity(SolidityType::UINT160)),
        map(tag("uint168"), |_| Type::Solidity(SolidityType::UINT168)),
        map(tag("uint176"), |_| Type::Solidity(SolidityType::UINT176)),
        map(tag("uint184"), |_| Type::Solidity(SolidityType::UINT184)),
    ))(i)
}

fn parse_solidity_type_fifth_part(i: Span) -> nom::IResult<Span, Type> {
    alt((
        map(tag("uint192"), |_| Type::Solidity(SolidityType::UINT192)),
        map(tag("uint200"), |_| Type::Solidity(SolidityType::UINT200)),
        map(tag("uint208"), |_| Type::Solidity(SolidityType::UINT208)),
        map(tag("uint216"), |_| Type::Solidity(SolidityType::UINT216)),
        map(tag("uint224"), |_| Type::Solidity(SolidityType::UINT224)),
        map(tag("uint232"), |_| Type::Solidity(SolidityType::UINT232)),
        map(tag("uint240"), |_| Type::Solidity(SolidityType::UINT240)),
        map(tag("uint248"), |_| Type::Solidity(SolidityType::UINT248)),
        map(tag("uint256"), |_| Type::Solidity(SolidityType::UINT256)),
    ))(i)
}

fn parse_fixed_array_type(i: Span) -> nom::IResult<Span, Type> {
    let (i, identifier) = parse_identifier_type(i)?;
    let (i, literal) =
        nom::sequence::delimited(left_square_bracket, integer, right_square_bracket)(i)?;

    let size = match literal {
        Literal::IntLiteral(i) => i,
        _ => unimplemented!(),
    };

    let fixed_sized_array_type = FixedSizedArrayType {
        key_type: Box::new(identifier),
        size,
    };
    Ok((i, Type::FixedSizedArrayType(fixed_sized_array_type)))
}

fn parse_inout_type(i: Span) -> nom::IResult<Span, Type> {
    let (i, _) = tag("inout")(i)?;
    let (i, _) = whitespace(i)?;
    let (i, key_type) = parse_type(i)?;
    let inout_type = InoutType {
        key_type: Box::new(key_type),
    };
    Ok((i, Type::InoutType(inout_type)))
}

fn parse_array_type(i: Span) -> nom::IResult<Span, Type> {
    let (i, key_type) =
        nom::sequence::delimited(left_square_bracket, parse_type, right_square_bracket)(i)?;
    let array_type = ArrayType {
        key_type: Box::new(key_type),
    };
    Ok((i, Type::ArrayType(array_type)))
}

fn parse_dictionary_type(i: Span) -> nom::IResult<Span, Type> {
    let (i, _) = left_square_bracket(i)?;
    let (i, key_type) = parse_type(i)?;
    let (i, _) = colon(i)?;
    let (i, _) = whitespace(i)?;
    let (i, value_type) = parse_type(i)?;
    let (i, _) = right_square_bracket(i)?;
    let dictionary_type = DictionaryType {
        key_type: Box::new(key_type),
        value_type: Box::new(value_type),
    };
    Ok((i, Type::DictionaryType(dictionary_type)))
}

fn parse_basic_type(i: Span) -> nom::IResult<Span, Type> {
    let (i, base_type) = alt((
        map(tag("Bool"), |_| Type::Bool),
        map(tag("Int"), |_| Type::Int),
        map(tag("String"), |_| Type::String),
        map(tag("Address"), |_| Type::Address),
    ))(i)?;
    Ok((i, base_type))
}

#[cfg(test)]
mod test {

    use crate::ast::*;
    use crate::parser::types::*;
    use nom_locate::LocatedSpan;

    #[test]
    fn test_parse_int_type() {
        let input = "Int";
        let input = LocatedSpan::new(input);
        let result = parse_type(input);
        match result {
            Ok((_c, b)) => assert_eq!(b, Type::Int),
            Err(_) => assert_eq!(1, 0),
        }
    }

    #[test]
    fn test_parse_address_type() {
        let input = "Address";
        let input = LocatedSpan::new(input);
        let result = parse_type(input);
        match result {
            Ok((_c, b)) => assert_eq!(b, Type::Address),
            Err(_) => assert_eq!(1, 0),
        }
    }

    #[test]
    fn test_parse_bool_type() {
        let input = "Bool";
        let input = LocatedSpan::new(input);
        let result = parse_type(input);
        match result {
            Ok((_c, b)) => assert_eq!(b, Type::Bool),
            Err(_) => assert_eq!(1, 0),
        }
    }

    #[test]
    fn test_parse_string_type() {
        let input = "String";
        let input = LocatedSpan::new(input);
        let result = parse_type(input);
        match result {
            Ok((_c, b)) => assert_eq!(b, Type::String),
            Err(_) => assert_eq!(1, 0),
        }
    }
}
