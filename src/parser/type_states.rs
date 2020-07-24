use crate::ast::{Identifier, LineInfo, TypeState};
use crate::parser::operators::{left_parens, right_parens};
use crate::parser::utils::*;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::error::ErrorKind;
use nom::multi::many0;
use nom::sequence::preceded;
use std::collections::HashSet;

pub fn parse_type_states(i: Span) -> nom::IResult<Span, Vec<TypeState>> {
    let (i, _) = left_parens(i)?;
    let (i, type_states) =
        nom::multi::separated_list(tag(","), preceded(whitespace, parse_type_state))(i)?;
    let (i, _) = right_parens(i)?;

    // Ensure no repeats
    if type_states.len()
        != type_states
            .iter()
            .map(|id| id.identifier.token.as_str())
            .collect::<HashSet<&str>>()
            .len()
    {
        return Err(nom::Err::Failure((i, ErrorKind::SeparatedList)));
    }
    Ok((i, type_states))
}

pub fn parse_type_state(i: Span) -> nom::IResult<Span, TypeState> {
    let line_info = LineInfo {
        line: i.location_line(),
        offset: i.location_offset(),
    };
    let (remains, head) = nom::character::complete::alpha1(i)?;
    // Must start with upper case
    if !head
        .to_string()
        .chars()
        .next()
        .unwrap()
        .is_ascii_uppercase()
    {
        return Err(nom::Err::Failure((i, ErrorKind::Char)));
    }

    let (i, tail) = nom::combinator::recognize(many0(alt((
        nom::character::complete::alphanumeric1,
        tag("_"),
    ))))(remains)?;
    let head = head.to_string();
    let token = head + tail.fragment();
    let state = TypeState {
        identifier: Identifier {
            token,
            enclosing_type: None,
            line_info,
        },
    };
    Ok((i, state))
}

#[cfg(test)]
mod test {
    use super::parse_type_state;
    use super::parse_type_states;
    use crate::ast::{Identifier, LineInfo, TypeState};
    use nom_locate::LocatedSpan;

    fn create_states(state_names: Vec<&str>, line_info: Vec<(u32, usize)>) -> Vec<TypeState> {
        let line_info = line_info
            .into_iter()
            .map(|(line, offset)| LineInfo { line, offset })
            .collect::<Vec<LineInfo>>();

        state_names
            .into_iter()
            .zip(line_info.into_iter())
            .map(|(token, line_info)| TypeState {
                identifier: Identifier {
                    token: token.to_string(),
                    enclosing_type: None,
                    line_info,
                },
            })
            .collect::<Vec<TypeState>>()
    }

    #[test]
    fn test_parse_type_states() {
        let input = LocatedSpan::new("(S1, S2, S3) {");
        let (remains, states) =
            parse_type_states(input).unwrap_or_else(|_| panic!("Failure parsing type states"));
        assert_eq!(*remains.fragment(), " {");
        assert_eq!(
            states,
            create_states(vec!["S1", "S2", "S3"], vec![(1, 1), (1, 5), (1, 9)])
        );

        let input = LocatedSpan::new("(s1, s2, s3) {");
        parse_type_states(input).expect_err("Should not allow lowercase starting states");

        let input = LocatedSpan::new("(S1, S2, S1, S3)");
        parse_type_states(input).expect_err("Should not allow duplicate typestates");
    }

    #[test]
    fn test_parse_type_state() {
        let input = LocatedSpan::new("S1");
        let (remains, state) =
            parse_type_state(input).unwrap_or_else(|_| panic!("Could not parse type state"));
        assert!(remains.fragment().is_empty());
        assert_eq!(vec![state], create_states(vec!["S1"], vec![(1, 0)]));

        let input = LocatedSpan::new("s1");
        parse_type_state(input).expect_err("Should not allow non upper case starting state");

        let input = LocatedSpan::new("_1");
        parse_type_state(input).expect_err("Should not allow non alpha starting state");
    }
}
