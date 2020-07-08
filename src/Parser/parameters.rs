use crate::Parser::types::*;
use crate::Parser::utils::*;

pub fn parse_parameter_list(i: Span) -> nom::IResult<Span, Vec<Parameter>> {
    let (i, _) = left_parens(i)?;
    let (i, vector) =
        nom::multi::separated_list(tag(","), preceded(whitespace, parse_parameter))(i)?;
    let (i, _) = right_parens(i)?;
    Ok((i, vector))
}

fn parse_parameter(i: Span) -> nom::IResult<Span, Parameter> {
    let line_info = LineInfo {
        line: i.location_line(),
        offset: i.location_offset(),
    };
    let (i, identifier) = parse_identifier(i)?;
    let (i, type_assigned) = parse_type_annotation(i)?;
    let (i, equal) = nom::combinator::opt(preceded(whitespace, equal_operator))(i)?;
    if equal.is_none() {
        let parameter = Parameter {
            identifier,
            type_assignment: type_assigned.type_assigned,
            expression: None,
            line_info,
        };
        return Ok((i, parameter));
    }
    let (i, expression) = preceded(whitespace, parse_expression)(i)?;
    let parameter = Parameter {
        identifier,
        type_assignment: type_assigned.type_assigned,
        expression: Some(expression),
        line_info,
    };
    Ok((i, parameter))
}

#[cfg(test)]
mod test {

    use crate::Parser::parameters::*;

    #[test]
    fn test_parse_parameter() {
        let input = LocatedSpan::new("first: Int");
        let (_rest, result) = parse_parameter(input).expect("Error parsing parameter");
        assert_eq!(
            result,
            Parameter {
                identifier: Identifier {
                    token: String::from("first"),
                    enclosing_type: None,
                    line_info: LineInfo { line: 1, offset: 0 },
                },

                type_assignment: Type::Int,
                expression: None,
                line_info: LineInfo { line: 1, offset: 0 },
            }
        );

        let input = LocatedSpan::new("first: Int = second");
        let (_rest, result) =
            parse_parameter(input).expect("Error parsing parameter with expression");
        assert_eq!(
            result,
            Parameter {
                identifier: Identifier {
                    token: String::from("first"),
                    enclosing_type: None,
                    line_info: LineInfo { line: 1, offset: 0 },
                },

                type_assignment: Type::Int,
                expression: Some(Expression::Identifier(Identifier {
                    token: String::from("second"),
                    enclosing_type: None,
                    line_info: LineInfo { line: 1, offset: 0 },
                })),

                line_info: LineInfo { line: 1, offset: 0 },
            }
        );
    }
}
