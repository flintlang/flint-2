use crate::ast::{
    AttemptExpression, BinaryExpression, BracketedExpression, CastExpression, Expression,
    Identifier, InoutExpression, LineInfo, RangeExpression, SubscriptExpression,
};
use crate::parser::calls::*;
use crate::parser::declarations::parse_variable_declaration;
use crate::parser::identifiers::parse_identifier;
use crate::parser::literals::*;
use crate::parser::operators::*;
use crate::parser::types::*;
use crate::parser::utils::*;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::map;
use nom::sequence::preceded;

pub fn parse_expression(i: Span) -> nom::IResult<Span, Expression> {
    alt((
        map(parse_inout_expression, Expression::InoutExpression),
        map(parse_external_call, Expression::ExternalCall),
        map(parse_cast_expression, Expression::CastExpression),
        map(parse_binary_expression, Expression::BinaryExpression),
        map(tag(Identifier::SELF), |_| Expression::SelfExpression),
        map(parse_subscript_expression, Expression::SubscriptExpression),
        map(parse_attempt_expression, Expression::AttemptExpression),
        map(parse_function_call, Expression::FunctionCall),
        map(parse_variable_declaration, Expression::VariableDeclaration),
        map(parse_literal, Expression::Literal),
        map(parse_identifier, Expression::Identifier),
        map(parse_bracketed_expression, Expression::BracketedExpression),
        map(parse_array_literal, Expression::ArrayLiteral),
        map(parse_dictionary_literal, Expression::DictionaryLiteral),
        map(
            parse_dictionary_empty_literal,
            Expression::DictionaryLiteral,
        ),
        map(parse_range_expression, Expression::RangeExpression),
    ))(i)
}

pub fn parse_expression_left(i: Span) -> nom::IResult<Span, Expression> {
    alt((
        map(parse_inout_expression, Expression::InoutExpression),
        map(parse_external_call, Expression::ExternalCall),
        map(parse_cast_expression, Expression::CastExpression),
        map(tag(Identifier::SELF), |_| Expression::SelfExpression),
        map(parse_subscript_expression, Expression::SubscriptExpression),
        map(parse_function_call, Expression::FunctionCall),
        map(parse_attempt_expression, Expression::AttemptExpression),
        map(parse_variable_declaration, Expression::VariableDeclaration),
        map(parse_literal, Expression::Literal),
        map(parse_identifier, Expression::Identifier),
        map(parse_bracketed_expression, Expression::BracketedExpression),
        map(parse_array_literal, Expression::ArrayLiteral),
        map(
            parse_dictionary_empty_literal,
            Expression::DictionaryLiteral,
        ),
        map(parse_range_expression, Expression::RangeExpression),
    ))(i)
}

fn parse_subscript_expression(i: Span) -> nom::IResult<Span, SubscriptExpression> {
    let (i, identifier) = parse_identifier(i)?;
    let (i, _) = left_square_bracket(i)?;
    let (i, expression) = parse_expression(i)?;
    let (i, _) = right_square_bracket(i)?;
    let subscript_expression = SubscriptExpression {
        base_expression: identifier,
        index_expression: Box::new(expression),
    };
    Ok((i, subscript_expression))
}

fn parse_range_expression(i: Span) -> nom::IResult<Span, RangeExpression> {
    let (i, _) = left_parens(i)?;
    let (i, start_literal) = parse_literal(i)?;
    let (i, op) = alt((half_open_range, closed_range))(i)?;
    let (i, end_literal) = parse_literal(i)?;
    let (i, _) = right_parens(i)?;
    let range_expression = RangeExpression {
        start_expression: Box::new(Expression::Literal(start_literal)),
        end_expression: Box::new(Expression::Literal(end_literal)),
        op: op.to_string(),
    };
    Ok((i, range_expression))
}

fn parse_cast_expression(i: Span) -> nom::IResult<Span, CastExpression> {
    let (i, _) = tag("cast")(i)?;
    let (i, _) = whitespace(i)?;
    let (i, expression) = parse_expression(i)?;
    let (i, _) = whitespace(i)?;
    let (i, _) = tag("to")(i)?;
    let (i, _) = whitespace(i)?;
    let (i, cast_type) = parse_type(i)?;
    let cast_expression = CastExpression {
        expression: Box::new(expression),
        cast_type,
    };
    Ok((i, cast_expression))
}

