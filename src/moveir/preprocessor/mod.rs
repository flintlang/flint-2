use self::utils::*;
use crate::ast::*;
use crate::context::*;
use crate::environment::*;
use crate::type_checker::ExpressionCheck;
use crate::visitor::Visitor;

pub mod utils;

pub(crate) struct MovePreProcessor {}

impl MovePreProcessor {
    const STATE_VAR_NAME: &'static str = "_contract_state";
}

impl Visitor for MovePreProcessor {
    fn start_contract_declaration(
        &mut self,
        contract: &mut ContractDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        if !contract.type_states.is_empty() {
            contract
                .contract_members
                .push(ContractMember::VariableDeclaration(
                    VariableDeclaration {
                        declaration_token: None,
                        identifier: Identifier {
                            token: MovePreProcessor::STATE_VAR_NAME.to_string(),
                            enclosing_type: None,
                            line_info: Default::default(),
                        },
                        variable_type: Type::TypeState,
                        expression: None,
                    },
                    None,
                ))
        }

        Ok(())
    }

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
                        .map(ContractBehaviourMember::FunctionDeclaration)
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
                if let ContractBehaviourMember::FunctionDeclaration(function) = m.clone() {
                    let wrapper = generate_contract_wrapper(function.clone(), _t, _ctx);
                    let wrapper = ContractBehaviourMember::FunctionDeclaration(wrapper);
                    let mut function = function;
                    function.head.modifiers.retain(|x| x != &Modifier::Public);
                    return vec![
                        ContractBehaviourMember::FunctionDeclaration(function),
                        wrapper,
                    ];
                } else {
                    return vec![m];
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
                        .map(StructMember::FunctionDeclaration)
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
                        .map(AssetMember::FunctionDeclaration)
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
            if let Some(context_ref) = &mut _ctx.scope_context {
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
            .token;

        let mangled_name =
            mangle_function_move(&_t.head.identifier.token, &enclosing_identifier, false);
        _t.mangled_identifier = Some(mangled_name);

        if _t.is_payable() {
            let payable_param = _t.first_payable_param();

            if payable_param.is_none() {
                panic!("lol")
            }
            let mut payable_param = payable_param.unwrap();
            let payable_param_name = payable_param.identifier.token.clone();
            let new_param_type =
                Type::UserDefinedType(Identifier::generated("Libra.Libra<LBR.LBR>"));
            payable_param.type_assignment = new_param_type;
            payable_param.identifier.token = mangle(&payable_param_name);
            let parameters = _t
                .head
                .parameters
                .clone()
                .into_iter()
                .map(|p| {
                    if p.identifier.token == payable_param_name {
                        payable_param.clone()
                    } else {
                        p
                    }
                })
                .collect();

            _t.head.parameters = parameters;

            let lhs = VariableDeclaration {
                declaration_token: None,
                identifier: Identifier::generated("amount"),
                variable_type: Type::UserDefinedType(Identifier::generated("Libra")),
                expression: None,
            };

            let lhs_expression = Expression::VariableDeclaration(lhs);

            let _lhs = Expression::Identifier(Identifier::generated("amount"));

            let rhs = Expression::FunctionCall(FunctionCall {
                identifier: Identifier::generated("Quartz_Self_Create_Libra"),
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

        if _ctx.asset_context.is_some() && enclosing_identifier != "Quartz$Global" {
            let asset_ctx = _ctx.asset_context.clone();
            let asset_ctx = asset_ctx.unwrap();
            let asset_ctx_identifier = asset_ctx.identifier;
            let param_type = Type::UserDefinedType(asset_ctx_identifier);
            let param_type = Type::InoutType(InoutType {
                key_type: Box::new(param_type),
            });
            let param_self_identifier = Identifier::generated(Identifier::SELF);

            let parameter = Parameter {
                identifier: param_self_identifier,
                type_assignment: param_type,
                expression: None,
                line_info: Default::default(),
            };

            _t.head.parameters.insert(0, parameter.clone());
            if let Some(ref scope) = _ctx.scope_context {
                let mut scope = scope.clone();
                scope.parameters.insert(0, parameter);

                _ctx.scope_context = Some(scope);
            }
        }

        if _ctx.struct_declaration_context.is_some() && enclosing_identifier != "Quartz_Global" {
            let struct_ctx = _ctx.struct_declaration_context.clone().unwrap();
            let struct_ctx_identifier = struct_ctx.identifier;
            let param_type = Type::UserDefinedType(struct_ctx_identifier);
            let param_type = Type::InoutType(InoutType {
                key_type: Box::new(param_type),
            });
            let param_self_identifier = Identifier::generated(Identifier::SELF);

            let parameter = Parameter {
                identifier: param_self_identifier,
                type_assignment: param_type,
                expression: None,
                line_info: Default::default(),
            };

            _t.head.parameters.insert(0, parameter.clone());
            if let Some(ref scope) = _ctx.scope_context {
                let mut scope = scope.clone();
                scope.parameters.insert(0, parameter);

                _ctx.scope_context = Some(scope);
            }
        }

        if _ctx.is_contract_behaviour_declaration_context() {
            let contract = _ctx.contract_behaviour_declaration_context.clone().unwrap();
            let identifier = contract.identifier.clone();
            let parameter_type = Type::UserDefinedType(identifier);
            let parameter_type = Type::InoutType(InoutType {
                key_type: Box::new(parameter_type),
            });
            let parameter = Parameter {
                identifier: Identifier::generated(Identifier::SELF),
                type_assignment: parameter_type,
                expression: None,
                line_info: Default::default(),
            };

            _t.head.parameters.insert(0, parameter.clone());
            _t.head
                .parameters
                .append(&mut _ctx.scope_context.as_ref().unwrap().parameters.clone());

            if let Some(scope) = _ctx.scope_context() {
                let mut scope = scope.clone();
                scope.parameters.insert(0, parameter);
                _ctx.scope_context = Some(scope);
            }
        }

        if let Some(ref scope) = _t.scope_context {
            let mut scope = scope.clone();
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
                identifier: Identifier::generated("ret"),
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
                        if let BinOp::Equal = b.op {
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
        if let Some(ref b_ctx) = _ctx.contract_behaviour_declaration_context {
            let caller_binding = b_ctx.caller.clone();
            if let Some(_) = caller_binding {
                _t.head.parameters.push(Parameter {
                    identifier: Identifier {
                        token: "caller".to_string(),
                        enclosing_type: None,
                        line_info: Default::default(),
                    },
                    type_assignment: Type::UserDefinedType(Identifier {
                        token: "&signer".to_string(),
                        enclosing_type: None,
                        line_info: Default::default(),
                    }),
                    expression: None,
                    line_info: Default::default(),
                });
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
                    *_t = expression;
                    if _ctx.is_function_declaration_context() {
                        let mut context = _ctx.function_declaration_context.clone().unwrap();
                        context.local_variables.push(variable.clone());

                        let scope = context.declaration.scope_context.clone();
                        let mut scope = scope.unwrap();
                        scope.local_variables.push(variable);

                        context.declaration.scope_context = Some(scope);
                        _ctx.function_declaration_context = Some(context)
                    } else if _ctx.is_special_declaration_context() {
                        let context = _ctx.special_declaration_context.clone();
                        let mut context = context.unwrap();
                        context.local_variables.push(variable.clone());

                        let scope = context.declaration.scope_context.clone();
                        let mut scope = scope;
                        scope.local_variables.push(variable);

                        context.declaration.scope_context = scope;
                        _ctx.special_declaration_context = Some(context);
                    } else if _ctx.has_scope_context() {
                        let scope = _ctx.scope_context.clone();
                        let mut scope = scope.unwrap();
                        scope.local_variables.push(variable);

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
            _ctx.function_call_receiver_trail
                .push(*_t.lhs_expression.clone());
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

    fn finish_binary_expression(
        &mut self,
        _t: &mut BinaryExpression,
        _ctx: &mut Context,
    ) -> VResult {
        if let BinOp::Dot = _t.op {
            _ctx.function_call_receiver_trail.clear();
        }

        Ok(())
    }

    fn start_function_call(&mut self, _t: &mut FunctionCall, _ctx: &mut Context) -> VResult {
        if Environment::is_runtime_function_call(_t) {
            return Ok(());
        }

        if let Some(mangled) = mangle_function_call_name(_t, _ctx) {
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

            let enclosing_type = _ctx.enclosing_type_identifier().unwrap_or_default().token;

            let caller_protections =
                if let Some(ref behaviour) = _ctx.contract_behaviour_declaration_context {
                    behaviour.caller_protections.clone()
                } else {
                    vec![]
                };

            let scope = _ctx.scope_context.clone().unwrap_or_default();

            let receiver_trail = &mut _ctx.function_call_receiver_trail;

            if receiver_trail.is_empty() {
                *receiver_trail = vec![Expression::SelfExpression]
            }

            let declared_enclosing = if is_global_function_call {
                "Quartz_Global".to_string()
            } else {
                let receiver = receiver_trail.last().unwrap().clone();
                _ctx.environment
                    .get_expression_type(
                        receiver,
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
                let expressions = receiver_trail;

                let mut expression = construct_expression(expressions.clone());

                if expression.enclosing_type().is_some() {
                    expression = expand_properties(expression, _ctx, false);
                } else if let Expression::BinaryExpression(_) = expression {
                    expression = expand_properties(expression, _ctx, false);
                }

                let enclosing_type = _ctx.enclosing_type_identifier().unwrap_or_default().token;

                let result_type = match expression.clone() {
                    Expression::Identifier(i) => {
                        if let Some(ref result) = scope.type_for(&i.token) {
                            result.clone()
                        } else {
                            _ctx.environment.get_expression_type(
                                expression.clone(),
                                &enclosing_type,
                                vec![],
                                vec![],
                                scope,
                            )
                        }
                    }
                    _ => _ctx.environment.get_expression_type(
                        expression.clone(),
                        &enclosing_type,
                        vec![],
                        vec![],
                        scope,
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

    fn start_external_call(&mut self, _t: &mut ExternalCall, _ctx: &mut Context) -> VResult {
        if _ctx.scope_context.is_none() {
            panic!("Not Enough Information To Workout External Trait name")
        }

        if _ctx.enclosing_type_identifier().is_none() {
            panic!("Not Enough Information To Workout External Trait name")
        }
        let scope = _ctx.scope_context.clone().unwrap();
        let enclosing = _ctx.enclosing_type_identifier();
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

    fn finish_return_statement(&mut self, _t: &mut ReturnStatement, _ctx: &mut Context) -> VResult {
        _t.cleanup = _ctx.post_statements.clone();
        _ctx.post_statements = vec![];
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
            expression = *i.expression;

            if let Some(ref scope) = _ctx.scope_context {
                if let Some(ref enclosing) = _ctx.enclosing_type_identifier() {
                    let enclosing = &enclosing.token;
                    let caller_protections =
                        if let Some(ref behaviour) = _ctx.contract_behaviour_declaration_context {
                            behaviour.caller_protections.clone()
                        } else {
                            vec![]
                        };
                    let expression_type = _ctx.environment.get_expression_type(
                        expression.clone(),
                        enclosing,
                        vec![],
                        caller_protections,
                        scope.clone(),
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
                    expression = pre_assign(expression, _ctx, borrow_local, true);
                }
            }
            Expression::BinaryExpression(b) => {
                if let BinOp::Dot = b.op {
                    expression = expand_properties(expression, _ctx, borrow_local)
                }
            }
            _ => {
                if let Expression::InoutExpression(_) = function_argument.expression {
                    expression = function_argument.expression
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

    fn finish_statement(&mut self, statement: &mut Statement, context: &mut Context) -> VResult {
        if let Statement::BecomeStatement(bs) = statement {
            // MID we should be in a contract behaviour context since we are using type states
            let contract_name = &context
                .contract_behaviour_declaration_context
                .as_ref()
                .unwrap()
                .identifier
                .token;
            let declared_states = context.environment.get_contract_type_states(contract_name);
            // We immediately unwrap as all become statements should have been checked for having a declared typestate
            let type_state_as_u8 = declared_states
                .iter()
                .position(|state| state == &bs.state)
                .unwrap() as u8;

            let state_variable = if context.is_special_declaration_context() {
                // Special declarations have no 'this' yet as it is being constructed
                // TODO the mangling is a problem
                Expression::Identifier(Identifier::generated(&format!(
                    "_this_{}",
                    MovePreProcessor::STATE_VAR_NAME
                )))
            } else {
                Expression::BinaryExpression(BinaryExpression {
                    lhs_expression: Box::new(Expression::SelfExpression),
                    rhs_expression: Box::new(Expression::Identifier(Identifier::generated(
                        MovePreProcessor::STATE_VAR_NAME,
                    ))),
                    op: BinOp::Dot,
                    line_info: Default::default(),
                })
            };

            *statement = Statement::Expression(Expression::BinaryExpression(BinaryExpression {
                lhs_expression: Box::new(state_variable),
                rhs_expression: Box::new(Expression::Literal(Literal::U8Literal(type_state_as_u8))),
                op: BinOp::Equal,
                line_info: Default::default(),
            }));
        }
        Ok(())
    }
}
