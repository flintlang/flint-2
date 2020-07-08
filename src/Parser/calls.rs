use crate::Parser::utils::*;

pub fn parse_external_call(i: Span) -> nom::IResult<Span, ExternalCall> {
    let (i, _) = tag("call")(i)?;
    let (i, _) = whitespace(i)?;
    let function_arguments = vec![];
    let (i, function_call) = parse_binary_expression(i)?;
    let external_call = ExternalCall {
        arguments: function_arguments,
        function_call,
        external_trait_name: None,
    };
    Ok((i, external_call))
}

pub fn parse_function_call(i: Span) -> nom::IResult<Span, FunctionCall> {
    let (i, identifier) = parse_identifier(i)?;
    let (i, arguments) = parse_function_call_arguments(i)?;
    let function_call = FunctionCall {
        identifier,
        arguments,
        mangled_identifier: None,
    };
    Ok((i, function_call))
}

fn parse_function_call_arguments(i: Span) -> nom::IResult<Span, Vec<FunctionArgument>> {
    let (i, _) = left_parens(i)?;
    let (i, arguments) = nom::multi::separated_list(
        tag(","),
        preceded(whitespace, parse_function_call_argument),
    )(i)?;
    let (i, _) = whitespace(i)?;
    let (i, _) = right_parens(i)?;
    Ok((i, arguments))
}

fn parse_function_call_argument(i: Span) -> nom::IResult<Span, FunctionArgument> {
    alt((
        map(
            nom::sequence::separated_pair(
                parse_identifier,
                colon,
                preceded(whitespace, parse_expression),
            ),
            |(i, e)| FunctionArgument {
                identifier: Some(i),
                expression: e,
            },
        ),
        map(parse_expression, |e| FunctionArgument {
            identifier: None,
            expression: e,
        }),
    ))(i)
}

#[cfg(test)]
mod test {

    use crate::Parser::calls::*;

    #[test]
    fn test_parse_function_call() {
        let input = LocatedSpan::new("foo(first, second)");
        let (rest, result) = parse_function_call(input).expect("Error parsing function call");
        assert_eq!(result, FunctionCall {
            identifier: Identifier {
                token: String::from("foo"),
                enclosing_type: None,
                line_info: LineInfo { line : 1, offset : 0}            
            },

            arguments: vec![FunctionArgument {
                identifier: None,
                expression: Expression::Identifier(Identifier {
                    token: String::from("first"),
                    enclosing_type: None,
                    line_info: LineInfo { line : 1, offset : 4}
                })
            }, FunctionArgument {
                identifier: None,
                expression: Expression::Identifier(Identifier {
                    token: String::from("second"),
                    enclosing_type: None,
                    line_info: LineInfo { line : 1, offset : 11}
                })
            }],
            mangled_identifier: None
        });
    }
}
