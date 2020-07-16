use crate::parser::calls::parse_function_call;
use crate::parser::expressions::*;
use crate::parser::identifiers::*;
use crate::parser::modifiers::*;
use crate::parser::operators::*;
use crate::parser::parameters::*;
use crate::parser::types::*;
use crate::parser::utils::*;
use nom::error::ErrorKind;
use nom::lib::std::collections::HashSet;

pub fn parse_top_level_declaration(i: Span) -> nom::IResult<Span, TopLevelDeclaration> {
    let (i, top) = alt((
        parse_contract_declaration,
        map(parse_contract_behaviour_declaration, |c| {
            TopLevelDeclaration::ContractBehaviourDeclaration(c)
        }),
        parse_struct_declaration,
        parse_asset_declaration,
        parse_enum_declaration,
        parse_trait_declaration,
    ))(i)?;
    Ok((i, top))
}

// event declaration

fn parse_event_declaration(i: Span) -> nom::IResult<Span, EventDeclaration> {
    let (i, _event_token) = tag("event")(i)?;
    let (i, _) = whitespace(i)?;
    let (i, identifier) = parse_identifier(i)?;
    let (i, _) = whitespace(i)?;
    let (i, parameter_list) = parse_parameter_list(i)?;
    let event_declaration = EventDeclaration {
        identifier,
        parameter_list,
    };
    Ok((i, event_declaration))
}

// contract declaration

fn parse_contract_declaration(i: Span) -> nom::IResult<Span, TopLevelDeclaration> {
    let (i, _contract_token) = tag("contract")(i)?;
    let (i, identifier) = preceded(nom::character::complete::space0, parse_identifier)(i)?;
    let (i, _) = whitespace(i)?;
    // TODO fix enforcing of type states being correct
    // TODO add type states contract parse test including a failing case for mismatched
    let (_, left_round_bracket) = nom::combinator::opt(left_parens)(i)?;
    let (i, type_states) = if left_round_bracket.is_none() {
        (i, vec![])
    } else {
        parse_type_states(i)?
    };

    let (i, _) = whitespace(i)?;
    let (i, conformances) = parse_conformances(i)?;
    let (i, _identifier_group) = nom::combinator::opt(parse_identifier_group)(i)?;
    let (i, _) = preceded(nom::character::complete::space0, left_brace)(i)?;
    let (i, contract_members) = many0(nom::sequence::terminated(
        preceded(whitespace, parse_contract_member),
        multi_whitespace,
    ))(i)?;
    let (i, _) = whitespace(i)?;
    let (i, _) = right_brace(i)?;
    let contract = ContractDeclaration {
        identifier,
        contract_members,
        type_states,
        conformances,
    };
    Ok((i, TopLevelDeclaration::ContractDeclaration(contract)))
}

fn parse_contract_member(i: Span) -> nom::IResult<Span, ContractMember> {
    alt((
        map(parse_event_declaration, |e| {
            ContractMember::EventDeclaration(e)
        }),
        map(parse_variable_declaration_enclosing, |v| {
            ContractMember::VariableDeclaration(v)
        }),
    ))(i)
}

fn parse_conformances(i: Span) -> nom::IResult<Span, Vec<Conformance>> {
    let (i, colon_token) = nom::combinator::opt(colon)(i)?;
    if colon_token.is_none() {
        return Ok((i, Vec::new()));
    }
    let (i, _) = whitespace(i)?;
    let (i, identifier_list) = parse_identifier_list(i)?;
    let conformances = identifier_list
        .into_iter()
        .map(|identifier| Conformance { identifier })
        .collect();
    Ok((i, conformances))
}

// contract behaviour declaration

