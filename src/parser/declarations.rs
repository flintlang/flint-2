use crate::ast::{
    AssetDeclaration, AssetMember, Attribute, CallerProtection, Conformance,
    ContractBehaviourDeclaration, ContractBehaviourMember, ContractDeclaration, ContractMember,
    EnumDeclaration, EnumMember, EventDeclaration, FunctionCall, FunctionDeclaration,
    FunctionSignatureDeclaration, Identifier, Modifier, SpecialDeclaration,
    SpecialSignatureDeclaration, StructDeclaration, StructMember, TopLevelDeclaration,
    TraitDeclaration, TraitMember, Type, VariableDeclaration,
};
use crate::parser::calls::parse_function_call;
use crate::parser::expressions::*;
use crate::parser::identifiers::*;
use crate::parser::modifiers::*;
use crate::parser::operators::*;
use crate::parser::parameters::*;
use crate::parser::type_states::*;
use crate::parser::types::*;
use crate::parser::utils::*;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::map;
use nom::multi::many0;
use nom::sequence::preceded;

pub fn parse_top_level_declaration(i: Span) -> nom::IResult<Span, TopLevelDeclaration> {
    let (i, top) = alt((
        parse_contract_declaration,
        map(
            parse_contract_behaviour_declaration,
            TopLevelDeclaration::ContractBehaviourDeclaration,
        ),
        parse_struct_declaration,
        parse_asset_declaration,
        parse_enum_declaration,
        parse_trait_declaration,
    ))(i)?;
    Ok((i, top))
}

// Event declaration

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
    let (i, _) = tag("contract")(i)?;
    let (i, identifier) = preceded(nom::character::complete::space0, parse_identifier)(i)?;
    let (i, _) = whitespace(i)?;
    let (_, left_round_bracket) = nom::combinator::opt(left_parens)(i)?;
    let (i, type_states) = if left_round_bracket.is_none() {
        (i, vec![])
    } else {
        parse_type_states(i)?
    };

    let (i, _) = whitespace(i)?;
    let (i, conformances) = parse_conformances(i)?;
    let (i, _) = nom::combinator::opt(parse_identifier_group)(i)?;
    let (i, _) = preceded(nom::character::complete::space0, left_brace)(i)?;
    let (i, mut contract_members) = many0(nom::sequence::terminated(
        preceded(whitespace, parse_contract_member),
        multi_whitespace,
    ))(i)?;

    // Add enclosing type to all contract members
    for member in contract_members.iter_mut() {
        if let ContractMember::VariableDeclaration(dec, _) = member {
            dec.identifier.enclosing_type = Some(identifier.token.clone());
        }
    }

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
        map(parse_event_declaration, ContractMember::EventDeclaration),
        map(parse_variable_declaration_enclosing, |(dec, modifier)| {
            ContractMember::VariableDeclaration(dec, modifier)
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
        (i, vec![])
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
    let (i, mut members) = many0(nom::sequence::terminated(
        preceded(whitespace, parse_contract_behaviour_member),
        multi_whitespace,
    ))(i)?;

    // Give members the enclosing type
    for member in members.iter_mut() {
        match member {
            ContractBehaviourMember::FunctionDeclaration(declaration) => {
                declaration.head.identifier.enclosing_type = Some(identifier.token.clone())
            }
            ContractBehaviourMember::SpecialDeclaration(declaration) => {
                declaration.head.enclosing_type = Some(identifier.token.clone())
            }
            ContractBehaviourMember::FunctionSignatureDeclaration(declarations) => {
                declarations.identifier.enclosing_type = Some(identifier.token.clone())
            }
            ContractBehaviourMember::SpecialSignatureDeclaration(declarations) => {
                declarations.enclosing_type = Some(identifier.token.clone())
            }
        }
    }

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
        map(
            parse_function_declaration,
            ContractBehaviourMember::FunctionDeclaration,
        ),
        map(
            parse_special_declaration,
            ContractBehaviourMember::SpecialDeclaration,
        ),
        map(
            parse_special_signature_declaration,
            ContractBehaviourMember::SpecialSignatureDeclaration,
        ),
        map(
            parse_function_signature_declaration,
            ContractBehaviourMember::FunctionSignatureDeclaration,
        ),
    ))(i)
}

