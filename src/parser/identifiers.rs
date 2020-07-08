use crate::parser::utils::*;

pub fn parse_enclosing_identifier(i: Span) -> nom::IResult<Span, Identifier> {
    let line_info = LineInfo {
        line: i.location_line(),
        offset: i.location_offset(),
    };
    let (i, enclosing_type) = parse_identifier(i)?;
    let (i, _) = dot_operator(i)?;
    let (i, identifier) = parse_identifier(i)?;
    let identifier = Identifier {
        token: identifier.token,
        enclosing_type: Some(enclosing_type.token),
        line_info,
    };
    Ok((i, identifier))
}

pub fn parse_identifier(i: Span) -> nom::IResult<Span, Identifier> {
    let line_info = LineInfo {
        line: i.location_line(),
        offset: i.location_offset(),
    };
    let (i, head) = alt((nom::character::complete::alpha1, tag("_")))(i)?;
    let (i, tail) = nom::combinator::recognize(many0(alt((
        nom::character::complete::alphanumeric1,
        tag("_"),
        tag("$"),
    ))))(i)?;
    let head = head.to_string();
    let token = head + tail.fragment();
    let identifier = Identifier {
        token,
        enclosing_type: None,
        line_info,
    };
    Ok((i, identifier))
}

pub fn parse_identifier_list(i: Span) -> nom::IResult<Span, Vec<Identifier>> {
    nom::multi::separated_list(tag(","), preceded(whitespace, parse_identifier))(i)
}

pub fn parse_identifier_group(i: Span) -> nom::IResult<Span, Vec<Identifier>> {
    let (i, _) = left_parens(i)?;
    let (i, identifier_list) = parse_identifier_list(i)?;
    let (i, _) = right_parens(i)?;
    Ok((i, identifier_list))
}

#[cfg(test)]
mod test {

    use crate::parser::identifiers::*;

    #[test]
    fn test_parse_identifier() {
        let input = LocatedSpan::new("id");
        let (_rest, result) = parse_identifier(input).expect("Error with parsing identifier");
        assert_eq!(
            result,
            Identifier {
                token: String::from("id"),
                enclosing_type: None,
                line_info: LineInfo { line: 1, offset: 0 },
            }
        );
    }

    #[test]
    fn test_parse_identifier_list() {
        let input = LocatedSpan::new("first, second, third");
        let (_rest, result) =
            parse_identifier_list(input).expect("Error with parsing identifier list");
        assert_eq!(
            result,
            vec![
                Identifier {
                    token: String::from("first"),
                    enclosing_type: None,
                    line_info: LineInfo { line: 1, offset: 0 },
                },
                Identifier {
                    token: String::from("second"),
                    enclosing_type: None,
                    line_info: LineInfo { line: 1, offset: 0 },
                },
                Identifier {
                    token: String::from("third"),
                    enclosing_type: None,
                    line_info: LineInfo { line: 1, offset: 0 },
                }
            ]
        );
    }

    #[test]
    fn test_parse_identifier_group() {
        let input = LocatedSpan::new("(first, second, third)");
        let (_rest, result) =
            parse_identifier_group(input).expect("Error with parsing identifier group");
        assert_eq!(
            result,
            vec![
                Identifier {
                    token: String::from("first"),
                    enclosing_type: None,
                    line_info: LineInfo { line: 1, offset: 0 },
                },
                Identifier {
                    token: String::from("second"),
                    enclosing_type: None,
                    line_info: LineInfo { line: 1, offset: 0 },
                },
                Identifier {
                    token: String::from("third"),
                    enclosing_type: None,
                    line_info: LineInfo { line: 1, offset: 0 },
                }
            ]
        );
    }
}
