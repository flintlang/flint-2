use crate::ast::{
    mangle, mangle_function_move, BinOp, BinaryExpression, CallerProtection,
    ContractBehaviourDeclaration, Expression, FunctionArgument, FunctionCall, FunctionDeclaration,
    Identifier, InoutExpression, InoutType, Parameter, ReturnStatement, Statement, Type,
    TypeIdentifier, VariableDeclaration,
};
use crate::context::{Context, ScopeContext};
use crate::environment::{CallableInformation, Environment, FunctionCallMatchResult};
use crate::moveir::expression::MoveExpression;
use crate::moveir::function::FunctionContext;
use crate::moveir::ir::MoveIRBlock;
use crate::type_checker::ExpressionCheck;

pub fn convert_default_parameter_functions(
    base: FunctionDeclaration,
    t: &TypeIdentifier,
    _ctx: &mut Context,
) -> Vec<FunctionDeclaration> {
    let default_parameters: Vec<Parameter> = base
        .clone()
        .head
        .parameters
        .into_iter()
        .filter(|p| p.expression.is_some())
        .rev()
        .collect();
    let mut functions = vec![base];

    for parameter in default_parameters {
        let mut processed = Vec::new();
        for f in &functions {
            let mut assigned_function = f.clone();
            let mut removed = f.clone();

            assigned_function.head.parameters = assigned_function
                .head
                .parameters
                .into_iter()
                .filter(|p| p.identifier.token != f.head.identifier.token)
                .collect();

            removed.head.parameters = removed
                .head
                .parameters
                .into_iter()
                .map(|p| {
                    if p.identifier.token == parameter.identifier.token {
                        let mut param = p;
                        param.expression = None;
                        param
                    } else {
                        p
                    }
                })
                .collect();

            if assigned_function.scope_context.is_some() {//REMOVEBEFOREFLIGHT
                let scope = ScopeContext {
                    parameters: assigned_function.head.parameters.clone(),
                    local_variables: vec![],
                    ..Default::default()
                };
                assigned_function.scope_context = Some(scope);
            }

            _ctx.environment.remove_function(f, t);

            let protections = if let Some(ref context) = _ctx.contract_behaviour_declaration_context
            {
                context.caller_protections.clone()
            } else {
                vec![]
            };
            _ctx.environment
                .add_function(&removed, t, protections.clone());

            processed.push(removed);

            let arguments: Vec<FunctionArgument> = f
                .head
                .parameters
                .clone()
                .into_iter()
                .map(|p| {
                    if p.identifier.token == parameter.identifier.token {
                        let mut expression = parameter.expression.as_ref().unwrap().clone();
                        expression.assign_enclosing_type(t);
                        FunctionArgument {
                            identifier: Some(p.identifier),
                            expression,
                        }
                    } else {
                        FunctionArgument {
                            identifier: Some(p.identifier.clone()),
                            expression: Expression::Identifier(p.identifier),
                        }
                    }
                })
                .collect();

            if assigned_function.head.result_type.is_some() {//REMOVEBEFOREFLIGHT
                let function_call = FunctionCall {
                    identifier: f.head.identifier.clone(),
                    arguments,
                    mangled_identifier: None,
                };
                let return_statement = ReturnStatement {
                    expression: Option::from(Expression::FunctionCall(function_call)),
                    cleanup: vec![],
                    line_info: parameter.line_info.clone(),
                };
                let return_statement = Statement::ReturnStatement(return_statement);
                assigned_function.body = vec![return_statement];
            } else {
                let function_call = FunctionCall {
                    identifier: f.head.identifier.clone(),
                    arguments,
                    mangled_identifier: None,
                };
                let function_call = Statement::Expression(Expression::FunctionCall(function_call));
                assigned_function.body = vec![function_call];
            }

            _ctx.environment
                .add_function(&assigned_function, t, protections);

            processed.push(assigned_function);
        }
        functions = processed.clone();
    }
    functions
}

pub fn get_declaration(ctx: &mut Context) -> Vec<Statement> {
    if let Some(ref scope) = ctx.scope_context {
        let declarations = scope
            .local_variables
            .clone()
            .into_iter()
            .map(|v| {
                let mut declaration = v;
                if !declaration.identifier.is_self() {
                    declaration.identifier = Identifier {
                        token: mangle(&declaration.identifier.token),
                        enclosing_type: None,
                        line_info: Default::default(),
                    };
                }
                Statement::Expression(Expression::VariableDeclaration(declaration))
            })
            .collect();
        return declarations;
    }
    return vec![];
}