fn parse_contract_behaviour_declaration(
    i: Span,
) -> nom::IResult<Span, ContractBehaviourDeclaration> {
    let (i, identifier) = parse_identifier(i)?;
    let (i, _) = whitespace(i)?;
    let (i, at_token) = nom::combinator::opt(at)(i)?;
    let (i, type_states) = if at_token.is_none() {
        (
            i,
            vec![TypeState {
                identifier: Identifier {
                    token: "any".to_string(),
                    enclosing_type: None,
                    line_info: Default::default(),
                },
            }],
        )
    } else {
        parse_type_states(i)?
    };
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, _) = double_colon(i)?;
    let (i, _) = whitespace(i)?;
    let (i, caller_binding) = nom::combinator::opt(parse_caller_binding)(i)?;
    let (i, _) = whitespace(i)?;
    let (i, caller_protections) = parse_caller_protection_group(i)?;
    let (i, _) = whitespace(i)?;
    let (i, _) = left_brace(i)?;
    let (i, members) = many0(nom::sequence::terminated(
        preceded(whitespace, parse_contract_behaviour_member),
        multi_whitespace,
    ))(i)?;
    let (i, _) = right_brace(i)?;
    let contract_behaviour_declaration = ContractBehaviourDeclaration {
        members,
        identifier,
        type_states,
        caller_protections,
        caller_binding,
    };
    Ok((i, contract_behaviour_declaration))
}

fn parse_contract_behaviour_member(i: Span) -> nom::IResult<Span, ContractBehaviourMember> {
    alt((
        map(parse_function_declaration, |f| {
            ContractBehaviourMember::FunctionDeclaration(f)
        }),
        map(parse_special_declaration, |s| {
            ContractBehaviourMember::SpecialDeclaration(s)
        }),
        map(parse_special_signature_declaration, |s| {
            ContractBehaviourMember::SpecialSignatureDeclaration(s)
        }),
        map(parse_function_signature_declaration, |f| {
            ContractBehaviourMember::FunctionSignatureDeclaration(f)
        }),
    ))(i)
}

fn parse_caller_binding(i: Span) -> nom::IResult<Span, Identifier> {
    let (i, identifier) = parse_identifier(i)?;
    let (i, _) = whitespace(i)?;
    let (i, _) = left_arrow(i)?;
    Ok((i, identifier))
}

fn parse_type_states(i: Span) -> nom::IResult<Span, Vec<TypeState>> {
    let (remaining, identifier_group) = parse_identifier_group(i)?;
    // Ensure all start with an upper case ascii letter
    if !identifier_group.iter().all(|id| {
        let first = id.token.chars().next().unwrap();
        first.is_alphabetic() && first.is_ascii_uppercase()
    }) {
        return Err(nom::Err::Failure((i, ErrorKind::Char)));
    }

    // Ensure no repeats
    if identifier_group.len()
        != identifier_group
            .iter()
            .map(|id| id.token.as_str())
            .collect::<HashSet<&str>>()
            .len()
    {
        return Err(nom::Err::Failure((i, ErrorKind::SeparatedList)));
    }

    let types_states = identifier_group
        .into_iter()
        .map(|identifier| TypeState { identifier })
        .collect();
    Ok((remaining, types_states))
}

#[allow(dead_code)]
fn parse_protection_binding(i: Span) -> nom::IResult<Span, Identifier> {
    let (i, identifier) = parse_identifier(i)?;
    let (i, _) = left_arrow(i)?;
    Ok((i, identifier))
}

fn parse_caller_protection_group(i: Span) -> nom::IResult<Span, Vec<CallerProtection>> {
    let (i, identifiers) = parse_identifier_group(i)?;
    let caller_protections = identifiers
        .into_iter()
        .map(|identifier| CallerProtection { identifier })
        .collect();
    Ok((i, caller_protections))
}

// variable declaration

fn parse_variable_declaration_enclosing(i: Span) -> nom::IResult<Span, VariableDeclaration> {
    let (i, _) = parse_modifiers(i)?;
    let (i, _) = whitespace(i)?;
    let (i, declaration_token) = alt((tag("var"), tag("let")))(i)?;
    let declaration_token = Some(declaration_token.fragment().to_string());
    let (i, identifier) = preceded(nom::character::complete::space0, parse_identifier)(i)?;
    let (i, type_annotation) = parse_type_annotation(i)?;
    let (i, _) = whitespace(i)?;
    let (i, equal_token) = nom::combinator::opt(equal_operator)(i)?;
    if equal_token.is_none() {
        let variable_declaration = VariableDeclaration {
            declaration_token,
            identifier,
            variable_type: type_annotation.type_assigned,
            expression: None,
        };
        return Ok((i, variable_declaration));
    }
    let (i, expression) = preceded(nom::character::complete::space0, parse_expression)(i)?;
    let variable_declaration = VariableDeclaration {
        declaration_token,
        identifier,
        variable_type: type_annotation.type_assigned,
        expression: Option::from(Box::new(expression)),
    };
    Ok((i, variable_declaration))
}

