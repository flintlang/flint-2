use crate::ast::{mangle, mangle_function_move, Assertion, BinOp, BinaryExpression, CallerProtection, ContractBehaviourDeclaration, Expression, FunctionArgument, FunctionCall, FunctionDeclaration, Identifier, InoutExpression, InoutType, Parameter, ReturnStatement, Statement, Type, VariableDeclaration, FixedSizedArrayType, ArrayType};
use crate::context::{Context, ScopeContext};
use crate::environment::{CallableInformation, Environment, FunctionCallMatchResult};
use crate::moveir::preprocessor::get_mutable_reference;
use crate::type_checker::ExpressionChecker;
use crate::utils::type_states::*;
use itertools::Itertools;

pub fn convert_default_parameter_functions(
    base: FunctionDeclaration,
    type_id: &str,
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

            if assigned_function.scope_context.is_some() {
                let scope = ScopeContext {
                    parameters: assigned_function.head.parameters.clone(),
                    local_variables: vec![],
                    ..Default::default()
                };
                assigned_function.scope_context = Some(scope);
            }

            _ctx.environment.remove_function(f, type_id);

            let (protections, type_states) =
                if let Some(ref context) = _ctx.contract_behaviour_declaration_context {
                    (
                        context.caller_protections.clone(),
                        context.type_states.clone(),
                    )
                } else {
                    (vec![], vec![])
                };
            _ctx.environment.add_function(
                removed.clone(),
                type_id,
                protections.clone(),
                type_states.clone(),
            );

            processed.push(removed);

            let arguments: Vec<FunctionArgument> = f
                .head
                .parameters
                .clone()
                .into_iter()
                .map(|p| {
                    if p.identifier.token == parameter.identifier.token {
                        let mut expression = parameter.expression.as_ref().unwrap().clone();
                        expression.assign_enclosing_type(type_id);
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

            if assigned_function.head.result_type.is_some() {
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

            _ctx.environment.add_function(
                assigned_function.clone(),
                type_id,
                protections,
                type_states,
            );

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
    wrapper.mangled_identifier = Option::from(mangle_function_move(
        &function.head.identifier.token,
        &"".to_string(),
        true,
    ));

    wrapper.body = vec![];
    wrapper.tags.push("acquires T".to_string());

    if !function.is_void() && !function.body.is_empty() {
        let mut func = function.clone();
        wrapper.body.push(func.body.remove(0));
    }

    let contract_address_parameter = Parameter {
        identifier: Identifier::generated("address_this"),
        type_assignment: Type::Address,
        expression: None,
        line_info: Default::default(),
    };

    let original_parameter = wrapper.head.parameters.remove(0);

    wrapper
        .head
        .parameters
        .insert(0, contract_address_parameter);
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
    let caller_protections: Vec<CallerProtection> = contract_behaviour_declaration
        .caller_protections
        .clone()
        .into_iter()
        .filter(|c| c.is_any())
        .collect();

    let (state_properties, protection_functions) =
        split_caller_protections(&contract_behaviour_declaration, &context);

    if !contract_behaviour_declaration.caller_protections.is_empty()
        && caller_protections.is_empty()
        && !protection_functions.is_empty()
    {
        let caller_id: Identifier;

        if let Some(caller) = &contract_behaviour_declaration.caller_binding {
            caller_id = caller.clone();
        } else {
            caller_id = Identifier::generated("caller");
        }

        if let Some(predicate) = generate_caller_protections_predicate(
            &protection_functions,
            &caller_id.token,
            &contract_behaviour_declaration.identifier,
            &wrapper.head.identifier.token,
            &context,
        ) {
            let assertion = Assertion {
                expression: predicate,
                line_info: contract_behaviour_declaration.identifier.line_info.clone(),
            };

            wrapper.body.push(Statement::Assertion(assertion));
        }
    }
    let is_stateful = !contract_behaviour_declaration.type_states.is_empty();
    if is_stateful {
        let type_state_declaration = VariableDeclaration {
            declaration_token: None,
            identifier: Identifier::generated(Identifier::TYPESTATE_VAR_NAME),
            variable_type: Type::TypeState,
            expression: None,
        };

        wrapper
            .body
            .push(Statement::Expression(Expression::VariableDeclaration(
                type_state_declaration,
            )));
    }

    let self_assignment = BinaryExpression {
        lhs_expression: Box::new(Expression::SelfExpression),
        rhs_expression: Box::new(Expression::RawAssembly(
            "borrow_global_mut<T>(copy(address_this))".to_string(),
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

    if !contract_behaviour_declaration.caller_protections.is_empty()
        && caller_protections.is_empty()
    {
        let caller_id: Identifier;

        if let Some(caller) = &contract_behaviour_declaration.caller_binding {
            caller_id = caller.clone();
        } else {
            caller_id = Identifier::generated("caller");
        }

        if let Some(predicate) = generate_caller_protections_predicate(
            &state_properties,
            &caller_id.token,
            &contract_behaviour_declaration.identifier,
            &wrapper.head.identifier.token,
            &context,
        ) {
            let assertion = Assertion {
                expression: predicate,
                line_info: contract_behaviour_declaration.identifier.line_info.clone(),
            };

            wrapper.body.push(Statement::Assertion(assertion));
        }
    }

    if is_stateful {
        // TODO we need the slice [1..] because an extra _ is added to the front for some reason
        // This should be fixed
        let state_identifier = Identifier::generated(&Identifier::TYPESTATE_VAR_NAME[1..]);
        let type_state_assignment = BinaryExpression {
            lhs_expression: Box::new(Expression::Identifier(state_identifier.clone())),
            rhs_expression: Box::new(Expression::BinaryExpression(BinaryExpression {
                lhs_expression: Box::new(Expression::SelfExpression),
                rhs_expression: Box::new(Expression::Identifier(Identifier::generated(
                    Identifier::TYPESTATE_VAR_NAME,
                ))),
                op: BinOp::Dot,
                line_info: Default::default(),
            })),
            op: BinOp::Equal,
            line_info: Default::default(),
        };

        wrapper
            .body
            .push(Statement::Expression(Expression::BinaryExpression(
                type_state_assignment,
            )));

        let contract_name = &contract_behaviour_declaration.identifier.token;
        let allowed_type_states_as_u8s = extract_allowed_states(
            &contract_behaviour_declaration.type_states,
            &context.environment.get_contract_type_states(contract_name),
        )
        .collect::<Vec<u8>>();
        let condition =
            generate_type_state_condition(state_identifier, &allowed_type_states_as_u8s);
        let assertion = Assertion {
            expression: Expression::BinaryExpression(condition),
            line_info: contract_behaviour_declaration.identifier.line_info.clone(),
        };

        wrapper.body.push(Statement::Assertion(assertion));
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
            if let Some(ref scope_context) = ctx.scope_context {
                if let Some(identifier_type) = scope_context.type_for(&i.token) {
                    if !identifier_type.is_inout_type() {
                        return pre_assign(expression, ctx, borrow, false);
                    }
                }

                if i.enclosing_type.is_some() {
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

fn cmp_expressions(first: &Expression, second: &Expression) -> bool {
    // compares expressions ignoring line_info
    match first {
        Expression::SelfExpression => {
            if let Expression::SelfExpression = second {
                return true;
            }
        }
        Expression::Identifier(e1) => {
            if let Expression::Identifier(e2) = second {
                return e1.token == e2.token && e1.enclosing_type == e2.enclosing_type;
            }
        }
        Expression::InoutExpression(e1) => {
            if let Expression::InoutExpression(e2) = second {
                return cmp_expressions(&e1.expression, &e2.expression);
            }
        }
        Expression::BinaryExpression(e1) => {
            if let Expression::BinaryExpression(e2) = second {
                return cmp_expressions(&e1.lhs_expression, &e2.lhs_expression)
                    && cmp_expressions(&e1.rhs_expression, &e2.rhs_expression)
                    && e1.op == e2.op;
            }
        }
        Expression::BracketedExpression(e1) => {
            if let Expression::BracketedExpression(e2) = second {
                return cmp_expressions(&e1.expression, &e2.expression);
            }
        }
        Expression::CastExpression(e1) => {
            if let Expression::CastExpression(e2) = second {
                return e1.cast_type == e2.cast_type
                    && cmp_expressions(&e1.expression, &e2.expression);
            }
        }
        Expression::SubscriptExpression(e1) => {
            if let Expression::SubscriptExpression(e2) = second {
                return e1.base_expression == e2.base_expression
                    && cmp_expressions(&e1.index_expression, &e2.index_expression);
            }
        }
        Expression::AttemptExpression(e1) => {
            if let Expression::AttemptExpression(e2) = second {
                return e1.kind == e2.kind
                    && cmp_expressions(
                        &Expression::FunctionCall(e1.function_call.clone()),
                        &Expression::FunctionCall(e2.function_call.clone()),
                    );
            }
        }
        Expression::RangeExpression(e1) => {
            if let Expression::RangeExpression(e2) = second {
                return e1.op == e2.op
                    && cmp_expressions(&e1.start_expression, &e2.start_expression)
                    && cmp_expressions(&e1.end_expression, &e2.end_expression);
            }
        }
        Expression::VariableDeclaration(v1) => {
            if let Expression::VariableDeclaration(v2) = second {
                if v1.declaration_token == v2.declaration_token
                    && v1.identifier == v2.identifier
                    && v1.variable_type == v2.variable_type
                {
                    if let Some(e1) = &v1.expression {
                        if let Some(e2) = &v2.expression {
                            return cmp_expressions(&e1, &e2);
                        }
                    }
                }
            }
        }
        Expression::ExternalCall(e1) => {
            if let Expression::ExternalCall(e2) = second {
                return e1.arguments == e2.arguments
                    && e1.external_trait_name == e2.external_trait_name
                    && cmp_expressions(
                        &Expression::BinaryExpression(e1.function_call.clone()),
                        &Expression::BinaryExpression(e2.function_call.clone()),
                    );
            }
        }
        Expression::ArrayLiteral(first_exprs) => {
            if let Expression::ArrayLiteral(second_exprs) = second {
                let mut found = false;
                for expr in &first_exprs.elements {
                    for other_expr in &second_exprs.elements {
                        found |= cmp_expressions(&expr, &other_expr);
                    }
                    if !found {
                        return false;
                    }
                    found = false;
                }
                return true;
            }
        }
        Expression::Sequence(first_exprs) => {
            if let Expression::Sequence(second_exprs) = second {
                let mut found = false;
                for expr in first_exprs {
                    for other_expr in second_exprs {
                        found |= cmp_expressions(&expr, &other_expr);
                    }
                    if !found {
                        return false;
                    }
                    found = false;
                }
                return true;
            }
        }
        Expression::DictionaryLiteral(first_mappings) => {
            if let Expression::DictionaryLiteral(second_mappings) = second {
                let mut found = false;
                for expr in &first_mappings.elements {
                    for other_expr in &second_mappings.elements {
                        found |= cmp_expressions(&expr.0, &other_expr.0)
                            && cmp_expressions(&expr.1, &other_expr.1);
                    }
                    if !found {
                        return false;
                    }
                    found = false;
                }
                return true;
            }
        }
        _ => return first == second,
    }
    false
}

pub fn pre_assign(
    expression: Expression,
    ctx: &mut Context,
    borrow: bool,
    is_reference: bool,
) -> Expression {
    let enclosing_type = ctx.enclosing_type_identifier().unwrap().token.clone();
    let scope = ctx.scope_context.as_mut().unwrap();
    let mut expression_type =
        ctx.environment
            .get_expression_type(&expression, &enclosing_type, &[], &[], &scope);

    if expression_type.is_external_contract(ctx.environment.clone()) {
        expression_type = Type::Address
    }

    let mut expression = if borrow || !is_reference || expression_type.is_built_in_type() {
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
                    return cmp_expressions(&expression, &*b.rhs_expression);
                }
            }
            false
        })
        .collect();
    if let Some(statement) = statements.first() {
        if let Expression::Identifier(i) = &*statement.lhs_expression {
            temp_identifier = i.clone()
        }
    } else {
        temp_identifier = scope.fresh_identifier(expression.get_line_info());
        let declaration = if expression_type.is_built_in_type() || borrow {
            VariableDeclaration {
                declaration_token: None,
                identifier: temp_identifier.clone(),
                variable_type: expression_type.clone(),
                expression: None,
            }
        } else {
            VariableDeclaration {
                declaration_token: None,
                identifier: temp_identifier.clone(),
                variable_type: Type::InoutType(InoutType {
                    key_type: Box::new(expression_type.clone()),
                }),
                expression: None,
            }
        };

        if struct_is_mutable_reference(&mut expression, &temp_identifier, ctx) {
            ctx.pre_statements
                .push(Statement::Expression(Expression::BinaryExpression(
                    BinaryExpression {
                        lhs_expression: Box::new(Expression::Identifier(temp_identifier.clone())),
                        rhs_expression: Box::new(expression),
                        op: BinOp::Equal,
                        line_info: temp_identifier.line_info.clone(),
                    },
                )));
        } else {
            let mangled_identifier = Identifier::generated(&mangle(&temp_identifier.token));
            ctx.post_statements.push(release(
                Expression::Identifier(mangled_identifier),
                Type::InoutType(InoutType {
                    key_type: Box::new(expression_type),
                }),
            ));
        }

        // If is function declaration context
        if let Some(ref mut function_declaration_context) = ctx.function_declaration_context {
            let mut variable_present = false;

            for local_variable in function_declaration_context.local_variables.clone() {
                if local_variable.identifier == declaration.identifier
                    && local_variable.variable_type == declaration.variable_type
                {
                    // do not add to local variables
                    variable_present = true;
                    break;
                }
            }

            if !variable_present {
                function_declaration_context
                    .local_variables
                    .push(declaration.clone());
            }

            if let Some(ref mut scope_context) =
                function_declaration_context.declaration.scope_context
            {
                scope_context.local_variables.push(declaration);
            }
        }
        // Otherwise if special declaration context
        else if let Some(ref mut special_declaration_context) = ctx.special_declaration_context {
            special_declaration_context
                .local_variables
                .push(declaration.clone());
            special_declaration_context
                .declaration
                .scope_context
                .local_variables
                .push(declaration);
        }
    }
    if borrow {
        Expression::InoutExpression(InoutExpression {
            ampersand_token: "&".to_string(),
            expression: Box::new(Expression::Identifier(temp_identifier)),
        })
    } else {
        Expression::Identifier(temp_identifier)
    }
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

pub fn mangle_function_call_name(
    function_call: &FunctionCall,
    context: &Context,
) -> Option<String> {
    if !Environment::is_runtime_function_call(function_call) && !context.is_external_function_call {
        let enclosing_type = if let Some(ref enclosing) = function_call.identifier.enclosing_type {
            enclosing.clone()
        } else {
            context.enclosing_type_identifier().unwrap().token.clone()
        };

        let caller_protections: &[CallerProtection] =
            if let Some(ref behaviour) = context.contract_behaviour_declaration_context {
                &behaviour.caller_protections
            } else {
                &[]
            };

        let scope = &context.scope_or_default();

        let match_result = context.environment.match_function_call(
            &function_call,
            &enclosing_type,
            caller_protections,
            scope,
        );

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
                if c.candidates.len() > 1 {
                    panic!(
                        "Found too many function declarations! ({} found)",
                        c.candidates.len()
                    )
                }

                let candidate = c
                    .candidates
                    .first()
                    .expect("Unable to find function declaration");

                if let CallableInformation::FunctionInformation(fi) = candidate {
                    let declaration = &fi.declaration;
                    let param_types = &declaration.head.parameters;
                    let _param_types: Vec<&Type> =
                        param_types.iter().map(|p| &p.type_assignment).collect();

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
        let _lol2 = !context.is_external_function_call;
        Some(function_call.identifier.token.clone())
    }
}

pub fn is_global_function_call(function_call: &FunctionCall, ctx: &Context) -> bool {
    let enclosing = ctx.enclosing_type_identifier().unwrap().token.clone();
    let caller_protections: &[CallerProtection] =
        if let Some(ref behaviour) = ctx.contract_behaviour_declaration_context {
            &behaviour.caller_protections
        } else {
            &[]
        };

    let scope = ctx.scope_or_default();

    let result =
        ctx.environment
            .match_function_call(&function_call, &enclosing, caller_protections, scope);

    if let FunctionCallMatchResult::MatchedGlobalFunction(_) = result {
        return true;
    }

    false
}

pub fn construct_expression(expressions: &[Expression]) -> Expression {
    match expressions.first() {
        Some(first) if expressions.len() > 1 => Expression::BinaryExpression(BinaryExpression {
            lhs_expression: Box::new(first.clone()),
            rhs_expression: Box::new(construct_expression(&expressions[1..])),
            op: BinOp::Dot,
            line_info: Default::default(),
        }),
        Some(first) => first.clone(),
        None => panic!("Cannot construct expression from no expressions"),
    }
}

fn struct_is_mutable_reference(
    mut expression: &mut Expression,
    temp_identifier: &Identifier,
    ctx: &mut Context,
) -> bool {
    if let Expression::BinaryExpression(be) = &mut expression {
        if let BinOp::Dot = be.op {
            let id = get_mutable_reference(&be, ctx);

            if let Some(id) = id {
                ctx.pre_statements
                    .push(Statement::Expression(Expression::BinaryExpression(
                        BinaryExpression {
                            lhs_expression: Box::new(Expression::Identifier(
                                temp_identifier.clone(),
                            )),
                            rhs_expression: Box::new(Expression::BinaryExpression(
                                BinaryExpression {
                                    lhs_expression: Box::new(Expression::Identifier(id)),
                                    rhs_expression: be.rhs_expression.clone(),
                                    op: BinOp::Dot,
                                    line_info: be.line_info.clone(),
                                },
                            )),
                            op: BinOp::Equal,
                            line_info: temp_identifier.line_info.clone(),
                        },
                    )));

                return false;
            }
        }
    }
    true
}

fn split_caller_protections(
    contract_behaviour_declaration: &ContractBehaviourDeclaration,
    context: &Context,
) -> (Vec<CallerProtection>, Vec<CallerProtection>) {
    let mut state_properties = vec![];
    let mut functions = vec![];

    for caller_protection in &contract_behaviour_declaration.caller_protections {
        let mut ident = caller_protection.clone().identifier;
        ident.enclosing_type =
            Option::from(contract_behaviour_declaration.identifier.token.clone());
        let en_ident = contract_behaviour_declaration.identifier.clone();
        let c_type = context.environment.get_expression_type(
            &Expression::Identifier(ident.clone()),
            &en_ident.token,
            &[],
            &[],
            &ScopeContext {
                parameters: vec![],
                local_variables: vec![],
                counter: 0,
            },
        );

        match c_type {
            Type::Address => state_properties.push(caller_protection.clone()),
            Type::ArrayType(_) => state_properties.push(caller_protection.clone()),
            Type::DictionaryType(_) => state_properties.push(caller_protection.clone()),
            _ => functions.push(caller_protection.clone()),
        }
    }

    (state_properties, functions)
}

pub fn generate_caller_protections_predicate(
    caller_protections: &[CallerProtection],
    caller_id: &str,
    en_ident: &Identifier,
    function_name: &str,
    context: &Context,
) -> Option<Expression> {
    caller_protections
        .iter()
        .cloned()
        .filter_map(|c| {
            let mut ident = c.clone().identifier;
            ident.enclosing_type = Option::from(en_ident.token.clone());
            let en_ident = en_ident.token.clone();
            let c_type = context.environment.get_expression_type(
                &Expression::Identifier(ident.clone()),
                &en_ident,
                &[],
                &[],
                &ScopeContext {
                    parameters: vec![],
                    local_variables: vec![],
                    counter: 0,
                },
            );

            match c_type {
                Type::Address => Some(Expression::BinaryExpression(BinaryExpression {
                    lhs_expression: Box::new(Expression::Identifier(ident)),
                    rhs_expression: Box::new(Expression::RawAssembly(
                        format!("Signer.address_of(copy({}))", caller_id),
                        None,
                    )),
                    op: BinOp::DoubleEqual,
                    line_info: Default::default(),
                })),
                Type::FixedSizedArrayType(_) | Type::ArrayType(_) => {
                    let array_type = match c_type {
                        Type::FixedSizedArrayType(FixedSizedArrayType { key_type, .. }) => *key_type,
                        Type::ArrayType(ArrayType { key_type }) => *key_type,
                        _ => panic!(),
                    };

                    assert_eq!(
                        array_type,
                        Type::Address,
                        "Array values for caller protection must have type Address"
                    );
                    if let Some(property) = context.environment.get_caller_protection(&c) {
                        if let Some(Expression::ArrayLiteral(array)) = property.property.get_value()
                        {
                            let predicate = array
                                .elements
                                .iter()
                                .cloned()
                                .map(|c| {
                                    Expression::BinaryExpression(BinaryExpression {
                                        lhs_expression: Box::new(c),
                                        rhs_expression: Box::new(Expression::RawAssembly(
                                            format!("Signer.address_of(copy({}))", caller_id),
                                            None,
                                        )),
                                        op: BinOp::DoubleEqual,
                                        line_info: Default::default(),
                                    })
                                })
                                .fold1(|left, right| {
                                    Expression::BinaryExpression(BinaryExpression {
                                        lhs_expression: Box::new(left),
                                        rhs_expression: Box::new(right),
                                        op: BinOp::Or,
                                        line_info: Default::default(),
                                    })
                                })
                                .unwrap();
                            Some(predicate)
                        } else {
                            panic!("Mismatching types for {:?}", c)
                        }
                    } else {
                        panic!("{:?} not found in caller protections", c)
                    }
                }
                Type::DictionaryType(dict_type) => {
                    assert_eq!(
                        *dict_type.value_type,
                        Type::Address,
                        "Dictionary values for caller protection must have type Address"
                    );
                    if let Some(property) = context.environment.get_caller_protection(&c) {
                        if let Some(Expression::DictionaryLiteral(dict)) =
                            property.property.get_value()
                        {
                            let predicate = dict
                                .elements
                                .iter()
                                .cloned()
                                .map(|(_, v)| {
                                    Expression::BinaryExpression(BinaryExpression {
                                        lhs_expression: Box::new(v),
                                        rhs_expression: Box::new(Expression::RawAssembly(
                                            format!("Signer.address_of(copy({}))", caller_id),
                                            None,
                                        )),
                                        op: BinOp::DoubleEqual,
                                        line_info: Default::default(),
                                    })
                                })
                                .fold1(|left, right| {
                                    Expression::BinaryExpression(BinaryExpression {
                                        lhs_expression: Box::new(left),
                                        rhs_expression: Box::new(right),
                                        op: BinOp::Or,
                                        line_info: Default::default(),
                                    })
                                })
                                .unwrap();
                            Some(predicate)
                        } else {
                            panic!("Mismatching types for {:?}", c)
                        }
                    } else {
                        panic!("{:?} not found in caller protections", c)
                    }
                }
                _ => {
                    let enclosing_type = ident.enclosing_type.as_deref().unwrap_or(&en_ident);
                    if let Some(types) = context.environment.types.get(enclosing_type) {
                        if let Some(function_info) = types.functions.get(&ident.token) {
                            if let Some(function) = function_info.get(0) {
                                let function_signature = &function.declaration.head;
                                if function_signature.is_predicate() {
                                    // caller protection is a predicate function
                                    return if ident.token != function_name {
                                        // prevents predicate being added to the predicate function itself
                                        Some(Expression::FunctionCall(FunctionCall {
                                            identifier: ident,
                                            arguments: vec![
                                                FunctionArgument {
                                                    identifier: None,
                                                    expression: Expression::Identifier(
                                                        Identifier {
                                                            token: "address_this".to_string(),
                                                            enclosing_type: None,
                                                            line_info: Default::default(),
                                                        },
                                                    ),
                                                },
                                                FunctionArgument {
                                                    identifier: None,
                                                    expression: Expression::RawAssembly(
                                                        format!(
                                                            "Signer.address_of(copy({}))",
                                                            caller_id
                                                        ),
                                                        None,
                                                    ),
                                                },
                                            ],
                                            mangled_identifier: None,
                                        }))
                                    } else {
                                        None
                                    };
                                } else if function_signature.is_0_ary_function() {
                                    // caller protection is a 0-ary function
                                    return if ident.token != function_name {
                                        // prevents 0-ary function being added to the 0-ary function itself
                                        Some(Expression::BinaryExpression(BinaryExpression {
                                            lhs_expression: Box::new(Expression::FunctionCall(
                                                FunctionCall {
                                                    identifier: ident,
                                                    arguments: vec![FunctionArgument {
                                                        identifier: None,
                                                        expression: Expression::Identifier(
                                                            Identifier {
                                                                token: "address_this".to_string(),
                                                                enclosing_type: None,
                                                                line_info: Default::default(),
                                                            },
                                                        ),
                                                    }],
                                                    mangled_identifier: None,
                                                },
                                            )),
                                            rhs_expression: Box::new(Expression::RawAssembly(
                                                format!("Signer.address_of(copy({}))", caller_id),
                                                None,
                                            )),
                                            op: BinOp::DoubleEqual,
                                            line_info: Default::default(),
                                        }))
                                    } else {
                                        None
                                    };
                                }
                            }
                        }
                    }
                    panic!(
                        "Invalid caller protection \"{}\" at line {}",
                        ident.token, ident.line_info.line
                    )
                }
            }
        })
        .fold1(|left, right| {
            Expression::BinaryExpression(BinaryExpression {
                lhs_expression: Box::new(left),
                rhs_expression: Box::new(right),
                op: BinOp::Or,
                line_info: Default::default(),
            })
        })
}