pub fn delete_declarations(statements: Vec<Statement>) -> Vec<Statement> {
    statements
        .into_iter()
        .filter(|s| {
            if let Statement::Expression(e) = s.clone() {
                if let Expression::VariableDeclaration(_) = e {
                    return false;
                }
            }
            true
        })
        .collect()
}

pub fn generate_contract_wrapper(
    function: FunctionDeclaration,
    contract_behaviour_declaration: &ContractBehaviourDeclaration,
    context: &mut Context,
) -> FunctionDeclaration {
    let mut wrapper = function.clone();
    wrapper.mangled_identifier = Option::from(mangle_function_move(&function.head.identifier.token, &"".to_string(), true));

    wrapper.body = vec![];
    wrapper.tags.push("acquires T".to_string());

    if !function.is_void() && !function.body.is_empty() {
        let mut func = function.clone();
        wrapper.body.push(func.body.remove(0));
    }

    let contract_address_parameter = Parameter {
        identifier: Identifier::generated("_address_this"),
        type_assignment: Type::Address,
        expression: None,
        line_info: Default::default(),
    };

    let original_parameter = wrapper.head.parameters.remove(0);

    wrapper
        .head
        .parameters
        .insert(0, contract_address_parameter.clone());
    let original_parameter = original_parameter;

    let self_declaration = VariableDeclaration {
        declaration_token: None,
        identifier: Identifier::generated(Identifier::SELF),
        variable_type: original_parameter.type_assignment.clone(),
        expression: None,
    };
    wrapper
        .body
        .push(Statement::Expression(Expression::VariableDeclaration(
            self_declaration,
        )));

    let sender_declaration = Expression::RawAssembly("let _sender: address".to_string(), None);
    wrapper.body.push(Statement::Expression(sender_declaration));

    let sender_assignment = BinaryExpression {
        lhs_expression: Box::new(Expression::Identifier(Identifier {
            token: "sender".to_string(),
            enclosing_type: None,
            line_info: Default::default(),
        })),
        rhs_expression: Box::new(Expression::RawAssembly(
            format!(
                "Signer.address_of(move({param}))",
                param = mangle(&contract_address_parameter.identifier.token)
            ),
            None,
        )),
        op: BinOp::Equal,
        line_info: Default::default(),
    };

    wrapper
        .body
        .push(Statement::Expression(Expression::BinaryExpression(
            sender_assignment,
        )));

    let self_assignment = BinaryExpression {
        lhs_expression: Box::new(Expression::SelfExpression),
        rhs_expression: Box::new(Expression::RawAssembly(
            "borrow_global_mut<T>(move(_sender))".to_string(),
            Some(original_parameter.type_assignment),
        )),
        op: BinOp::Equal,
        line_info: Default::default(),
    };
    wrapper
        .body
        .push(Statement::Expression(Expression::BinaryExpression(
            self_assignment,
        )));

    let caller_protections: Vec<CallerProtection> = contract_behaviour_declaration
        .caller_protections
        .clone()
        .into_iter()
        .filter(|c| c.is_any())
        .collect();
    if !contract_behaviour_declaration.caller_protections.is_empty()
        && caller_protections.is_empty()
    {
        let caller = Identifier::generated("_caller");

        wrapper.body.insert(
            0,
            Statement::Expression(Expression::VariableDeclaration(VariableDeclaration {
                declaration_token: None,
                identifier: Identifier {
                    token: mangle(&caller.token),
                    enclosing_type: None,
                    line_info: Default::default(),
                },
                variable_type: Type::Address,
                expression: None,
            })),
        );

        wrapper.body.push(generate_caller_statement(caller.clone()));

        let predicates = contract_behaviour_declaration.caller_protections.clone();

        let predicates: Vec<Expression> = predicates
            .into_iter()
            .map(|c| {
                let mut ident = c.identifier;
                ident.enclosing_type =
                    Option::from(contract_behaviour_declaration.identifier.token.clone());
                let en_ident = contract_behaviour_declaration.identifier.clone();
                let c_type = context.environment.get_expression_type(
                    Expression::Identifier(ident.clone()),
                    &en_ident.token,
                    vec![],
                    vec![],
                    ScopeContext {
                        parameters: vec![],
                        local_variables: vec![],
                        counter: 0,
                    },
                );

                match c_type {
                    Type::Address => Expression::BinaryExpression(BinaryExpression {
                        lhs_expression: Box::new(Expression::Identifier(ident)),
                        rhs_expression: Box::new(Expression::Identifier(caller.clone())),
                        op: BinOp::DoubleEqual,
                        line_info: Default::default(),
                    }),
                    _ => unimplemented!(),
                }
            })
            .collect();

        let assertion = generate_assertion(
            predicates,
            FunctionContext {
                environment: context.environment.clone(),
                scope_context: function.scope_context.clone().unwrap_or_default(),
                enclosing_type: contract_behaviour_declaration.identifier.token.clone(),
                block_stack: vec![MoveIRBlock { statements: vec![] }],
                in_struct_function: false,
                is_constructor: false,
            },
        );

        wrapper.body.push(assertion)
    }

    let arguments = function
        .head
        .parameters
        .clone()
        .into_iter()
        .map(|p| FunctionArgument {
            identifier: None,
            expression: Expression::Identifier(p.identifier),
        })
        .collect();

    let name = function.mangled_identifier.clone();
    let function_call = Expression::FunctionCall(FunctionCall {
        identifier: Identifier {
            token: name.unwrap_or_default(),
            enclosing_type: None,
            line_info: Default::default(),
        },
        arguments,
        mangled_identifier: None,
    });

    if function.is_void() {
        wrapper
            .body
            .push(Statement::Expression(function_call.clone()))
    }

    wrapper
        .body
        .push(Statement::ReturnStatement(ReturnStatement {
            expression: {
                if function.is_void() {
                    None
                } else {
                    Some(function_call)
                }
            },
            ..Default::default()
        }));

    wrapper
}