pub fn parse_variable_declaration(i: Span) -> nom::IResult<Span, VariableDeclaration> {
    let (i, _) = parse_modifiers(i)?;
    let (i, _) = whitespace(i)?;
    let (i, declaration_token) = alt((tag("var"), tag("let")))(i)?;
    let declaration_token = Some(declaration_token.fragment().to_string());
    let (i, identifier) = preceded(nom::character::complete::space0, parse_identifier)(i)?;
    let (i, type_annotation) = parse_type_annotation(i)?;
    let (i, _) = whitespace(i)?;
    let variable_declaration = VariableDeclaration {
        declaration_token,
        identifier,
        variable_type: type_annotation.type_assigned,
        expression: None,
    };
    Ok((i, variable_declaration))
}

// enum declaration

fn parse_enum_declaration(i: Span) -> nom::IResult<Span, TopLevelDeclaration> {
    let (i, enum_token) = tag("enum")(i)?;
    let (i, identifier) = preceded(nom::character::complete::space0, parse_identifier)(i)?;
    let (i, type_annotation) = nom::combinator::opt(parse_type_annotation)(i)?;
    let type_assigned = if type_annotation.is_none() {
        None
    } else {
        Some(type_annotation.unwrap().type_assigned)
    };
    let (i, _) = preceded(nom::character::complete::space0, left_brace)(i)?;
    let (i, _) = whitespace(i)?;

    let (i, members) = nom::multi::separated_list(whitespace, parse_enum_member)(i)?;
    let mut enum_members = Vec::<EnumMember>::new();
    for member in members {
        let enum_member = EnumMember {
            case_token: member.case_token,
            identifier: member.identifier,
            hidden_value: member.hidden_value,
            enum_type: Type::UserDefinedType(identifier.clone()),
        };
        enum_members.push(enum_member);
    }
    let members = enum_members;
    let (i, _) = whitespace(i)?;
    let (i, _) = right_brace(i)?;
    let enum_declaration = EnumDeclaration {
        enum_token: enum_token.to_string(),
        identifier,
        type_assigned,
        members,
    };
    Ok((i, TopLevelDeclaration::EnumDeclaration(enum_declaration)))
}

fn parse_enum_member(i: Span) -> nom::IResult<Span, EnumMember> {
    let (i, case_token) = tag("case")(i)?;
    let (i, identifier) = preceded(nom::character::complete::space0, parse_identifier)(i)?;
    let (i, equal_token) = nom::combinator::opt(preceded(whitespace, equal_operator))(i)?;
    let enum_type = Type::UserDefinedType(Identifier {
        ..Default::default()
    });
    if equal_token.is_none() {
        let enum_member = EnumMember {
            case_token: case_token.to_string(),
            identifier,
            hidden_value: None,
            enum_type,
        };
        return Ok((i, enum_member));
    }
    let (i, expression) = parse_expression(i)?;
    let enum_member = EnumMember {
        case_token: case_token.to_string(),
        identifier,
        hidden_value: Some(expression),
        enum_type,
    };
    Ok((i, enum_member))
}

// special declaration

fn parse_special_declaration(i: Span) -> nom::IResult<Span, SpecialDeclaration> {
    let (i, signature) = parse_special_signature_declaration(i)?;
    let (i, _) = whitespace(i)?;
    let (i, statements) = parse_code_block(i)?;
    let special_declaration = SpecialDeclaration {
        head: signature,
        body: statements,
        scope_context: Default::default(),
        generated: false,
    };

    Ok((i, special_declaration))
}

fn parse_special_signature_declaration(i: Span) -> nom::IResult<Span, SpecialSignatureDeclaration> {
    let (i, attributes) = parse_attributes(i)?;
    let (i, modifiers) = parse_modifiers(i)?;
    let (i, special_token) = alt((tag("init"), tag("fallback")))(i)?;
    let (i, parameters) = parse_parameter_list(i)?;
    let (i, _) = whitespace(i)?;
    let (i, mutates) = parse_mutates(i)?;
    let special_signature_declaration = SpecialSignatureDeclaration {
        attributes,
        modifiers,
        mutates,
        parameters,
        special_token: special_token.to_string(),
    };
    Ok((i, special_signature_declaration))
}

