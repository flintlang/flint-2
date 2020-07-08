mod statements;
mod utils;
mod operators;
mod literals;
mod identifiers;
mod parameters;
mod types;
mod expressions;
mod declarations;
mod calls;
mod modifiers;

use nom_locate::LocatedSpan;

use std::collections::HashSet;

use nom::{branch::alt, bytes::complete::tag, combinator::map, multi::many0, sequence::preceded};

use crate::environment::Environment;
use crate::Parser::utils::*;
use crate::Parser::statements::*;
use crate::Parser::operators::*;
use crate::Parser::literals::*;
use crate::Parser::identifiers::*;
use crate::Parser::parameters::*;
use crate::Parser::types::*;
use crate::Parser::expressions::*;
use crate::Parser::declarations::*;
use crate::Parser::calls::*;
use crate::Parser::modifiers::*;

use super::AST::*;

pub fn parse_program(i: &str) -> ParseResult {
    let input = LocatedSpan::new(i);
    let result = parse_module(input);

    let module = match result {
        Ok((i, module)) => {
            if !i.fragment().is_empty() {
                panic!("Parser Error Parsing {:?}", i.fragment())
            };
            Some(module)
        }
        Err(_) => (None),
    };

    let mut environment = Environment {
        ..Default::default()
    };
    if module.is_some() {
        let module = module.unwrap();
        environment.build(module.clone());
        return (Option::from(module), environment);
    }
    (module, environment)
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
    use nom_locate::{LocatedSpan, position};
    use sha3::Digest;

    use crate::AST::{*, BinOp::*, Literal::*};
    use crate::Parser::*;
    use crate::Parser::statements;

    use nom::error::ErrorKind;

    #[test]
    fn test_parse_module() {
        let input = LocatedSpan::new(
            "contract Coin {
                var minter: Address
                        var balance: [Address: Int] = [:]
                            event Sent(from: Address, to: Address, amount: Int)
        }",
        );

        let (rest, result) = parse_module(input).expect("Error parsing module");

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
                                        offset: 36,
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
                                        offset: 80,
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
                                        offset: 144,
                                    },
                                },

                                parameter_list: vec![
                                    Parameter {
                                        identifier: Identifier {
                                            token: String::from("from"),
                                            enclosing_type: None,
                                            line_info: LineInfo {
                                                line: 4,
                                                offset: 149,
                                            },
                                        },

                                        type_assignment: Type::Address,
                                        expression: None,
                                        line_info: LineInfo {
                                            line: 4,
                                            offset: 149,
                                        },
                                    },
                                    Parameter {
                                        identifier: Identifier {
                                            token: String::from("to"),
                                            enclosing_type: None,
                                            line_info: LineInfo {
                                                line: 4,
                                                offset: 164,
                                            },
                                        },
                                        type_assignment: Type::Address,
                                        expression: None,
                                        line_info: LineInfo {
                                            line: 4,
                                            offset: 164,
                                        },
                                    },
                                    Parameter {
                                        identifier: Identifier {
                                            token: String::from("amount"),
                                            enclosing_type: None,
                                            line_info: LineInfo {
                                                line: 4,
                                                offset: 177,
                                            },
                                        },
                                        type_assignment: Type::Int,
                                        expression: None,
                                        line_info: LineInfo {
                                            line: 4,
                                            offset: 177,
                                        },
                                    }
                                ],
                            })
                        ],

                        conformances: vec![],
                    }
                )]
            }
        );
    }
}