pub fn expand_properties(expression: Expression, ctx: &mut Context, borrow: bool) -> Expression {
    match expression.clone() {
        Expression::Identifier(i) => {
            if ctx.has_scope_context() {
                let scope = ctx.scope_context.clone();
                let scope = scope.unwrap();
                if let Some(identifier_type) = scope.type_for(&i.token) {
                    if !identifier_type.is_inout_type() {
                        return pre_assign(expression, ctx, borrow, false);
                    }
                }

                if i.enclosing_type.is_some() {//REMOVEBEFOREFLIGHT
                    return pre_assign(expression, ctx, borrow, true);
                }
            }
        }
        Expression::BinaryExpression(b) => {
            return if let BinOp::Dot = b.op {
                let mut binary = b.clone();
                let lhs = expand_properties(*b.lhs_expression, ctx, borrow);
                binary.lhs_expression = Box::from(lhs);
                pre_assign(Expression::BinaryExpression(binary), ctx, borrow, true)
            } else {
                let mut binary = b.clone();
                let lhs = b.lhs_expression.clone();
                let lhs = expand_properties(*lhs, ctx, borrow);
                binary.lhs_expression = Box::from(lhs);
                let rhs = b.rhs_expression.clone();
                let rhs = expand_properties(*rhs, ctx, borrow);
                binary.rhs_expression = Box::from(rhs);
                pre_assign(Expression::BinaryExpression(binary), ctx, borrow, true)
            };
        }
        _ => return expression,
    };
    expression
}