fn parse_attributes(i: Span) -> nom::IResult<Span, Vec<Attribute>> {
    many0(nom::sequence::terminated(parse_attribute, whitespace))(i)
}

fn parse_attribute(i: Span) -> nom::IResult<Span, Attribute> {
    let (i, at) = at(i)?;
    let (i, identifier) = parse_identifier(i)?;
    let attribute = Attribute {
        at_token: at.to_string(),
        identifier_token: identifier.token,
    };
    Ok((i, attribute))
}

fn parse_mutates(i: Span) -> nom::IResult<Span, Vec<Identifier>> {
    let identifiers = Vec::new();
    let (i, mutates) = nom::combinator::opt(tag("mutates"))(i)?;
    if mutates.is_none() {
        return Ok((i, identifiers));
    }
    let (i, _) = whitespace(i)?;
    let (i, _) = left_parens(i)?;
    let (i, identifiers) = nom::multi::separated_nonempty_list(
        tag(","),
        nom::sequence::terminated(
            preceded(
                whitespace,
                alt((parse_enclosing_identifier, parse_identifier)),
            ),
            whitespace,
        ),
    )(i)?;
    let (i, _) = right_parens(i)?;
    Ok((i, identifiers))
}

// function declaration

fn parse_function_declaration(i: Span) -> nom::IResult<Span, FunctionDeclaration> {
    let (i, signature) = parse_function_signature_declaration(i)?;
    let (i, _) = whitespace(i)?;
    let (i, statements) = parse_code_block(i)?;

    let function_declaration = FunctionDeclaration {
        head: signature,
        body: statements,
        scope_context: None,
        tags: vec![],
        mangled_identifier: None,
        is_external: false,
    };
    Ok((i, function_declaration))
}

fn parse_function_signature_declaration(
    i: Span,
) -> nom::IResult<Span, FunctionSignatureDeclaration> {
    let (i, attributes) = parse_attributes(i)?;
    let mut payable = false;
    for attribute in &attributes {
        if attribute.identifier_token == "payable".to_string() {
            payable = true;
        }
    }
    let (i, _) = whitespace(i)?;
    let (i, modifiers) = parse_modifiers(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, func_token) = tag("func")(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, identifier) = parse_identifier(i)?;
    let (i, parameters) = parse_parameter_list(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, result_type) = parse_result(i)?;
    let (i, _) = whitespace(i)?;
    let (i, mutates) = parse_mutates(i)?;
    let function_signature_declaration = FunctionSignatureDeclaration {
        func_token: func_token.to_string(),
        attributes,
        modifiers,
        identifier,
        mutates,
        parameters,
        result_type,
        payable,
    };
    Ok((i, function_signature_declaration))
}

fn parse_result(i: Span) -> nom::IResult<Span, Option<Type>> {
    let (i, token) = nom::combinator::opt(right_arrow)(i)?;
    if token.is_none() {
        return Ok((i, None));
    }
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, identifier) = parse_identifier_type(i)?;
    Ok((i, Some(identifier)))
}

// asset declaration

fn parse_asset_declaration(i: Span) -> nom::IResult<Span, TopLevelDeclaration> {
    let (i, _struct_token) = tag("asset")(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, identifier) = parse_identifier(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, _) = left_brace(i)?;
    let (i, members) = many0(nom::sequence::terminated(
        preceded(whitespace, parse_asset_member),
        nom::character::complete::multispace0,
    ))(i)?;
    let (i, _) = whitespace(i)?;
    let (i, _) = right_brace(i)?;
    let asset_declaration = AssetDeclaration {
        identifier,
        members,
    };
    Ok((i, TopLevelDeclaration::AssetDeclaration(asset_declaration)))
}

fn parse_asset_member(i: Span) -> nom::IResult<Span, AssetMember> {
    alt((
        map(parse_function_declaration, |f| {
            AssetMember::FunctionDeclaration(f)
        }),
        map(parse_special_declaration, |s| {
            AssetMember::SpecialDeclaration(s)
        }),
        map(parse_variable_declaration_enclosing, |v| {
            AssetMember::VariableDeclaration(v)
        }),
    ))(i)
}

// struct declaration

