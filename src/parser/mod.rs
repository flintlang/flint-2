mod calls;
mod declarations;
mod expressions;
mod identifiers;
mod literals;
mod modifiers;
mod operators;
mod parameters;
mod statements;
mod types;
mod utils;

use nom_locate::LocatedSpan;

use nom::multi::many0;

use crate::parser::declarations::*;
use crate::parser::operators::*;
use crate::parser::utils::*;

pub fn parse_program(i: &str) -> ParseResult {
    // TODO fix error typing to be more helpful
    let input = LocatedSpan::new(i);
    let result = parse_module(input);

    match result {
        Ok((_, module)) => {
            // TODO check span is empty?
            let mut environment: Environment = Default::default();
            environment.build(module.clone());
            Ok((module, environment))
        }
        Err(nom::Err::Failure((i, _))) => {
            Err(format!("Could not parse {:#?}", i.fragment()))
        }
        Err(nom::Err::Error((i, _))) => {
            Err(format!("Could not parse {:#?}", i.fragment()))
        }
        _ => Err("Could not parse. Not enough data".to_string()),
    }

    // TODO remove
    // let module = match result {
    //     Ok((i, module)) => {
    //         if !i.fragment().is_empty() {
    //             panic!("Parser Error Parsing {:?}", i.fragment())
    //         };
    //         Some(module)
    //     }
    //     Err(_) => None,
    // };
}

fn parse_module(i: Span) -> nom::IResult<Span, Module> {
    let (i, _) = whitespace(i)?;
    let (i, declarations) = many0(nom::sequence::terminated(
        parse_top_level_declaration,
        whitespace,
    ))(i)?;
    Ok((i, Module { declarations }))
}

#[cfg(test)]
mod tests {
    use nom_locate::LocatedSpan;

    use crate::parser::declarations::*;
    use crate::parser::operators::*;
    use crate::parser::utils::*;

    #[test]
    fn test_parse_module() {
        let input = LocatedSpan::new(
            "contract Coin (Antique, Old, New, Invalid) {
                var minter: Address
                        var balance: [Address: Int] = [:]
                            event Sent(from: Address, to: Address, amount: Int)
        }",
        );

        let (_rest, result) = parse_module(input).expect("Error parsing module");

        assert_eq!(
            result,
            Module {
                declarations: vec![TopLevelDeclaration::ContractDeclaration(
                    ContractDeclaration {
                        identifier: Identifier {
                            token: String::from("Coin"),
                            enclosing_type: None,
                            line_info: LineInfo { line: 1, offset: 9 },
                        },

                        contract_members: vec![
                            ContractMember::VariableDeclaration(VariableDeclaration {
                                declaration_token: Some(String::from("var")),

                                identifier: Identifier {
                                    token: String::from("minter"),
                                    enclosing_type: None,
                                    line_info: LineInfo {
                                        line: 2,
                                        offset: 65,
                                    },
                                },

                                variable_type: Type::Address,
                                expression: None,
                            }),
                            ContractMember::VariableDeclaration(VariableDeclaration {
                                declaration_token: Some(String::from("var")),

                                identifier: Identifier {
                                    token: String::from("balance"),
                                    enclosing_type: None,
                                    line_info: LineInfo {
                                        line: 3,
                                        offset: 109,
                                    },
                                },

                                variable_type: Type::DictionaryType(DictionaryType {
                                    key_type: Box::new(Type::Address),
                                    value_type: Box::new(Type::Int),
                                }),

                                expression: Some(Box::new(Expression::DictionaryLiteral(
                                    DictionaryLiteral { elements: vec![] }
                                ))),
                            }),
                            ContractMember::EventDeclaration(EventDeclaration {
                                identifier: Identifier {
                                    token: String::from("Sent"),
                                    enclosing_type: None,
                                    line_info: LineInfo {
                                        line: 4,
                                        offset: 163,
                                    },
                                },

                                parameter_list: vec![
                                    Parameter {
                                        identifier: Identifier {
                                            token: String::from("from"),
                                            enclosing_type: None,
                                            line_info: LineInfo {
                                                line: 4,
                                                offset: 178,
                                            },
                                        },

                                        type_assignment: Type::Address,
                                        expression: None,
                                        line_info: LineInfo {
                                            line: 4,
                                            offset: 178,
                                        },
                                    },
                                    Parameter {
                                        identifier: Identifier {
                                            token: String::from("to"),
                                            enclosing_type: None,
                                            line_info: LineInfo {
                                                line: 4,
                                                offset: 193,
                                            },
                                        },
                                        type_assignment: Type::Address,
                                        expression: None,
                                        line_info: LineInfo {
                                            line: 4,
                                            offset: 193,
                                        },
                                    },
                                    Parameter {
                                        identifier: Identifier {
                                            token: String::from("amount"),
                                            enclosing_type: None,
                                            line_info: LineInfo {
                                                line: 4,
                                                offset: 206,
                                            },
                                        },
                                        type_assignment: Type::Int,
                                        expression: None,
                                        line_info: LineInfo {
                                            line: 4,
                                            offset: 206,
                                        },
                                    }
                                ],
                            })
                        ],
                        type_states: vec![
                            TypeState {
                                identifier: Identifier {
                                    token: "Antique".to_string(),
                                    enclosing_type: None,
                                    line_info: LineInfo {
                                        line: 1,
                                        offset: 15,
                                    }
                                }
                            },
                            TypeState {
                                identifier: Identifier {
                                    token: "Old".to_string(),
                                    enclosing_type: None,
                                    line_info: LineInfo {
                                        line: 1,
                                        offset: 24,
                                    }
                                }
                            },
                            TypeState {
                                identifier: Identifier {
                                    token: "New".to_string(),
                                    enclosing_type: None,
                                    line_info: LineInfo {
                                        line: 1,
                                        offset: 29,
                                    }
                                }
                            },
                            TypeState {
                                identifier: Identifier {
                                    token: "Invalid".to_string(),
                                    enclosing_type: None,
                                    line_info: LineInfo {
                                        line: 1,
                                        offset: 34,
                                    }
                                }
                            }
                        ],
                        conformances: vec![]
                    }
                )]
            }
        );
    }
}