pub fn pre_assign(
    expression: Expression,
    ctx: &mut Context,
    borrow: bool,
    is_reference: bool,
) -> Expression {
    let enclosing_type = ctx.enclosing_type_identifier().unwrap();
    let scope = ctx.scope_context.clone();
    let mut scope = scope.unwrap();
    let mut expression_type = ctx.environment.get_expression_type(
        expression.clone(),
        &enclosing_type.token,
        vec![],
        vec![],
        scope.clone(),
    );

    if expression_type.is_external_contract(ctx.environment.clone()) {
        expression_type = Type::Address
    }

    let expression = if borrow || !is_reference || expression_type.is_built_in_type() {
        expression
    } else {
        Expression::InoutExpression(InoutExpression {
            ampersand_token: "".to_string(),
            expression: Box::new(expression),
        })
    };

    let mut temp_identifier = Identifier::generated("_temp_move_preassign");

    let statements: Vec<BinaryExpression> = ctx
        .pre_statements
        .clone()
        .into_iter()
        .filter_map(|s| match s {
            Statement::Expression(Expression::BinaryExpression(e)) => Some(e),
            _ => None,
        })
        .filter(|b| {
            if let BinOp::Equal = b.op {
                if let Expression::Identifier(_) = *b.lhs_expression {
                    return expression == *b.rhs_expression;
                }
            }
            false
        })
        .collect();
    if statements.is_empty() {
        // TODO temp_identifier is temp__6, so it is created here
        temp_identifier = scope.fresh_identifier(expression.get_line_info());
        let declaration = if expression_type.is_built_in_type() || borrow {
            VariableDeclaration {
                declaration_token: None,
                identifier: temp_identifier.clone(),
                variable_type: expression_type,
                expression: None,
            }
        } else {
            let var = VariableDeclaration {
                declaration_token: None,
                identifier: temp_identifier.clone(),
                variable_type: Type::InoutType(InoutType {
                    key_type: Box::new(expression_type.clone()),
                }),
                expression: None,
            };
            let mut post_statement = ctx.post_statements.clone();
            post_statement.push(release(
                Expression::Identifier(temp_identifier.clone()),
                Type::InoutType(InoutType {
                    key_type: Box::new(expression_type),
                }),
            ));
            ctx.post_statements = post_statement;
            var
        };

        let mut pre_statement = ctx.pre_statements.clone();
        pre_statement.push(Statement::Expression(Expression::BinaryExpression(
            BinaryExpression {
                lhs_expression: Box::new(Expression::Identifier(temp_identifier.clone())),
                rhs_expression: Box::new(expression),
                op: BinOp::Equal,
                line_info: temp_identifier.line_info.clone(),
            },
        )));
        ctx.pre_statements = pre_statement;

        if ctx.is_function_declaration_context() {
            let context = ctx.function_declaration_context.clone();
            let mut context = context.unwrap();
            context.local_variables.push(declaration.clone());

            if let Some(ref scope_ctx) = context.declaration.scope_context {
                let mut scope_ctx = scope_ctx.clone();
                scope_ctx.local_variables.push(declaration.clone());

                context.declaration.scope_context = Some(scope_ctx);
            }

            ctx.function_declaration_context = Option::from(context);
        }

        if ctx.is_special_declaration_context() {
            let context = ctx.special_declaration_context.clone();
            let mut context = context.unwrap();
            context.local_variables.push(declaration.clone());

            let scope_ctx = context.declaration.scope_context.clone();
            let mut scope_ctx = scope_ctx;
            scope_ctx.local_variables.push(declaration.clone());

            context.declaration.scope_context = scope_ctx;

            ctx.special_declaration_context = Option::from(context);
        }
        scope.local_variables.push(declaration);
    } else {
        let statement = statements.first();
        let statement = statement.unwrap();
        if let Expression::Identifier(i) = &*statement.lhs_expression {
            temp_identifier = i.clone()
        }
    }
    ctx.scope_context = Option::from(scope);
    if borrow {
        Expression::InoutExpression(InoutExpression {
            ampersand_token: "&".to_string(),
            expression: Box::new(Expression::Identifier(temp_identifier)),
        })
    } else {
        Expression::Identifier(temp_identifier)
    }
}

pub fn generate_caller_statement(caller: Identifier) -> Statement {
    let assignment = BinaryExpression {
        lhs_expression: Box::new(Expression::Identifier(caller.clone())),
        rhs_expression: Box::new(Expression::RawAssembly(
            "get_txn_sender()".to_string(),
            Option::from(Type::Address),
        )),
        op: BinOp::Equal,
        line_info: caller.line_info,
    };

    Statement::Expression(Expression::BinaryExpression(assignment))
}

pub fn generate_assertion(
    predicate: Vec<Expression>,
    function_context: FunctionContext,
) -> Statement {
    let mut predicates = predicate;
    if predicates.len() >= 2 {
        let or_expression = Expression::BinaryExpression(BinaryExpression {
            lhs_expression: Box::new(predicates.remove(0)),
            rhs_expression: Box::new(predicates.remove(0)),
            op: BinOp::Or,
            line_info: Default::default(),
        });
        while !predicates.is_empty() {
            unimplemented!()
        }
        let expression = MoveExpression {
            expression: or_expression,
            position: Default::default(),
        }
        .generate(&function_context);
        let string = format!("assert({ex}, 1)", ex = expression);
        return Statement::Expression(Expression::RawAssembly(string, Option::from(Type::Error)));
    }

    if predicates.is_empty() {
        unimplemented!()
    }
    let expression = predicates.remove(0);
    let expression = MoveExpression {
        expression,
        position: Default::default(),
    }
    .generate(&function_context);
    let string = format!("assert({ex}, 1)", ex = expression);
    Statement::Expression(Expression::RawAssembly(string, Option::from(Type::Error)))
}

