mod utils;

use crate::ast::*;
use crate::context::*;
use crate::environment::*;
use crate::moveir::preprocessor::utils::*;
use crate::moveir::{FunctionContext, MoveExpression, MoveIRBlock};
use crate::type_checker::ExpressionCheck;
use crate::visitor::Visitor;

pub(crate) struct MovePreProcessor {}

impl Visitor for MovePreProcessor {
    fn start_contract_behaviour_declaration(
        &mut self,
        _t: &mut ContractBehaviourDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        _t.members = _t
            .members
            .clone()
            .into_iter()
            .flat_map(|f| {
                if let ContractBehaviourMember::FunctionDeclaration(fd) = f {
                    let functions =
                        convert_default_parameter_functions(fd, &_t.identifier.token, _ctx);
                    functions
                        .into_iter()
                        .map(|f| ContractBehaviourMember::FunctionDeclaration(f))
                        .collect()
                } else {
                    vec![f]
                }
            })
            .collect();
        Ok(())
    }

    fn finish_contract_behaviour_declaration(
        &mut self,
        _t: &mut ContractBehaviourDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        _t.members = _t
            .members
            .clone()
            .into_iter()
            .flat_map(|m| {
                if let ContractBehaviourMember::FunctionDeclaration(f) = m.clone() {
                    let wrapper = generate_contract_wrapper(f.clone(), _t, _ctx);
                    let wrapper = ContractBehaviourMember::FunctionDeclaration(wrapper);
                    let mut function = f.clone();
                    function.head.modifiers.retain(|x| *x != "public");
                    let function = ContractBehaviourMember::FunctionDeclaration(function);
                    return vec![function, wrapper.clone()];
                } else {
                    return vec![m.clone()];
                }
            })
            .collect();
        Ok(())
    }

    fn start_struct_declaration(
        &mut self,
        _t: &mut StructDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        _t.members = _t
            .members
            .clone()
            .into_iter()
            .flat_map(|f| {
                if let StructMember::FunctionDeclaration(fd) = f {
                    let functions =
                        convert_default_parameter_functions(fd, &_t.identifier.token, _ctx);
                    functions
                        .into_iter()
                        .map(|f| StructMember::FunctionDeclaration(f))
                        .collect()
                } else {
                    vec![f]
                }
            })
            .collect();
        Ok(())
    }

    fn start_asset_declaration(
        &mut self,
        _t: &mut AssetDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        _t.members = _t
            .members
            .clone()
            .into_iter()
            .flat_map(|f| {
                if let AssetMember::FunctionDeclaration(fd) = f {
                    let functions =
                        convert_default_parameter_functions(fd, &_t.identifier.token, _ctx);
                    functions
                        .into_iter()
                        .map(|f| AssetMember::FunctionDeclaration(f))
                        .collect()
                } else {
                    vec![f]
                }
            })
            .collect();
        Ok(())
    }

    fn start_variable_declaration(
        &mut self,
        _t: &mut VariableDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        if _ctx.in_function_or_special() {
            if _ctx.scope_context().is_some() {
                let context_ref = _ctx.scope_context.as_mut().unwrap();
                context_ref.local_variables.push(_t.clone());
            }

            if _ctx.is_function_declaration_context() {
                let context_ref = _ctx.function_declaration_context.as_mut().unwrap();
                context_ref.local_variables.push(_t.clone());
            }

            if _ctx.is_special_declaration_context() {
                let context_ref = _ctx.special_declaration_context.as_mut().unwrap();
                context_ref.local_variables.push(_t.clone());
            }
        }
        Ok(())
    }