fn parse_inout_expression(i: Span) -> nom::IResult<Span, InoutExpression> {
    let (i, _) = ampersand(i)?;
    let (i, expression) = parse_expression(i)?;
    let inout_expression = InoutExpression {
        ampersand_token: "&".to_string(),
        expression: Box::new(expression),
    };
    Ok((i, inout_expression))
}

fn parse_bracketed_expression(i: Span) -> nom::IResult<Span, BracketedExpression> {
    let (i, _) = left_parens(i)?;
    let (i, expression) = parse_expression(i)?;
    let (i, _) = right_parens(i)?;
    let bracketed_expression = BracketedExpression {
        expression: Box::new(expression),
    };
    Ok((i, bracketed_expression))
}

fn parse_attempt_expression(i: Span) -> nom::IResult<Span, AttemptExpression> {
    let (i, _) = tag("try")(i)?;
    let (i, kind) = alt((bang, question))(i)?;
    let (i, _) = whitespace(i)?;
    let (i, function_call) = parse_function_call(i)?;
    let attempt_expression = AttemptExpression {
        kind: kind.fragment().to_string(),
        function_call,
        predicate: None,
    };
    Ok((i, attempt_expression))
}

pub fn parse_binary_expression(input: Span) -> nom::IResult<Span, BinaryExpression> {
    let (i, _) = parse_expression_left(input)?;
    let _ = preceded(whitespace, parse_binary_op)(i)?;
    let (i, expression) = parse_binary_expression_precedence(input, 0)?;
    if let Expression::BinaryExpression(b) = expression {
        Ok((i, b))
    } else {
        unimplemented!()
    }
}

pub fn parse_binary_expression_precedence(
    i: Span,
    operator_precedence: i32,
) -> nom::IResult<Span, Expression> {
    let line_info = LineInfo {
        line: i.location_line(),
        offset: i.location_offset(),
    };
    let (i, lhs_expression) = parse_expression_left(i)?;
    let mut lhs_expression = lhs_expression;
    let mut result = lhs_expression.clone();
    let mut input = i;
    loop {
        let (i, op) = nom::combinator::opt(preceded(whitespace, parse_binary_op))(input)?;
        if op.is_none() {
            break;
        }
        let op = op.unwrap();
        let current_precedence = get_operator_precedence(&op);
        if current_precedence < operator_precedence {
            break;
        }

        let next_precedence = if op.is_left() {
            current_precedence + 1
        } else {
            current_precedence
        };
        let (i, _) = whitespace(i)?;
        let (i, rhs) = parse_binary_expression_precedence(i, next_precedence)?;
        input = i;
        let binary_expression = BinaryExpression {
            lhs_expression: Box::new(lhs_expression.clone()),
            op: op.clone(),
            rhs_expression: Box::new(rhs),
            line_info: line_info.clone(),
        };
        result = Expression::BinaryExpression(binary_expression);
        lhs_expression = result.clone();
    }
    Ok((input, result))
}

#[cfg(test)]
mod test {

    use crate::ast::*;
    use crate::parser::expressions::*;
    use nom_locate::LocatedSpan;

    #[test]
    fn test_parse_inout_expression() {
        let input = LocatedSpan::new("&expression");
        let (_rest, result) = parse_expression(input).expect("Error parsing inout expression");
        assert_eq!(
            result,
            Expression::InoutExpression(InoutExpression {
                ampersand_token: String::from("&"),
                expression: Box::new(Expression::Identifier(Identifier {
                    token: String::from("expression"),
                    enclosing_type: None,
                    line_info: LineInfo { line: 1, offset: 0 },
                })),
            })
        );
    }

    #[test]
    fn test_parse_bracketed_expression() {
        let input = LocatedSpan::new("(expression)");
        let (_rest, result) = parse_expression(input).expect("Error parsing bracketed expression");
        assert_eq!(
            result,
            Expression::BracketedExpression(BracketedExpression {
                expression: Box::new(Expression::Identifier(Identifier {
                    token: String::from("expression"),
                    enclosing_type: None,
                    line_info: LineInfo { line: 1, offset: 0 },
                }))
            })
        );
    }