pub fn release(expression: Expression, expression_type: Type) -> Statement {
    Statement::Expression(Expression::BinaryExpression(BinaryExpression {
        lhs_expression: Box::new(Expression::RawAssembly(
            "_".to_string(),
            Option::from(expression_type),
        )),
        rhs_expression: Box::new(expression.clone()),
        op: BinOp::Equal,
        line_info: expression.get_line_info(),
    }))
}

pub fn mangle_function_call_name(function_call: &FunctionCall, ctx: &Context) -> Option<String> {
    if !Environment::is_runtime_function_call(function_call) && !ctx.is_external_function_call {
        let enclosing_type = if let Some(ref enclosing) = function_call.identifier.enclosing_type {
            enclosing.clone()
        } else {
            let enclosing = ctx.enclosing_type_identifier().unwrap();
            enclosing.token
        };

        let call = function_call.clone();

        let caller_protections =
            if let Some(ref behaviour) = ctx.contract_behaviour_declaration_context {
                behaviour.caller_protections.clone()
            } else {
                vec![]
            };

        let scope = ctx.scope_context.clone();
        let scope = scope.unwrap_or_default();

        let match_result =
            ctx.environment
                .match_function_call(call, &enclosing_type, caller_protections, scope);

        match match_result {
            FunctionCallMatchResult::MatchedFunction(fi) => {
                let declaration = fi.declaration;
                let param_types = declaration.head.parameters;
                let _param_types: Vec<Type> =
                    param_types.into_iter().map(|p| p.type_assignment).collect();
                Some(mangle_function_move(
                    &declaration.head.identifier.token,
                    &enclosing_type,
                    false,
                ))
            }
            FunctionCallMatchResult::MatchedFunctionWithoutCaller(c) => {
                if c.candidates.len() != 1 {
                    panic!("Unable to find function declaration")
                }

                let candidate = c.candidates[0].clone();

                if let CallableInformation::FunctionInformation(fi) = candidate {
                    let declaration = fi.declaration;
                    let param_types = declaration.head.parameters;
                    let _param_types: Vec<Type> =
                        param_types.into_iter().map(|p| p.type_assignment).collect();

                    Some(mangle_function_move(
                        &declaration.head.identifier.token,
                        &enclosing_type,
                        false,
                    ))
                } else {
                    panic!("Non-function CallableInformation where function expected")
                }
            }

            FunctionCallMatchResult::MatchedInitializer(_i) => Some(mangle_function_move(
                "init",
                &function_call.identifier.token,
                false,
            )),

            FunctionCallMatchResult::MatchedFallback(_) => unimplemented!(),
            FunctionCallMatchResult::MatchedGlobalFunction(fi) => {
                let declaration = fi.declaration;

                Some(mangle_function_move(
                    &declaration.head.identifier.token,
                    &"Quartz_Global".to_string(),
                    false,
                ))
            }
            FunctionCallMatchResult::Failure(_) => None,
        }
    } else {
        let _lol = !Environment::is_runtime_function_call(function_call);
        let _lol2 = !ctx.is_external_function_call;
        Some(function_call.identifier.token.clone())
    }
}

pub fn is_global_function_call(function_call: FunctionCall, ctx: &Context) -> bool {
    let enclosing = ctx.enclosing_type_identifier().unwrap().token;
    let caller_protections = if let Some(ref behaviour) = ctx.contract_behaviour_declaration_context
    {
        behaviour.caller_protections.clone()
    } else {
        vec![]
    };

    let scope = ctx.scope_context.clone().unwrap_or_default();

    let result =
        ctx.environment
            .match_function_call(function_call, &enclosing, caller_protections, scope);

    if let FunctionCallMatchResult::MatchedGlobalFunction(_) = result {
        return true;
    }

    false
}

pub fn construct_expression(expressions: Vec<Expression>) -> Expression {
    match expressions.first() {
        Some(first) if expressions.len() > 1 => Expression::BinaryExpression(BinaryExpression {
            lhs_expression: Box::new(first.clone()),
            rhs_expression: Box::new(construct_expression(expressions[1..].to_vec())),
            op: BinOp::Dot,
            line_info: Default::default(),
        }),
        Some(first) => first.clone(),
        None => panic!("Cannot construct expression from no expressions"),
    }
}