    fn start_function_declaration(
        &mut self,
        _t: &mut FunctionDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        let enclosing_identifier = _ctx
            .enclosing_type_identifier()
            .unwrap_or(Identifier {
                token: "".to_string(),
                enclosing_type: None,
                line_info: Default::default(),
            })
            .token
            .clone();

        let mangled_name = mangle_function_move(
            _t.head.identifier.token.clone(),
            &enclosing_identifier,
            false,
        );
        _t.mangled_identifier = Some(mangled_name);

        if _t.is_payable() {
            let payable_param = _t.first_payable_param().clone();

            if payable_param.is_none() {
                panic!("lol")
            }
            let mut payable_param = payable_param.unwrap();
            let payable_param_name = payable_param.identifier.token.clone();
            let new_param_type = Type::UserDefinedType(Identifier {
                token: "LibraCoin.T".to_string(),
                enclosing_type: None,
                line_info: Default::default(),
            });
            payable_param.type_assignment = new_param_type;
            let mut ident = payable_param.identifier.clone();
            ident.token = mangle(payable_param_name.clone());
            payable_param.identifier = ident;
            let parameters = _t.head.parameters.clone();
            let parameters = parameters
                .into_iter()
                .map(|p| {
                    if p.identifier.token.clone() == payable_param_name {
                        payable_param.clone()
                    } else {
                        p
                    }
                })
                .collect();

            _t.head.parameters = parameters;

            let lhs = VariableDeclaration {
                declaration_token: None,
                identifier: Identifier {
                    token: "amount".to_string(),
                    enclosing_type: None,
                    line_info: Default::default(),
                },
                variable_type: Type::UserDefinedType(Identifier {
                    token: "Libra".to_string(),
                    enclosing_type: None,
                    line_info: Default::default(),
                }),
                expression: None,
            };

            let lhs_expression = Expression::VariableDeclaration(lhs.clone());

            let _lhs = Expression::Identifier(Identifier {
                token: "amount".to_string(),
                enclosing_type: None,
                line_info: Default::default(),
            });

            let rhs = Expression::FunctionCall(FunctionCall {
                identifier: Identifier {
                    token: "Quartz_Self_Create_Libra".to_string(),
                    enclosing_type: None,
                    line_info: Default::default(),
                },
                arguments: vec![FunctionArgument {
                    identifier: None,
                    expression: Expression::Identifier(payable_param.identifier),
                }],
                mangled_identifier: None,
            });
            let assignment = BinaryExpression {
                lhs_expression: Box::new(lhs_expression),
                rhs_expression: Box::new(rhs),
                op: BinOp::Equal,
                line_info: Default::default(),
            };
            _t.body.insert(
                0,
                Statement::Expression(Expression::BinaryExpression(assignment)),
            );
        }

        if _ctx.asset_context.is_some() && enclosing_identifier != "Quartz$Global".to_string() {
            let asset_ctx = _ctx.asset_context.clone();
            let asset_ctx = asset_ctx.unwrap();
            let asset_ctx_identifier = asset_ctx.identifier.clone();
            let param_type = Type::UserDefinedType(asset_ctx_identifier);
            let param_type = Type::InoutType(InoutType {
                key_type: Box::new(param_type),
            });
            let param_self_identifier = Identifier {
                token: "self".to_string(),
                enclosing_type: None,
                line_info: Default::default(),
            };

            let parameter = Parameter {
                identifier: param_self_identifier,
                type_assignment: param_type,
                expression: None,
                line_info: Default::default(),
            };

            _t.head.parameters.insert(0, parameter.clone());
            if _ctx.scope_context.is_some() {
                let scope = _ctx.scope_context.clone();
                let mut scope = scope.unwrap();
                scope.parameters.insert(0, parameter);

                _ctx.scope_context = Some(scope);
            }
        }

        if _ctx.struct_declaration_context.is_some()
            && enclosing_identifier != "Quartz_Global".to_string()
        {
            let struct_ctx = _ctx.struct_declaration_context.clone();
            let struct_ctx = struct_ctx.unwrap();
            let struct_ctx_identifier = struct_ctx.identifier.clone();
            let param_type = Type::UserDefinedType(struct_ctx_identifier);
            let param_type = Type::InoutType(InoutType {
                key_type: Box::new(param_type),
            });
            let param_self_identifier = Identifier {
                token: "self".to_string(),
                enclosing_type: None,
                line_info: Default::default(),
            };

            let parameter = Parameter {
                identifier: param_self_identifier,
                type_assignment: param_type,
                expression: None,
                line_info: Default::default(),
            };

            _t.head.parameters.insert(0, parameter.clone());
            if _ctx.scope_context.is_some() {
                let scope = _ctx.scope_context.clone();
                let mut scope = scope.unwrap();
                scope.parameters.insert(0, parameter);

                _ctx.scope_context = Some(scope);
            }
        }

        if _ctx.is_contract_behaviour_declaration_context() {
            let contract = _ctx.contract_behaviour_declaration_context.clone();
            let contract = contract.unwrap();
            let identifier = contract.identifier.clone();
            let parameter_type = Type::UserDefinedType(identifier);
            let parameter_type = Type::InoutType(InoutType {
                key_type: Box::new(parameter_type),
            });
            let parameter = Parameter {
                identifier: Identifier {
                    token: "self".to_string(),
                    enclosing_type: None,
                    line_info: Default::default(),
                },
                type_assignment: parameter_type,
                expression: None,
                line_info: Default::default(),
            };

            _t.head.parameters.insert(0, parameter.clone());

            if _ctx.scope_context().is_some() {
                let scope = _ctx.scope_context.clone();
                let mut scope = scope.unwrap();
                scope.parameters.insert(0, parameter.clone());
                _ctx.scope_context = Some(scope);
            }

            if contract.caller.is_some() {
                let caller = contract.caller.unwrap();

                _t.body.insert(0, generate_caller_statement(caller))
            }
        }

        let scope = _t.scope_context.clone();
        if scope.is_some() {
            let mut scope = scope.unwrap();
            scope.parameters = _t.head.parameters.clone();
            _t.scope_context = Some(scope);
        }
        Ok(())
    }

