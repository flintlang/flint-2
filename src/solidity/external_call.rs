use super::*;

pub struct SolidityExternalCall {
    pub call: ExternalCall,
}

impl SolidityExternalCall {
    pub fn generate(&self, function_context: &mut FunctionContext) -> YulExpression {
        let gas = YulExpression::Literal(YulLiteral::Num(2300));
        let value = YulExpression::Literal(YulLiteral::Num(0));

        let rhs = *self.call.function_call.rhs_expression.clone();
        let call: FunctionCall = if let Expression::FunctionCall(f) = rhs {
            f
        } else {
            panic!("Solidity External Call RHS not function call")
        };

        let enclosing = if let Some(ref identifier) = call.identifier.enclosing_type {
            identifier
        } else {
            &function_context.enclosing_type
        };
        let matched = function_context.environment.match_function_call(
            &call,
            enclosing,
            &[],
            &function_context.scope_context,
        );

        let match_result: FunctionInformation;
        if let FunctionCallMatchResult::MatchedFunction(m) = matched.clone() {
            match_result = m;
        } else {
            panic!("Solidity External Call cannot match function call")
        }

        let function_selector = match_result.declaration.external_signature_hash().clone();
        let first_slice = &function_selector.clone()[..2];
        let second_slice = &function_selector.clone()[2..4];
        let third_slice = &function_selector.clone()[4..6];
        let fourth_slice = &function_selector.clone()[6..8];

        let address_expression = SolidityExpression {
            expression: *self.call.function_call.lhs_expression.clone(),
            is_lvalue: false,
        }
        .generate(function_context);

        let mut static_slots = vec![];
        let mut dynamic_slots = vec![];
        let mut static_size = 0;

        let param_types = match_result.declaration.head.parameter_types().clone();

        for param in param_types {
            match param {
                Type::Solidity(_) => static_size += 32,
                _ => panic!(
                    "Non Solidity Type not allowed in external call: {:?}",
                    param
                ),
            }
        }

        let dynamic_size = 0;

        let param_types = match_result.declaration.head.parameter_types().clone();
        let f_args = call.arguments.clone();

        let pairs: Vec<(Type, FunctionArgument)> =
            param_types.into_iter().zip(f_args.into_iter()).collect();

        for (p, q) in pairs {
            match p {
                Type::String => {
                    dynamic_slots.push(YulExpression::Literal(YulLiteral::Num(32)));
                    unimplemented!()
                }
                Type::Int => {
                    let expression = q.clone();
                    let expression = SolidityExpression {
                        expression: expression.expression.clone(),
                        is_lvalue: false,
                    }
                    .generate(function_context);
                    static_slots.push(expression);
                }
                Type::Address => {
                    let expression = q.clone();
                    let expression = SolidityExpression {
                        expression: expression.expression.clone(),
                        is_lvalue: false,
                    }
                    .generate(function_context);
                    static_slots.push(expression);
                }
                Type::Bool => {
                    let expression = q.clone();
                    let expression = SolidityExpression {
                        expression: expression.expression.clone(),
                        is_lvalue: false,
                    }
                    .generate(function_context);
                    static_slots.push(expression);
                }
                Type::Solidity(_) => {
                    let expression = q.clone();
                    let expression = SolidityExpression {
                        expression: expression.expression.clone(),
                        is_lvalue: false,
                    }
                    .generate(function_context);
                    static_slots.push(expression);
                }
                _ => panic!("Can not use non basic types in external call"),
            }
        }

        let call_input = function_context.fresh_variable();

        let input_size = 4 + static_size + dynamic_size;
        let mut slots = static_slots.clone();
        slots.append(&mut dynamic_slots);

        let output_size = 32;

        let call_success = function_context.fresh_variable();
        let call_output = function_context.fresh_variable();

        let statement =
            YulStatement::Expression(YulExpression::VariableDeclaration(YulVariableDeclaration {
                declaration: call_input.clone(),
                declaration_type: YulType::Any,
                expression: Option::from(Box::new(SolidityRuntimeFunction::allocate_memory(
                    input_size,
                ))),
            }));
        function_context.emit(statement);

        let statement = YulStatement::Expression(YulExpression::FunctionCall(YulFunctionCall {
            name: "mstore8".to_string(),
            arguments: vec![
                YulExpression::Identifier(call_input.clone()),
                YulExpression::Literal(YulLiteral::Hex(format!("0x{}", first_slice))),
            ],
        }));
        function_context.emit(statement);

        let statement = YulStatement::Expression(YulExpression::FunctionCall(YulFunctionCall {
            name: "mstore8".to_string(),
            arguments: vec![
                YulExpression::FunctionCall(YulFunctionCall {
                    name: "add".to_string(),
                    arguments: vec![
                        YulExpression::Identifier(call_input.clone()),
                        YulExpression::Literal(YulLiteral::Num(1)),
                    ],
                }),
                YulExpression::Literal(YulLiteral::Hex(format!("0x{}", second_slice))),
            ],
        }));
        function_context.emit(statement);

        let statement = YulStatement::Expression(YulExpression::FunctionCall(YulFunctionCall {
            name: "mstore8".to_string(),
            arguments: vec![
                YulExpression::FunctionCall(YulFunctionCall {
                    name: "add".to_string(),
                    arguments: vec![
                        YulExpression::Identifier(call_input.clone()),
                        YulExpression::Literal(YulLiteral::Num(2)),
                    ],
                }),
                YulExpression::Literal(YulLiteral::Hex(format!("0x{}", third_slice))),
            ],
        }));
        function_context.emit(statement);
        let statement = YulStatement::Expression(YulExpression::FunctionCall(YulFunctionCall {
            name: "mstore8".to_string(),
            arguments: vec![
                YulExpression::FunctionCall(YulFunctionCall {
                    name: "add".to_string(),
                    arguments: vec![
                        YulExpression::Identifier(call_input.clone()),
                        YulExpression::Literal(YulLiteral::Num(3)),
                    ],
                }),
                YulExpression::Literal(YulLiteral::Hex(format!("0x{}", fourth_slice))),
            ],
        }));
        function_context.emit(statement);

        let mut cur_position = 4;
        for slot in slots {
            let call = YulExpression::FunctionCall(YulFunctionCall {
                name: "add".to_string(),
                arguments: vec![
                    YulExpression::Identifier(call_input.clone()),
                    YulExpression::Literal(YulLiteral::Num(cur_position)),
                ],
            });
            let expresion =
                YulStatement::Expression(YulExpression::FunctionCall(YulFunctionCall {
                    name: "mstore".to_string(),
                    arguments: vec![call, slot.clone()],
                }));
            function_context.emit(expresion);
            cur_position += 32;
        }

        let statement =
            YulStatement::Expression(YulExpression::VariableDeclaration(YulVariableDeclaration {
                declaration: call_output.clone(),
                declaration_type: YulType::Any,
                expression: Option::from(Box::new(SolidityRuntimeFunction::allocate_memory(
                    output_size,
                ))),
            }));
        function_context.emit(statement);

        let call_exp = YulExpression::FunctionCall(YulFunctionCall {
            name: "call".to_string(),
            arguments: vec![
                gas,
                address_expression,
                value,
                YulExpression::Identifier(call_input.clone()),
                YulExpression::Literal(YulLiteral::Num(input_size)),
                YulExpression::Identifier(call_output.clone()),
                YulExpression::Literal(YulLiteral::Num(output_size)),
            ],
        });
        let var =
            YulStatement::Expression(YulExpression::VariableDeclaration(YulVariableDeclaration {
                declaration: call_success.clone(),
                declaration_type: YulType::Any,
                expression: Option::from(Box::new(call_exp)),
            }));

        function_context.emit(var);

        let f_call = YulExpression::FunctionCall(YulFunctionCall {
            name: "mload".to_string(),
            arguments: vec![YulExpression::Identifier(call_output.clone())],
        });
        let expression = YulStatement::Expression(YulExpression::Assignment(YulAssignment {
            identifiers: vec![call_output.clone()],
            expression: Box::new(f_call),
        }));

        function_context.emit(expression);

        YulExpression::Identifier(call_output)
    }
}