    #[test]
    fn test_parse_attempt_expression() {
        let input = LocatedSpan::new("try?foo()");
        let (_rest, result) =
            parse_attempt_expression(input).expect("Error parsing attempt expression");
        assert_eq!(
            result,
            AttemptExpression {
                kind: String::from("?"),
                function_call: FunctionCall {
                    identifier: Identifier {
                        token: String::from("foo"),
                        enclosing_type: None,
                        line_info: LineInfo { line: 1, offset: 0 },
                    },

                    arguments: vec![],
                    mangled_identifier: None,
                },
            }
        );
    }

    #[test]
    fn test_parse_subscript_expression() {
        let input = LocatedSpan::new("base[index]");
        let (_rest, result) = parse_expression(input).expect("Error parsing subscript expression");
        assert_eq!(
            result,
            Expression::SubscriptExpression(SubscriptExpression {
                base_expression: Identifier {
                    token: String::from("base"),
                    enclosing_type: None,
                    line_info: LineInfo { line: 1, offset: 0 },
                },

                index_expression: Box::new(Expression::Identifier(Identifier {
                    token: String::from("index"),
                    enclosing_type: None,
                    line_info: LineInfo { line: 1, offset: 0 },
                })),
            })
        );
    }

    #[test]
    fn test_parse_binary_expression() {
        let input = LocatedSpan::new("x ** 2");
        let (_rest, result) = parse_expression(input).expect("Error parsing binary expression");
        assert_eq!(
            result,
            Expression::BinaryExpression(BinaryExpression {
                lhs_expression: Box::new(Expression::Identifier(Identifier {
                    token: String::from("x"),
                    enclosing_type: None,
                    line_info: LineInfo { line: 1, offset: 0 },
                })),

                rhs_expression: Box::new(Expression::Literal(Literal::IntLiteral(2))),
                op: BinOp::Power,
                line_info: LineInfo { line: 1, offset: 0 },
            })
        );
    }

    #[test]
    fn test_parse_self_expression() {
        let input = LocatedSpan::new("self.rectangle.width");
        let (_rest, result) = parse_expression(input).expect("Error parsing self expression");
        assert_eq!(
            result,
            Expression::BinaryExpression(BinaryExpression {
                lhs_expression: Box::new(Expression::BinaryExpression(BinaryExpression {
                    lhs_expression: Box::new(Expression::SelfExpression),
                    rhs_expression: Box::new(Expression::Identifier(Identifier {
                        token: String::from("rectangle"),
                        enclosing_type: None,
                        line_info: LineInfo { line: 1, offset: 5 }
                    })),
                    op: BinOp::Dot,
                    line_info: LineInfo { line: 1, offset: 0 }
                })),
                rhs_expression: Box::new(Expression::Identifier(Identifier {
                    token: String::from("width"),
                    enclosing_type: None,
                    line_info: LineInfo {
                        line: 1,
                        offset: 15
                    }
                })),
                op: BinOp::Dot,
                line_info: LineInfo { line: 1, offset: 0 }
            })
        );
    }

    #[test]
    fn test_parse_cast_expression() {
        let input = LocatedSpan::new("cast x to Int");
        let (_rest, result) = parse_expression(input).expect("Error parsing cast expression");
        assert_eq!(
            result,
            Expression::CastExpression(CastExpression {
                expression: Box::new(Expression::Identifier(Identifier {
                    token: String::from("x"),
                    enclosing_type: None,
                    line_info: LineInfo { line: 1, offset: 0 },
                })),

                cast_type: Type::Int
            })
        );
    }

    #[test]
    fn test_parse_range_expression() {
        let input = LocatedSpan::new("(0..<3)");
        let (_rest, result) = parse_expression(input).expect("Error parsing range expression");
        assert_eq!(
            result,
            Expression::RangeExpression(RangeExpression {
                start_expression: Box::new(Expression::Literal(Literal::IntLiteral(0))),
                end_expression: Box::new(Expression::Literal(Literal::IntLiteral(3))),
                op: String::from("..<")
            })
        );
    }
}