    fn finish_function_declaration(
        &mut self,
        _t: &mut FunctionDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        let function_declaration = _t;
        let mut statements = get_declaration(_ctx);

        let mut deletions = delete_declarations(function_declaration.body.clone());

        statements.append(&mut deletions);
        function_declaration.body = statements;

        if function_declaration.is_void() {
            let statement = function_declaration.body.last();
            if !function_declaration.body.is_empty() {
                if let Statement::ReturnStatement(_) = statement.unwrap() {
                } else {
                    function_declaration
                        .body
                        .push(Statement::ReturnStatement(ReturnStatement {
                            expression: None,
                            ..Default::default()
                        }));
                }
            } else {
                function_declaration
                    .body
                    .push(Statement::ReturnStatement(ReturnStatement {
                        expression: None,
                        ..Default::default()
                    }));
            }
        } else {
            let variable_declaration = VariableDeclaration {
                declaration_token: None,
                identifier: Identifier {
                    token: "ret".to_string(),
                    enclosing_type: None,
                    line_info: Default::default(),
                },
                variable_type: function_declaration
                    .head
                    .result_type
                    .as_ref()
                    .unwrap()
                    .clone(),
                expression: None,
            };
            function_declaration.body.insert(
                0,
                Statement::Expression(Expression::VariableDeclaration(variable_declaration)),
            )
        }

        Ok(())
    }

    fn start_special_declaration(
        &mut self,
        _t: &mut SpecialDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        let members = _t.body.clone();

        let members = members
            .into_iter()
            .filter(|m| {
                if let Statement::Expression(e) = m.clone() {
                    if let Expression::BinaryExpression(b) = e {
                        if let BinOp::Equal = b.op.clone() {
                            if let Expression::DictionaryLiteral(_) = *b.rhs_expression {
                                return false;
                            }
                        }
                    }
                }
                true
            })
            .collect();

        _t.body = members;
        if _ctx.contract_behaviour_declaration_context.is_some() {
            let b_ctx = _ctx.contract_behaviour_declaration_context.clone();
            let b_ctx = b_ctx.unwrap();
            let caller_binding = b_ctx.caller.clone();
            if caller_binding.is_some() {
                let caller_binding = caller_binding.unwrap();
                _t.body.insert(0, generate_caller_statement(caller_binding))
            }
        }
        Ok(())
    }

