use crate::Parser::utils::*;
use crate::Parser::calls::*;

pub fn parse_expression(i: Span) -> nom::IResult<Span, Expression> {
    alt((
        map(parse_inout_expression, |inout| {
            Expression::InoutExpression(inout)
        }),
        map(parse_external_call, |e| Expression::ExternalCall(e)),
        map(parse_cast_expression, |c| Expression::CastExpression(c)),
        map(parse_binary_expression, |be| {
            Expression::BinaryExpression(be)
        }),
        map(tag("self"), |_| Expression::SelfExpression),
        map(parse_subscript_expression, |s| {
            Expression::SubscriptExpression(s)
        }),
        map(parse_function_call, |f| Expression::FunctionCall(f)),
        map(parse_variable_declaration, |v| {
            Expression::VariableDeclaration(v)
        }),
        map(parse_literal, |l| Expression::Literal(l)),
        map(parse_identifier, |i| Expression::Identifier(i)),
        map(parse_bracketed_expression, |b| {
            Expression::BracketedExpression(b)
        }),
        map(parse_array_literal, |a| Expression::ArrayLiteral(a)),
        map(parse_dictionary_literal, |d| {
            Expression::DictionaryLiteral(d)
        }),
        map(parse_dictionary_empty_literal, |d| {
            Expression::DictionaryLiteral(d)
        }),
        map(parse_range_expression, |r| Expression::RangeExpression(r)),
    ))(i)
}

pub fn parse_expression_left(i: Span) -> nom::IResult<Span, Expression> {
    alt((
        map(parse_inout_expression, |inout| {
            Expression::InoutExpression(inout)
        }),
        map(parse_external_call, |e| Expression::ExternalCall(e)),
        map(parse_cast_expression, |c| Expression::CastExpression(c)),
        map(tag("self"), |_| Expression::SelfExpression),
        map(parse_subscript_expression, |s| {
            Expression::SubscriptExpression(s)
        }),
        map(parse_function_call, |f| Expression::FunctionCall(f)),
        map(parse_variable_declaration, |v| {
            Expression::VariableDeclaration(v)
        }),
        map(parse_literal, |l| Expression::Literal(l)),
        map(parse_identifier, |i| Expression::Identifier(i)),
        map(parse_bracketed_expression, |b| {
            Expression::BracketedExpression(b)
        }),
        map(parse_array_literal, |a| Expression::ArrayLiteral(a)),
        map(parse_dictionary_empty_literal, |a| {
            Expression::DictionaryLiteral(a)
        }),
        map(parse_range_expression, |r| Expression::RangeExpression(r)),
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
    let (i, function_call) = parse_function_call(i)?;
    let attempt_expression = AttemptExpression {
        kind: kind.fragment().to_string(),
        function_call,
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

    use crate::Parser::expressions::*;

    #[test]
    fn test_parse_inout_expression() {
        let input = LocatedSpan::new("&expression");
        let (rest, result) = parse_expression(input).expect("Error parsing inout expression");
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
        let (rest, result) = parse_expression(input).expect("Error parsing bracketed expression");
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
    //TODO: why is this function never called?
    fn test_parse_attempt_expression() {
        let input = LocatedSpan::new("try?foo()");
        let (rest, result) =
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
        let (rest, result) = parse_expression(input).expect("Error parsing subscript expression");
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
        let (rest, result) = parse_expression(input).expect("Error parsing binary expression");
        assert_eq!(result, Expression::BinaryExpression(BinaryExpression {
            lhs_expression: Box::new(Expression::Identifier(Identifier {
                token: String::from("x"),
                enclosing_type: None,
                line_info: LineInfo { line: 1, offset: 0 },
            })),

            rhs_expression: Box::new(Expression::Literal(Literal::IntLiteral(2))),
            op: BinOp::Power,
            line_info: LineInfo { line: 1, offset: 0 },
        }));
    }
}
