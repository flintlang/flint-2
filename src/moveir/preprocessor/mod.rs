use self::utils::*;
use crate::ast::*;
use crate::context::*;
use crate::environment::*;
use crate::type_checker::ExpressionChecker;
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
        declaration: &mut ContractBehaviourDeclaration,
        ctx: &mut Context,
    ) -> VResult {
        // If we are in the declaration that contains the initialiser, then that is where we will insert the
        // getters and setters since there are no caller protections or type state restrictions
        // TODO the above explanation is somewhat hacky
        let is_init_decl = |member: &ContractBehaviourMember| {
            if let ContractBehaviourMember::SpecialDeclaration(special) = member {
                special.head.special_token.eq("init")
            } else {
                false
            }
        };
        if declaration.members.iter().any(is_init_decl) {
            let non_private_contract_members = ctx
                .environment
                .property_declarations(&declaration.identifier.token)
                .into_iter()
                // Some(_) ensures it has some modifier, and is therefore not private
                .filter(|property| property.get_modifier().is_some())
                .collect::<Vec<Property>>();

            for non_private_contract_member in non_private_contract_members {
                match non_private_contract_member.get_modifier().as_ref().unwrap() {
                    Modifier::Public => {
                        generate_and_add_getter(&non_private_contract_member, declaration, ctx);
                        generate_and_add_setter(&non_private_contract_member, declaration);
                    }
                    Modifier::Visible => {
                        generate_and_add_getter(&non_private_contract_member, declaration, ctx)
                    }
                }
            }
        }

        declaration.members = declaration
            .members
            .clone()
            .into_iter()
            .flat_map(|f| {
                if let ContractBehaviourMember::FunctionDeclaration(fd) = f {
                    let functions =
                        convert_default_parameter_functions(fd, &declaration.identifier.token, ctx);
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
        declaration: &mut VariableDeclaration,
        ctx: &mut Context,
    ) -> VResult {
        if ctx.in_function_or_special() {
            if let Some(ref mut scope_context) = ctx.scope_context {
                scope_context.local_variables.push(declaration.clone());
            }

            // If is function declaration context
            if let Some(ref mut function_declaration_context) = ctx.function_declaration_context {
                function_declaration_context
                    .local_variables
                    .push(declaration.clone());
            }

            // If is special declaration context  // TODO should these be else ifs?
            if let Some(ref mut special_declaration_context) = ctx.special_declaration_context {
                special_declaration_context.local_variables.push(declaration.clone());
            }
        }
        Ok(())
    }

    fn start_function_declaration(
        &mut self,
        declaration: &mut FunctionDeclaration,
        ctx: &mut Context,
    ) -> VResult {
        let enclosing_identifier = ctx
            .enclosing_type_identifier()
            .map(|id| id.token.to_string())
            .unwrap_or_default();

        let mangled_name = mangle_function_move(
            &declaration.head.identifier.token,
            &enclosing_identifier,
            false,
        );
        declaration.mangled_identifier = Some(mangled_name);

        if declaration.is_payable() {
            let payable_param = declaration.first_payable_param();

            if payable_param.is_none() {
                panic!("lol")
            }
            let mut payable_param = payable_param.unwrap();
            let payable_param_name = payable_param.identifier.token.clone();
            let new_param_type =
                Type::UserDefinedType(Identifier::generated("Libra.Libra<LBR.LBR>"));
            payable_param.type_assignment = new_param_type;
            payable_param.identifier.token = mangle(&payable_param_name);
            let parameters = declaration
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

            declaration.head.parameters = parameters;

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
            declaration.body.insert(
                0,
                Statement::Expression(Expression::BinaryExpression(assignment)),
            );
        }

        if ctx.asset_context.is_some() && enclosing_identifier != "Quartz$Global" {
            let asset_ctx = ctx.asset_context.clone();
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

            declaration.head.parameters.insert(0, parameter.clone());
            if let Some(ref mut scope) = ctx.scope_context {
                scope.parameters.insert(0, parameter);
            }
        }

        if ctx.struct_declaration_context.is_some() && enclosing_identifier != "Quartz_Global" {
            let struct_ctx = ctx.struct_declaration_context.clone().unwrap();
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

            declaration.head.parameters.insert(0, parameter.clone());
            if let Some(ref mut scope) = ctx.scope_context {
                scope.parameters.insert(0, parameter);
            }
        }

        // If is contract behaviour declaration context
        if let Some(ref contract) = ctx.contract_behaviour_declaration_context {
            let identifier = &contract.identifier;
            let parameter_type = Type::InoutType(InoutType {
                key_type: Box::new(Type::UserDefinedType(identifier.clone())),
            });
            let parameter = Parameter {
                identifier: Identifier::generated(Identifier::SELF),
                type_assignment: parameter_type,
                expression: None,
                line_info: Default::default(),
            };

            declaration.head.parameters.insert(0, parameter.clone());
            declaration
                .head
                .parameters
                .append(&mut ctx.scope_context.as_mut().unwrap().parameters);

            if let Some(ref mut scope) = ctx.scope_context {
                scope.parameters.insert(0, parameter);
            }
        }

        if let Some(ref scope) = declaration.scope_context {
            let mut scope = scope.clone();
            scope.parameters = declaration.head.parameters.clone();
            declaration.scope_context = Some(scope);
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
            if b_ctx.caller.is_some() {
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
        special: &mut SpecialDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        let mut statements = get_declaration(_ctx);
        statements.append(&mut special.body.clone());
        special.body = statements;

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
                    // If is function declaration context, or else if special declaration context
                    if let Some(ref mut function_declaration_context) =
                        _ctx.function_declaration_context
                    {
                        function_declaration_context
                            .local_variables
                            .push(variable.clone());
                        function_declaration_context
                            .declaration
                            .scope_context
                            .as_mut()
                            .unwrap()
                            .local_variables
                            .push(variable);
                    } else if let Some(ref mut special_declaration_context) =
                        _ctx.special_declaration_context
                    {
                        special_declaration_context
                            .local_variables
                            .push(variable.clone());
                        special_declaration_context
                            .declaration
                            .scope_context
                            .local_variables
                            .push(variable);
                    } else if let Some(ref mut scope_context) = _ctx.scope_context {
                        scope_context.local_variables.push(variable);
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
            let op = _t.op.get_assignment_shorthand();
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

    fn start_function_call(&mut self, call: &mut FunctionCall, _ctx: &mut Context) -> VResult {
        if Environment::is_runtime_function_call(call) {
            return Ok(());
        }

        if let Some(mangled) = mangle_function_call_name(call, _ctx) {
            call.mangled_identifier = Option::from(Identifier {
                token: mangled,
                enclosing_type: None,
                line_info: Default::default(),
            });
        }

        if !_ctx.environment.is_initialise_call(&call)
            && !_ctx.environment.is_trait_declared(&call.identifier.token)
        {
            let is_global_function_call = is_global_function_call(&call, _ctx);

            let enclosing_type = _ctx
                .enclosing_type_identifier()
                .map(|id| id.token.to_string())
                .unwrap_or_default();

            let caller_protections: &[_] =
                if let Some(ref behaviour) = _ctx.contract_behaviour_declaration_context {
                    &behaviour.caller_protections
                } else {
                    &[]
                };

            let receiver_trail = &mut _ctx.function_call_receiver_trail;

            if receiver_trail.is_empty() {
                *receiver_trail = vec![Expression::SelfExpression]
            }

            let declared_enclosing = if is_global_function_call {
                "Quartz_Global".to_string()
            } else {
                let receiver = receiver_trail.last().unwrap();
                _ctx.environment
                    .get_expression_type(
                        receiver,
                        &enclosing_type,
                        &[],
                        caller_protections,
                        _ctx.scope_context.as_ref().unwrap_or_default(),
                    )
                    .name()
            };

            if _ctx.environment.is_struct_declared(&declared_enclosing)
                || _ctx.environment.is_contract_declared(&declared_enclosing)
                || _ctx.environment.is_trait_declared(&declared_enclosing)
                || _ctx.environment.is_asset_declared(&declared_enclosing)
                    && !is_global_function_call
            {
                let mut expression = construct_expression(&receiver_trail);
                let enclosing_type = _ctx
                    .enclosing_type_identifier()
                    .map(|id| id.token.to_string())
                    .unwrap_or_default();

                if expression.enclosing_type().is_some() {
                    expression = expand_properties(expression, _ctx, false);
                } else if let Expression::BinaryExpression(_) = expression {
                    expression = expand_properties(expression, _ctx, false);
                }

                let result_type = match expression {
                    Expression::Identifier(ref i) => {
                        if let Some(ref result) = _ctx
                            .scope_context
                            .as_ref()
                            .unwrap_or_default()
                            .type_for(&i.token)
                        {
                            result.clone()
                        } else {
                            _ctx.environment.get_expression_type(
                                &expression,
                                &enclosing_type,
                                &[],
                                &[],
                                _ctx.scope_context.as_ref().unwrap_or_default(),
                            )
                        }
                    }
                    _ => _ctx.environment.get_expression_type(
                        &expression,
                        &enclosing_type,
                        &[],
                        &[],
                        _ctx.scope_context.as_ref().unwrap_or_default(),
                    ),
                };

                if !result_type.is_inout_type() {
                    let inout = InoutExpression {
                        ampersand_token: "".to_string(),
                        expression: Box::new(expression.clone()),
                    };
                    expression = Expression::InoutExpression(inout)
                }

                let mut arguments = call.arguments.clone();
                arguments.insert(
                    0,
                    FunctionArgument {
                        identifier: None,
                        expression,
                    },
                );

                call.arguments = arguments;
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
        let enclosing = _ctx.enclosing_type_identifier().unwrap().token.to_string();
        let receiver = &*_t.function_call.lhs_expression;
        let receiver_type = _ctx.environment.get_expression_type(
            &receiver,
            &enclosing,
            &[],
            &[],
            _ctx.scope_context.as_ref().unwrap(),
        );
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
                    let caller_protections: &[CallerProtection] =
                        if let Some(ref behaviour) = _ctx.contract_behaviour_declaration_context {
                            &behaviour.caller_protections
                        } else {
                            &[]
                        };
                    let expression_type = _ctx.environment.get_expression_type(
                        &expression,
                        enclosing,
                        &[],
                        caller_protections,
                        &scope,
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

            let state_variable = if context.special_declaration_context.is_some() {
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

fn generate_and_add_getter(
    member: &Property,
    behaviour_declaration: &mut ContractBehaviourDeclaration,
    ctx: &mut Context,
) {
    let mut member_identifier = member.get_identifier();
    member_identifier.enclosing_type = Some(behaviour_declaration.identifier.token.clone());

    // converts the name to start with a capital, so value becomes getValue
    let getter_name = format!(
        "get{}{}",
        member_identifier
            .token
            .chars()
            .next()
            .unwrap()
            .to_ascii_uppercase(),
        member_identifier.token.chars().skip(1).collect::<String>()
    );

    let member_type = member.get_type();

    let return_statement = Statement::ReturnStatement(ReturnStatement {
        expression: Some(Expression::BinaryExpression(BinaryExpression {
            // TODO the self part of this does not get a copy or move because function context
            // does not declare it an in out type
            lhs_expression: Box::new(Expression::SelfExpression),
            rhs_expression: Box::new(Expression::Identifier(member_identifier)),
            op: BinOp::Dot,
            line_info: Default::default(),
        })),
        cleanup: vec![],
        line_info: Default::default(),
    });

    let getter_signature = FunctionSignatureDeclaration {
        func_token: "func".to_string(),
        attributes: vec![],
        modifiers: vec![Modifier::Public],
        mutates: vec![],
        identifier: Identifier::generated(&getter_name),
        parameters: vec![],
        result_type: Some(member_type),
        payable: false,
    };

    let getter = FunctionDeclaration {
        head: getter_signature,
        body: vec![return_statement],
        scope_context: Some(Default::default()),
        tags: vec![],
        mangled_identifier: None,
        is_external: false,
    };

    behaviour_declaration
        .members
        .push(ContractBehaviourMember::FunctionDeclaration(getter.clone()));

    ctx.environment.add_function(
        getter,
        &behaviour_declaration.identifier.token,
        vec![], // These should be empty anyway as we should only make getters and setters
        vec![], // In restriction free zones
    );
}

fn generate_and_add_setter(
    member: &Property,
    behaviour_declaration: &mut ContractBehaviourDeclaration,
) {
    let member_identifier = member.get_identifier();

    // converts the name to start with a capital, so value becomes setValue
    let setter_name = format!(
        "set{}{}",
        member_identifier
            .token
            .chars()
            .next()
            .unwrap()
            .to_ascii_uppercase(),
        member_identifier.token.chars().skip(1).collect::<String>()
    );

    let parameter_identifier = Identifier::generated(member_identifier.token.as_str());
    let parameter = Parameter {
        identifier: parameter_identifier.clone(),
        type_assignment: member.get_type(),
        expression: None,
        line_info: Default::default(),
    };

    let assignment = BinaryExpression {
        lhs_expression: Box::new(Expression::BinaryExpression(BinaryExpression {
            lhs_expression: Box::new(Expression::SelfExpression),
            rhs_expression: Box::new(Expression::Identifier(member_identifier.clone())),
            op: BinOp::Dot,
            line_info: Default::default(),
        })),
        rhs_expression: Box::new(Expression::Identifier(parameter_identifier)),
        op: BinOp::Equal,
        line_info: Default::default(),
    };

    let assignment = Statement::Expression(Expression::BinaryExpression(assignment));

    let setter_signature = FunctionSignatureDeclaration {
        func_token: "func".to_string(),
        attributes: vec![],
        modifiers: vec![Modifier::Public],
        mutates: vec![member_identifier],
        identifier: Identifier::generated(&setter_name),
        parameters: vec![parameter],
        result_type: None,
        payable: false,
    };

    let setter_declaration = FunctionDeclaration {
        head: setter_signature,
        body: vec![assignment],
        scope_context: Some(Default::default()),
        tags: vec![],
        mangled_identifier: None,
        is_external: false,
    };

    behaviour_declaration
        .members
        .push(ContractBehaviourMember::FunctionDeclaration(
            setter_declaration,
        ));
}