    fn finish_special_declaration(
        &mut self,
        _t: &mut SpecialDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        let function_declaration = _t;
        let body = function_declaration.body.clone();
        let mut statements = get_declaration(_ctx);
        if statements.is_empty() {}
        for statement in body {
            statements.push(statement.clone())
        }
        function_declaration.body = statements;

        Ok(())
    }

    fn start_expression(&mut self, _t: &mut Expression, _ctx: &mut Context) -> VResult {
        if let Expression::BinaryExpression(b) = _t {
            if let BinOp::Dot = b.op {
                if let Expression::Identifier(lhs) = &*b.lhs_expression {
                    if let Expression::Identifier(rhs) = &*b.rhs_expression {
                        if _ctx.environment.is_enum_declared(&lhs.token) {
                            let property = _ctx.environment.property_declarations(&lhs.token);
                            let property: Vec<Property> = property
                                .into_iter()
                                .filter(|p| p.get_identifier().token == rhs.token)
                                .collect();

                            if !property.is_empty() {
                                let property = property.first().unwrap();
                                if property.get_type() != Type::Error {
                                    *_t = property.get_value().unwrap()
                                }
                            }
                        }
                    }
                }
            } else if let BinOp::Equal = b.op {
                if let Expression::VariableDeclaration(v) = &*b.lhs_expression {
                    let variable = v.clone();
                    let identifier = if v.identifier.is_self() {
                        Expression::SelfExpression
                    } else {
                        Expression::Identifier(v.identifier.clone())
                    };
                    let expression = Expression::BinaryExpression(BinaryExpression {
                        lhs_expression: Box::new(identifier),
                        rhs_expression: b.rhs_expression.clone(),
                        op: BinOp::Equal,
                        line_info: b.line_info.clone(),
                    });
                    *_t = expression.clone();
                    if _ctx.is_function_declaration_context() {
                        let context = _ctx.function_declaration_context.clone();
                        let mut context = context.unwrap();
                        context.local_variables.push(variable.clone());

                        let scope = context.declaration.scope_context.clone();
                        let mut scope = scope.unwrap();
                        scope.local_variables.push(variable.clone());

                        context.declaration.scope_context = Some(scope);
                        _ctx.function_declaration_context = Some(context)
                    }

                    if _ctx.is_special_declaration_context() {
                        let context = _ctx.special_declaration_context.clone();
                        let mut context = context.unwrap();
                        context.local_variables.push(variable.clone());

                        let scope = context.declaration.scope_context.clone();
                        let mut scope = scope;
                        scope.local_variables.push(variable.clone());

                        context.declaration.scope_context = scope;
                        _ctx.special_declaration_context = Some(context);
                    }

                    if _ctx.has_scope_context() {
                        let scope = _ctx.scope_context.clone();
                        let mut scope = scope.unwrap();
                        scope.local_variables.push(variable.clone());

                        _ctx.scope_context = Some(scope)
                    }
                }
            }
        }
        Ok(())
    }