fn parse_struct_declaration(i: Span) -> nom::IResult<Span, TopLevelDeclaration> {
    let (i, _struct_token) = tag("struct")(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, identifier) = parse_identifier(i)?;
    let (i, conformances) = parse_conformances(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, _) = left_brace(i)?;
    let (i, members) = many0(nom::sequence::terminated(
        preceded(whitespace, parse_struct_member),
        nom::character::complete::multispace0,
    ))(i)?;
    let (i, _) = whitespace(i)?;
    let (i, _) = right_brace(i)?;
    let struct_declaration = StructDeclaration {
        identifier,
        conformances,
        members,
    };
    Ok((
        i,
        TopLevelDeclaration::StructDeclaration(struct_declaration),
    ))
}

fn parse_struct_member(i: Span) -> nom::IResult<Span, StructMember> {
    alt((
        map(parse_function_declaration, |f| {
            StructMember::FunctionDeclaration(f)
        }),
        map(parse_special_declaration, |s| {
            StructMember::SpecialDeclaration(s)
        }),
        map(parse_variable_declaration_enclosing, |v| {
            StructMember::VariableDeclaration(v)
        }),
    ))(i)
}

// trait declaration

fn parse_trait_declaration(i: Span) -> nom::IResult<Span, TopLevelDeclaration> {
    let (i, modifiers) = many0(nom::sequence::terminated(
        preceded(whitespace, parse_trait_modifier),
        whitespace,
    ))(i)?;
    let (i, external) = nom::combinator::opt(tag("external"))(i)?;
    let (i, _) = nom::combinator::opt(nom::character::complete::space0)(i)?;
    let (i, _) = tag("trait")(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, identifier) = parse_identifier(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, _) = left_brace(i)?;
    let (i, members) = many0(nom::sequence::terminated(
        preceded(whitespace, parse_trait_member),
        nom::character::complete::multispace0,
    ))(i)?;
    let (i, _) = right_brace(i)?;
    let trait_declaration = TraitDeclaration {
        external: external.is_some(),
        identifier,
        members,
        modifiers,
    };
    Ok((i, TopLevelDeclaration::TraitDeclaration(trait_declaration)))
}

fn parse_trait_modifier(i: Span) -> nom::IResult<Span, FunctionCall> {
    let (i, _) = tag("@")(i)?;
    let (i, fc) = nom::combinator::opt(parse_function_call)(i)?;
    if let Some(fc) = fc {
        return Ok((i, fc));
    }
    let (i, identifier) = parse_identifier(i)?;
    let fc = FunctionCall {
        identifier,
        arguments: vec![],
        mangled_identifier: None,
    };

    Ok((i, fc))
}

fn parse_trait_member(i: Span) -> nom::IResult<Span, TraitMember> {
    alt((
        map(parse_function_declaration, |f| {
            TraitMember::FunctionDeclaration(f)
        }),
        map(parse_special_declaration, |s| {
            TraitMember::SpecialDeclaration(s)
        }),
        map(parse_function_signature_declaration, |f| {
            TraitMember::FunctionSignatureDeclaration(f)
        }),
        map(parse_special_signature_declaration, |s| {
            TraitMember::SpecialSignatureDeclaration(s)
        }),
        map(parse_event_declaration, |e| {
            TraitMember::EventDeclaration(e)
        }),
        map(parse_contract_behaviour_declaration, |c| {
            TraitMember::ContractBehaviourDeclaration(c)
        }),
    ))(i)
}

#[cfg(test)]
mod test {
    use crate::ast::*;
    use crate::parser::declarations::*;
    use nom_locate::LocatedSpan;

    #[test]
    fn test_parse_contract_member() {
        let input = LocatedSpan::new("var minter: Address");
        let (_rest, result) = parse_contract_member(input).expect("Error parsing contract member");
        assert_eq!(
            result,
            ContractMember::VariableDeclaration(VariableDeclaration {
                declaration_token: Some(String::from("var")),

                identifier: Identifier {
                    token: String::from("minter"),
                    enclosing_type: None,
                    line_info: LineInfo { line: 1, offset: 0 },
                },

                variable_type: Type::Address,
                expression: None,
            })
        );
    }

    #[test]
    fn test_parse_caller_binding() {
        let input = "caller <-";
        let input = LocatedSpan::new(input);
        let result = parse_caller_binding(input);
        match result {
            Ok((_c, b)) => assert_eq!(b, Identifier::generated("caller")),
            Err(_) => assert_eq!(1, 0),
        }
    }

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
    fn test_parse_type_state() {
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
}