fn parse_caller_binding(i: Span) -> nom::IResult<Span, Identifier> {
    let (i, identifier) = parse_identifier(i)?;
    let (i, _) = whitespace(i)?;
    let (i, _) = left_arrow(i)?;
    Ok((i, identifier))
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

fn parse_variable_declaration_enclosing(
    i: Span,
) -> nom::IResult<Span, (VariableDeclaration, Option<Modifier>)> {
    let (i, modifier) = nom::combinator::opt(parse_modifier)(i)?;
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
        return Ok((i, (variable_declaration, modifier)));
    }
    let (i, expression) = preceded(nom::character::complete::space0, parse_expression)(i)?;
    let variable_declaration = VariableDeclaration {
        declaration_token,
        identifier,
        variable_type: type_annotation.type_assigned,
        expression: Option::from(Box::new(expression)),
    };
    Ok((i, (variable_declaration, modifier)))
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
    let type_assigned = type_annotation.map(|t_a| t_a.type_assigned);
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
    let (i, expression) = preceded(nom::character::complete::space0, parse_expression)(i)?;
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
        enclosing_type: None,
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
    let payable = attributes.iter().any(|a| a.identifier_token == "payable");
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
        map(parse_function_declaration, AssetMember::FunctionDeclaration),
        map(parse_special_declaration, AssetMember::SpecialDeclaration),
        map(parse_variable_declaration_enclosing, |(dec, _)| {
            AssetMember::VariableDeclaration(dec)
        }),
    ))(i)
}

// struct declaration

fn parse_struct_declaration(i: Span) -> nom::IResult<Span, TopLevelDeclaration> {
    let (i, _) = tag("struct")(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, identifier) = parse_identifier(i)?;
    let (i, conformances) = parse_conformances(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, _) = left_brace(i)?;
    let (i, mut members) = many0(nom::sequence::terminated(
        preceded(whitespace, parse_struct_member),
        nom::character::complete::multispace0,
    ))(i)?;

    // Add enclosing type to all struct members
    for member in members.iter_mut() {
        match member {
            StructMember::VariableDeclaration(dec, _) => {
                dec.identifier.enclosing_type = Some(identifier.token.clone())
            }
            StructMember::FunctionDeclaration(dec) => {
                dec.head.identifier.enclosing_type = Some(identifier.token.clone())
            }
            StructMember::SpecialDeclaration(dec) => {
                dec.head.enclosing_type = Some(identifier.token.clone())
            }
        }
    }

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
        map(
            parse_function_declaration,
            StructMember::FunctionDeclaration,
        ),
        map(parse_special_declaration, StructMember::SpecialDeclaration),
        map(parse_variable_declaration_enclosing, |(dec, modifier)| {
            StructMember::VariableDeclaration(dec, modifier)
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
        map(parse_function_declaration, TraitMember::FunctionDeclaration),
        map(parse_special_declaration, TraitMember::SpecialDeclaration),
        map(
            parse_function_signature_declaration,
            TraitMember::FunctionSignatureDeclaration,
        ),
        map(
            parse_special_signature_declaration,
            TraitMember::SpecialSignatureDeclaration,
        ),
        map(parse_event_declaration, TraitMember::EventDeclaration),
        map(
            parse_contract_behaviour_declaration,
            TraitMember::ContractBehaviourDeclaration,
        ),
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
            ContractMember::VariableDeclaration(
                VariableDeclaration {
                    declaration_token: Some(String::from("var")),

                    identifier: Identifier {
                        token: String::from("minter"),
                        enclosing_type: None,
                        line_info: LineInfo { line: 1, offset: 0 },
                    },

                    variable_type: Type::Address,
                    expression: None,
                },
                None,
            )
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
}