    fn start_binary_expression(
        &mut self,
        _t: &mut BinaryExpression,
        _ctx: &mut Context,
    ) -> VResult {
        if _t.op.is_assignment_shorthand() {
            let op = _t.op.clone();
            let op = op.get_assignment_shorthand();
            _t.op = BinOp::Equal;

            let rhs = BinaryExpression {
                lhs_expression: _t.lhs_expression.clone(),
                rhs_expression: _t.rhs_expression.clone(),
                op,
                line_info: _t.line_info.clone(),
            };
            _t.rhs_expression = Box::from(Expression::BinaryExpression(rhs));
        } else if let BinOp::Dot = _t.op {
            let mut trail = _ctx.function_call_receiver_trail.clone();
            trail.push(*_t.lhs_expression.clone());

            _ctx.function_call_receiver_trail = trail;
            match *_t.lhs_expression.clone() {
                Expression::Identifier(_) => {
                    if let Expression::FunctionCall(_) = *_t.rhs_expression {
                    } else {
                        let lhs = _t.lhs_expression.clone();
                        let lhs = *lhs;
                        let lhs = expand_properties(lhs, _ctx, false);
                        _t.lhs_expression = Box::from(lhs);
                    }
                }
                Expression::BinaryExpression(b) => {
                    if let BinOp::Dot = b.op {
                        let lhs = _t.lhs_expression.clone();
                        let lhs = *lhs;
                        let lhs = expand_properties(lhs, _ctx, false);
                        _t.lhs_expression = Box::from(lhs);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn finish_return_statement(&mut self, _t: &mut ReturnStatement, _ctx: &mut Context) -> VResult {
        _t.cleanup = _ctx.post_statements.clone();
        _ctx.post_statements = vec![];
        Ok(())
    }

    fn start_external_call(&mut self, _t: &mut ExternalCall, _ctx: &mut Context) -> VResult {
        if _ctx.scope_context.is_none() {
            panic!("Not Enough Information To Workout External Trait name")
        }

        if _ctx.enclosing_type_identifier().is_none() {
            panic!("Not Enough Information To Workout External Trait name")
        }
        let scope = _ctx.scope_context.clone();
        let scope = scope.unwrap();
        let enclosing = _ctx.enclosing_type_identifier().clone();
        let enclosing = enclosing.unwrap();
        let enclosing = enclosing.token;
        let receiver = _t.function_call.lhs_expression.clone();
        let receiver = *receiver;
        let receiver_type =
            _ctx.environment
                .get_expression_type(receiver, &enclosing, vec![], vec![], scope);
        _t.external_trait_name = Option::from(receiver_type.name());
        Ok(())
    }

    fn start_function_call(&mut self, _t: &mut FunctionCall, _ctx: &mut Context) -> VResult {
        let mut receiver_trail = _ctx.function_call_receiver_trail.clone();

        if Environment::is_runtime_function_call(_t) {
            return Ok(());
        }

        if receiver_trail.is_empty() {
            receiver_trail = vec![Expression::SelfExpression]
        }

        let mangled = mangle_function_call_name(_t, _ctx);
        if mangled.is_some() {
            let mangled = mangled.unwrap();
            _t.mangled_identifier = Option::from(Identifier {
                token: mangled,
                enclosing_type: None,
                line_info: Default::default(),
            });
        }

        let function_call = _t.clone();
        if !_ctx.environment.is_initiliase_call(function_call.clone())
            && !_ctx
                .environment
                .is_trait_declared(&function_call.identifier.token)
        {
            let is_global_function_call = is_global_function_call(function_call, _ctx);

            let enclosing_type = _ctx.enclosing_type_identifier();
            let enclosing_type = enclosing_type.unwrap_or_default();
            let enclosing_type = enclosing_type.token;

            let caller_protections = if _ctx.contract_behaviour_declaration_context.is_some() {
                let behaviour = _ctx.contract_behaviour_declaration_context.clone();
                let behaviour = behaviour.unwrap();
                behaviour.caller_protections
            } else {
                vec![]
            };

            let scope = _ctx.scope_context.clone();
            let scope = scope.unwrap_or_default();

            let declared_enclosing = if is_global_function_call {
                "Quartz_Global".to_string()
            } else {
                let receiver = receiver_trail.last();
                let receiver = receiver.unwrap();
                let receivier = receiver.clone();
                _ctx.environment
                    .get_expression_type(
                        receivier,
                        &enclosing_type,
                        vec![],
                        caller_protections,
                        scope.clone(),
                    )
                    .name()
            };

            if _ctx.environment.is_struct_declared(&declared_enclosing)
                || _ctx.environment.is_contract_declared(&declared_enclosing)
                || _ctx.environment.is_trait_declared(&declared_enclosing)
                || _ctx.environment.is_asset_declared(&declared_enclosing)
                    && !is_global_function_call
            {
                let expresssions = receiver_trail.clone();

                let mut expression = construct_expression(expresssions);

                if expression.enclosing_type().is_some() {
                    expression = expand_properties(expression, _ctx, false);
                } else if let Expression::BinaryExpression(_) = expression.clone() {
                    expression = expand_properties(expression, _ctx, false);
                }

                let enclosing_type = _ctx.enclosing_type_identifier();
                let enclosing_type = enclosing_type.unwrap_or_default();
                let enclosing_type = enclosing_type.token;

                let result_type = match expression.clone() {
                    Expression::Identifier(i) => {
                        if scope.type_for(i.token.clone()).is_some() {
                            let result = scope.type_for(i.token).clone();
                            result.unwrap()
                        } else {
                            _ctx.environment.get_expression_type(
                                expression.clone(),
                                &enclosing_type,
                                vec![],
                                vec![],
                                scope.clone(),
                            )
                        }
                    }
                    _ => _ctx.environment.get_expression_type(
                        expression.clone(),
                        &enclosing_type,
                        vec![],
                        vec![],
                        scope.clone(),
                    ),
                };

                if !result_type.is_inout_type() {
                    let inout = InoutExpression {
                        ampersand_token: "".to_string(),
                        expression: Box::new(expression.clone()),
                    };
                    expression = Expression::InoutExpression(inout)
                }

                let mut arguments = _t.arguments.clone();
                arguments.insert(
                    0,
                    FunctionArgument {
                        identifier: None,
                        expression,
                    },
                );

                _t.arguments = arguments;
            }
        }

        _ctx.function_call_receiver_trail = vec![];

        Ok(())
    }

    fn start_function_argument(
        &mut self,
        _t: &mut FunctionArgument,
        _ctx: &mut Context,
    ) -> VResult {
        let mut borrow_local = false;
        let function_argument = _t.clone();
        let mut expression;
        if let Expression::InoutExpression(i) = function_argument.expression.clone() {
            expression = *i.expression.clone();

            if _ctx.scope_context.is_some() {
                let scope = _ctx.scope_context.clone();
                let scope = scope.unwrap();

                if _ctx.enclosing_type_identifier().is_some() {
                    let enclosing = _ctx.enclosing_type_identifier().clone();
                    let enclosing = enclosing.unwrap();
                    let enclosing = enclosing.token;
                    let caller_protections =
                        if _ctx.contract_behaviour_declaration_context.is_some() {
                            let behaviour = _ctx.contract_behaviour_declaration_context.clone();
                            let behaviour = behaviour.unwrap();
                            behaviour.caller_protections
                        } else {
                            vec![]
                        };
                    let expression_type = _ctx.environment.get_expression_type(
                        expression.clone(),
                        &enclosing,
                        vec![],
                        caller_protections,
                        scope,
                    );

                    if !expression_type.is_currency_type()
                        && !expression_type.is_external_resource(_ctx.environment.clone())
                    {
                        borrow_local = true;
                    }
                } else {
                    borrow_local = true;
                }
            } else {
                borrow_local = true;
            }
        } else {
            expression = function_argument.expression.clone();
        }

        match expression.clone() {
            Expression::Identifier(ident) => {
                if ident.enclosing_type.is_some() {
                    let _ident_enclosing = ident.enclosing_type.clone();
                    expression = pre_assign(expression, _ctx, borrow_local, true);
                }
            }
            Expression::BinaryExpression(b) => {
                if let BinOp::Dot = b.op {
                    expression = expand_properties(expression, _ctx, borrow_local)
                }
            }
            _ => {
                if let Expression::InoutExpression(_) = function_argument.expression.clone() {
                    expression = function_argument.expression.clone()
                }
            }
        }

        _t.expression = expression;
        Ok(())
    }

    fn start_type(&mut self, _t: &mut Type, _ctx: &mut Context) -> VResult {
        if _t.is_external_contract(_ctx.environment.clone()) {
            *_t = Type::Address
        }
        Ok(())
    }
}
