use super::ast::*;
use super::context::*;
use super::visitor::*;
use crate::environment::FunctionCallMatchResult::{Failure, MatchedFunction, MatchedInitializer};
use crate::environment::{Candidates, Environment};
use crate::type_checker::ExpressionChecker;
use crate::utils::unique::Unique;
use itertools::Itertools;

pub struct SemanticAnalysis {}

impl Visitor for SemanticAnalysis {
    fn start_contract_declaration(
        &mut self,
        declaration: &mut ContractDeclaration,
        context: &mut Context,
    ) -> VResult {
        if !context
            .environment
            .has_public_initialiser(&declaration.identifier.token)
        {
            return Err(Box::from(format!(
                "No public Initialiser for contract {}",
                declaration.identifier.token
            )));
        }

        if context.environment.is_conflicting(&declaration.identifier) {
            return Err(Box::from(format!(
                "Conflicting Declarations for {}",
                declaration.identifier.token
            )));
        }

        if is_conformance_repeated(&declaration.conformances) {
            return Err(Box::from("Conformances are repeated".to_owned()));
        }

        if context
            .environment
            .conflicting_trait_signatures(&declaration.identifier.token)
        {
            return Err(Box::from("Conflicting traits".to_owned()));
        }
        Ok(())
    }

    fn start_contract_behaviour_declaration(
        &mut self,
        declaration: &mut ContractBehaviourDeclaration,
        context: &mut Context,
    ) -> VResult {
        if !context
            .environment
            .is_contract_declared(&declaration.identifier.token)
        {
            return Err(Box::from(format!(
                "Undeclared contract {}",
                declaration.identifier.token
            )));
        }

        let stateful = context
            .environment
            .is_contract_stateful(&declaration.identifier.token);
        let states = declaration.type_states.clone();
        if !stateful && !states.is_empty() {
            return Err(Box::from(
                format!(
                    "Undeclared type states {:?}",
                    states
                        .iter()
                        .map(|state| state.identifier.token.clone())
                        .collect::<Vec<String>>()
                )
                .as_str(),
            ));
        }

        if !context.is_trait_declaration_context() {
            let members = declaration.members.clone();
            for member in members {
                match member {
                    ContractBehaviourMember::SpecialSignatureDeclaration(_)
                    | ContractBehaviourMember::FunctionSignatureDeclaration(_) => {
                        return Err(Box::from(format!(
                            "Signature Declaration in Contract {}",
                            declaration.identifier.token
                        )));
                    }
                    _ => continue,
                }
            }
        }

        //TODO Update the context to be contractBehaviourContext
        Ok(())
    }

    fn start_struct_declaration(
        &mut self,
        declaration: &mut StructDeclaration,
        context: &mut Context,
    ) -> VResult {
        if context.environment.is_conflicting(&declaration.identifier) {
            let i = declaration.identifier.token.clone();
            return Err(Box::from(format!("Conflicting declarations for {}", i)));
        }

        if context
            .environment
            .is_recursive_struct(&declaration.identifier.token)
        {
            return Err(Box::from(format!(
                "Recusive struct definition for {} on {}",
                declaration.identifier.token, declaration.identifier.line_info
            )));
        }

        if is_conformance_repeated(&declaration.conformances) {
            return Err(Box::from("Conformances are repeated".to_owned()));
        }

        if context
            .environment
            .conflicting_trait_signatures(&declaration.identifier.token)
        {
            return Err(Box::from("Conflicting traits".to_owned()));
        }
        Ok(())
    }

    fn finish_struct_member(
        &mut self,
        member: &mut StructMember,
        context: &mut Context,
    ) -> VResult {
        let enclosing = context.enclosing_type_identifier().unwrap();
        if let StructMember::VariableDeclaration(ref declaration, _) = member {
            if let Some(expression) = declaration.expression.as_deref() {
                let source_type = context.environment.get_expression_type(
                    expression,
                    &enclosing.token,
                    context.type_states(),
                    context.caller_protections(),
                    Default::default(),
                );
                if declaration.variable_type != source_type {
                    return Err(Box::from(format!(
                        "Cannot initialise struct field of type `{}` with type `{}` on {}",
                        declaration.variable_type, source_type, declaration.identifier.line_info
                    )));
                }
            }
        }
        Ok(())
    }

    fn start_asset_declaration(
        &mut self,
        declaration: &mut AssetDeclaration,
        context: &mut Context,
    ) -> VResult {
        if context.environment.is_conflicting(&declaration.identifier) {
            return Err(Box::from(format!(
                "Conflicting declarations for {} on line {}",
                &declaration.identifier.token, &declaration.identifier.line_info
            )));
        }

        Ok(())
    }

    fn finish_contract_member(
        &mut self,
        member: &mut ContractMember,
        context: &mut Context,
    ) -> VResult {
        let enclosing = context.enclosing_type_identifier().unwrap();
        if let ContractMember::VariableDeclaration(declaration, _) = member {
            if let Some(expression) = declaration.expression.as_deref() {
                let source_type = context.environment.get_expression_type(
                    expression,
                    &enclosing.token,
                    context.type_states(),
                    context.caller_protections(),
                    Default::default(),
                );

                if let Type::FixedSizedArrayType(FixedSizedArrayType {
                                                     key_type: lhs_type,
                                                     size,
                                                 }) = &declaration.variable_type
                {
                    // TODO check the length of the source and declaration match
                    if let Type::ArrayType(ArrayType { key_type: rhs_type }) = &source_type {
                        return if *lhs_type == *rhs_type {
                            if let Expression::ArrayLiteral(ArrayLiteral { elements }) = expression
                            {
                                if *size == elements.len() as u64 {
                                    Ok(())
                                } else {
                                    Err(Box::from(format!(
                                        "LHS array has fixed size of {} but RHS literal has size {} on line {}",
                                        size, elements.len(), declaration.identifier.line_info.line
                                    )))
                                }
                            } else {
                                Ok(())
                            }
                        } else {
                            Err(Box::from(format!(
                                "Cannot assign array of type `{}` to an array of type `{}` on {}",
                                rhs_type, lhs_type, &declaration.identifier.line_info
                            )))
                        };
                    }
                }

                if declaration.variable_type != source_type {
                    return Err(Box::from(format!(
                        "Cannot initialise contract property of type `{}` with type `{}` on {}",
                        declaration.variable_type, source_type, &declaration.identifier.line_info
                    )));
                }
            }
        }
        Ok(())
    }

    fn start_trait_declaration(
        &mut self,
        _declaration: &mut TraitDeclaration,
        _context: &mut Context,
    ) -> VResult {
        Ok(())
    }

    fn start_variable_declaration(
        &mut self,
        declaration: &mut VariableDeclaration,
        ctx: &mut Context,
    ) -> VResult {
        let type_declared = match &declaration.variable_type {
            Type::UserDefinedType(t) => ctx.environment.is_type_declared(&t.token.clone()),
            _ => true,
        };

        if !type_declared {
            return Err(Box::from(format!(
                "Type `{}` is not declared",
                declaration.variable_type
            )));
        }

        if ctx.in_function_or_special() {
            if let Some(ref mut scope_context) = ctx.scope_context {
                let redeclaration = scope_context.declaration(&declaration.identifier.token);
                if redeclaration.is_some() {
                    return Err(Box::from(format!(
                        "Redeclaration of identifier `{}` on {}",
                        declaration.identifier.token, declaration.identifier.line_info,
                    )));
                }
                scope_context.local_variables.push(declaration.clone());
            }
        } else if let Some(identifier) = ctx.enclosing_type_identifier() {
            if ctx
                .environment
                .conflicting_property_declaration(&declaration.identifier, &identifier.token)
            {
                return Err(Box::from("Conflicting property declarations".to_owned()));
            }
        }

        Ok(())
    }

    fn start_function_declaration(
        &mut self,
        declaration: &mut FunctionDeclaration,
        ctx: &mut Context,
    ) -> VResult {
        if let Some(ref identifier) = ctx.enclosing_type_identifier() {
            if ctx
                .environment
                .is_conflicting_function_declaration(&declaration, &identifier.token)
            {
                return Err(Box::from(format!(
                    "Conflicting Function Declarations for {}",
                    declaration.head.identifier.token
                )));
            }

            if identifier.token == ctx.target.currency.identifier {
                return Ok(());
            }
        }

        if !declaration
            .head
            .parameters
            .iter()
            .map(|p| &p.identifier.token)
            .is_unique()
        {
            return Err(Box::from(format!(
                "Function {} has duplicate parameters",
                declaration.head.identifier.token
            )));
        }

        let remaining_parameters = declaration
            .head
            .parameters
            .iter()
            .filter(|p| p.is_payable(&ctx.target))
            .count();
        if declaration.is_payable() {
            if remaining_parameters == 0 {
                return Err(Box::from(format!(
                    "Payable Function {} does not have payable parameter",
                    declaration.head.identifier.token
                )));
            } else if remaining_parameters > 1 {
                return Err(Box::from(format!(
                    "Payable parameter is ambiguous in function {}",
                    declaration.head.identifier.token
                )));
            }
        } else if remaining_parameters > 0 {
            return Err(Box::from(format!(
                "Function not marked payable but has payable parameter: {:?}",
                remaining_parameters
            )));
        }

        if declaration.is_public() {
            let parameters = declaration
                .head
                .parameters
                .iter()
                .filter(|p| p.is_dynamic() && !p.is_payable(&ctx.target))
                .count();
            if parameters > 0 {
                return Err(Box::from(format!(
                    "Public Function {} has dynamic parameters",
                    declaration.head.identifier.token
                )));
            }
        }

        if let Some(Type::UserDefinedType(_)) = declaration.head.result_type {
            return Err(Box::from(format!(
                "Not allowed to return struct in function {}",
                declaration.head.identifier.token
            )));
        }

        let mut return_statements = Vec::new();
        let mut become_statements = Vec::new();

        for statement in &declaration.body {
            match statement {
                Statement::ReturnStatement(ret) => return_statements.push(ret),
                Statement::BecomeStatement(bec) => become_statements.push(bec),
                _ => continue,
            }
        }

        let remaining = declaration
            .body
            .iter()
            .skip_while(|s| !is_return_or_become_statement(s));

        let remaining_after_end = remaining.filter(|s| !is_return_or_become_statement(s));
        if remaining_after_end.count() > 0 {
            return Err(Box::from(format!(
                "Statements after `return` in {}",
                declaration.head.identifier.token
            )));
        }

        if declaration.head.result_type.is_some() && !code_block_returns(&declaration.body) {
            return Err(Box::from(format!(
                "Function {} does not necessarily return",
                declaration.head.identifier.token
            )));
        }

        if return_statements.len() > 1 {
            return Err(Box::from(format!(
                "Multiple `return`s in function {}",
                declaration.head.identifier.token
            )));
        }

        if become_statements.len() > 1 {
            return Err(Box::from(format!(
                "Multiple `become` statements in {}",
                declaration.head.identifier.token
            )));
        }

        for become_statement in &become_statements {
            for return_statement in &return_statements {
                if return_statement.line_info.line
                    > become_statement.state.identifier.line_info.line
                {
                    return Err(Box::from(format!(
                        "`return` statement after `become` in function {}",
                        declaration.head.identifier.token
                    )));
                }
            }
        }

        Ok(())
    }

    fn finish_function_declaration(
        &mut self,
        declaration: &mut FunctionDeclaration,
        context: &mut Context,
    ) -> VResult {
        declaration.scope_context = context.scope_context.clone();
        Ok(())
    }

    fn start_special_declaration(
        &mut self,
        declaration: &mut SpecialDeclaration,
        ctx: &mut Context,
    ) -> VResult {
        if declaration.is_fallback() && declaration.head.has_parameters() {
            return Err(Box::from(format!(
                "Fallback {} declared with arguments",
                declaration.head.special_token
            )));
            // TODO check body only has simple statements bit long
        }

        if let Some(context) = &ctx.contract_behaviour_declaration_context {
            if !context.type_states.is_empty() {
                return Err(Box::from(
                    "Initialiser cannot have type state restrictions".to_string(),
                ));
            }

            // An initialiser in a stateful contract must have a become statement in it otherwise we
            // cannot be in any state
            if ctx
                .environment
                .is_contract_stateful(&context.identifier.token)
                && !declaration
                    .body
                    .iter()
                    .any(|state| matches!(state, Statement::BecomeStatement(_)))
            {
                return Err(Box::from(
                    "Initialiser of a contract with typestates must have a `become` statement"
                        .to_string(),
                ));
            }
        }
        Ok(())
    }

    fn start_identifier(&mut self, identifier: &mut Identifier, ctx: &mut Context) -> VResult {
        let token = &identifier.token;
        let line_number = identifier.line_info.line;

        // Check for invalid @ in name
        if token.contains('@') {
            return Err(Box::from(format!(
                "Invalid '@' character used in Identifier at line {}",
                identifier.line_info.line
            )));
        }

        if ctx.is_property_default_assignment
            && !ctx.environment.is_struct_declared(token)
            && !ctx.environment.is_asset_declared(token)
        {
            if let Some(enclosing_type) = ctx.enclosing_type_identifier() {
                return if ctx
                    .environment
                    .is_property_defined(token, &enclosing_type.token)
                {
                    // Check for property being used to define itself (I think)
                    Err(Box::from(format!(
                        "State property used within property initialiser at line {}",
                        line_number
                    )))
                } else {
                    // Check for if property is defined
                    Err(Box::from(format!(
                        "Use of undeclared identifier `{}` at line {}",
                        token, line_number
                    )))
                };
            }
        }

        // Check: If we are a function call identifier or parameter, we move on
        if ctx.is_function_call_context || ctx.is_function_call_argument_label {
            return Ok(());
        }

        if ctx.in_function_or_special() && !ctx.in_become && !ctx.in_emit {
            if let Some(enclosing_type) = &identifier.enclosing_type {
                // Previously there was a check for enclosing type not being "Quartz$ErrorType"
                // but I cannot see why so I have removed it for simplicity

                // Check
                if *enclosing_type == ctx.target.currency.identifier {
                    return Ok(());
                }

                if let Some(property) = ctx.environment.property(token, enclosing_type) {
                    // Check: Do not allow reassignment to constants: This does not work since we never know
                    // if something is assigned to yet TODO add RHS expression when something is not yet defined
                    // So we know when something has been assigned to
                    if property.is_constant()
                        && ctx.is_lvalue
                        && property.property.get_value().is_some()
                    {
                        return Err(Box::from(format!(
                            "Cannot reassign to constant `{}` on line {}",
                            token, line_number
                        )));
                    }

                    let current_enclosing_type =
                        if let Some(declaration_context) = &ctx.function_declaration_context {
                            declaration_context
                                .declaration
                                .head
                                .identifier
                                .enclosing_type
                                .clone()
                        } else if let Some(declaration_context) = &ctx.special_declaration_context {
                            declaration_context.declaration.head.enclosing_type.clone()
                        } else {
                            panic!("Should be in a special or function declaration")
                        };

                    if Some(enclosing_type) != current_enclosing_type.as_ref() {
                        match property.get_modifier() {
                            Some(Modifier::Visible) => {
                                if ctx.is_lvalue {
                                    return Err(Box::from(format!(
                                        "Cannot assign to non-public value `{}` on line {}",
                                        token, line_number
                                    )));
                                }
                            }
                            None => {
                                return Err(Box::from(format!(
                                    "Cannot access private value `{}` on line {}",
                                    token, line_number
                                )));
                            }
                            Some(Modifier::Public) => (),
                        }
                    } else {
                        ensure_mutation_declared(token, ctx)?;
                    }
                } else {
                    // Check: cannot find property definition and it has an enclosing type
                    return Err(Box::from(format!(
                        "Use of undeclared identifier `{}` at line {}",
                        token, line_number
                    )));
                }
            } else if let Some(scope) = &ctx.scope_context {
                if let Some(declaration) = scope.declaration(token) {
                    // Previously we had checks for ctx.in_subscript and !declaration.variable_type.is_inout_type()
                    // TODO the is_enclosing conjunct is required for now because of naming conflicts: the analyser cannot
                    // yet distinguish self.<var> from <var> when reassigning. So could have const self.<var> and
                    // local <var>, try to reassign to <var> and it think you are reassigning to self.<var>
                    if declaration.is_constant() && ctx.is_lvalue && !ctx.is_enclosing {
                        return Err(Box::from(format!(
                            "Reassignment to constant `{}` on line {}",
                            token, line_number
                        )));
                    }
                } else if !ctx.environment.is_enum_declared(token) {
                    identifier.enclosing_type =
                        Option::from(ctx.enclosing_type_identifier().unwrap().token.clone());
                    if let Some(type_id) = &identifier.enclosing_type {
                        if ctx.environment.is_property_defined(token, type_id) {
                            ensure_mutation_declared(token, ctx)?;
                        } else if let Some(scope) = &ctx.scope_context {
                            return if scope.is_declared(token) {
                                Ok(())
                            } else if let Some(contract) =
                                &ctx.contract_behaviour_declaration_context
                            {
                                if let Some(caller) = &contract.caller {
                                    if *token == caller.token {
                                        return Ok(());
                                    }
                                }

                                Err(Box::from(format!(
                                    "Use of undeclared identifier `{}` at line {}",
                                    token, line_number
                                )))
                            } else {
                                Err(Box::from(format!(
                                    "Use of undeclared identifier `{}` at line {}",
                                    token, line_number
                                )))
                            };
                        }
                    }
                } else if !ctx.is_enclosing {
                    return Err(Box::from(format!(
                        "Invalid reference to `{}` on line {}",
                        token, line_number
                    )));
                }
                // Before we had another check for if there was an enum declared, but I don't think it did anything
            }
        }

        Ok(())
    }

    fn start_range_expression(
        &mut self,
        range_expression: &mut RangeExpression,
        _context: &mut Context,
    ) -> VResult {
        let start = range_expression.start_expression.clone();
        let end = range_expression.end_expression.clone();

        if is_literal(start.as_ref()) && is_literal(end.as_ref()) {
        } else {
            return Err(Box::from(format!(
                "Invalid Range Declaration: {:?}",
                range_expression
            )));
        }

        Ok(())
    }

    fn start_caller_protection(
        &mut self,
        protection: &mut CallerProtection,
        context: &mut Context,
    ) -> VResult {
        if context.enclosing_type_identifier().is_some()
            && !protection.is_any()
            && !context.environment.contains_caller_protection(
                protection,
                &context.enclosing_type_identifier().unwrap().token,
            )
        {
            return Err(Box::from(format!(
                "Undeclared caller protection {}",
                protection.identifier.token
            )));
        }

        Ok(())
    }

    fn start_conformance(
        &mut self,
        conformance: &mut Conformance,
        context: &mut Context,
    ) -> VResult {
        if !context.environment.is_trait_declared(&conformance.name()) {
            return Err(Box::from(format!(
                "Undeclared trait `{}` used",
                conformance.identifier.token
            )));
        }
        Ok(())
    }

    fn start_attempt_expression(
        &mut self,
        attempt: &mut AttemptExpression,
        _context: &mut Context,
    ) -> VResult {
        if attempt.is_soft() {}

        Ok(())
    }

    fn finish_binary_expression(
        &mut self,
        expression: &mut BinaryExpression,
        context: &mut crate::context::Context,
    ) -> VResult {
        let scope = context.scope_context.as_ref().unwrap();
        let enclosing = context.enclosing_type_identifier().unwrap();
        let left_type = if expression.op.is_assignment() {
            match *expression.lhs_expression {
                Expression::Identifier(_) => context.environment.get_expression_type(
                    &*expression.lhs_expression,
                    &enclosing.token,
                    context.type_states(),
                    context.caller_protections(),
                    scope,
                ),
                Expression::VariableDeclaration(ref declaration) => {
                    declaration.variable_type.clone()
                }
                Expression::SubscriptExpression(ref subscript) => {
                    let arr_type = context.environment.get_expression_type(
                        &Expression::Identifier(subscript.base_expression.clone()),
                        &enclosing.token,
                        context.type_states(),
                        context.caller_protections(),
                        scope,
                    );

                    match arr_type {
                        Type::ArrayType(ArrayType { key_type }) => *key_type,
                        Type::FixedSizedArrayType(FixedSizedArrayType { key_type, .. }) => {
                            *key_type
                        }
                        _ => {
                            return Err(Box::from(format!(
                                "Subscript expression on non-array type: {:?}",
                                expression.lhs_expression
                            )))
                        }
                    }
                }
                Expression::BinaryExpression(ref binary) if matches!(binary.op, BinOp::Dot) => {
                    context.environment.get_expression_type(
                        &*expression.lhs_expression,
                        &enclosing.token,
                        context.type_states(),
                        context.caller_protections(),
                        scope,
                    )
                }
                _ => {
                    return Err(Box::from(format!(
                        "Assignment to non-expression {:?}",
                        expression.lhs_expression
                    )));
                }
            }
        } else {
            context.environment.get_expression_type(
                &*expression.lhs_expression,
                &enclosing.token,
                context.type_states(),
                context.caller_protections(),
                scope,
            )
        };
        let right_type = context.environment.get_expression_type(
            &*expression.rhs_expression,
            &enclosing.token,
            context.type_states(),
            context.caller_protections(),
            scope,
        );
        if expression.op.accepts(&left_type, &right_type) {
            Ok(())
        } else if let BinOp::Equal = expression.op {
            Err(Box::from(format!(
                "Attempt to assign type `{}` to type `{}` on {}",
                right_type, left_type, &expression.line_info
            )))
        } else {
            Err(Box::from(format!(
                "Invalid types `{}`, `{}` for operator `{}` on {}",
                left_type, right_type, expression.op, &expression.line_info
            )))
        }
    }

    fn start_function_call(
        &mut self,
        call: &mut FunctionCall,
        context: &mut crate::context::Context,
    ) -> VResult {
        // We assume runtime function calls are fine since they are only called by generated code
        if Environment::is_runtime_function_call(call) {
            return Ok(());
        }

        let fail = |candidates: Candidates, type_id: &str| {
            if let Some(first) = candidates.candidates.first() {
                Err(Box::from(format!(
                    "Could not call `{}` with ({}) on {}, did you mean to call `{}` with ({}){}",
                    &call.identifier.token,
                    &context
                        .environment
                        .argument_types(&call, type_id, context.scope_or_default())
                        .join(", "),
                    &call.identifier.line_info,
                    first.name(),
                    first.get_parameter_types().iter().join(", "),
                    first
                        .line_info()
                        .map(|line| format!(" on {}", line))
                        .as_deref()
                        .unwrap_or("")
                )))
            } else {
                Err(Box::from(format!(
                    "Undefined function `{}` called on {}",
                    &call.identifier.token, &call.identifier.line_info
                )))
            }
        };

        let called_on_type = call
            .identifier
            .enclosing_type
            .as_deref()
            .or_else(|| context.declaration_context_type_id())
            .unwrap_or_default();

        if let Some(ref behaviour_context) = context.contract_behaviour_declaration_context {
            let contract_name = &*behaviour_context.identifier.token;
            let contract_call = contract_name == called_on_type;

            let function_info = context.environment.match_function_call(
                &call,
                called_on_type,
                &behaviour_context.caller_protections,
                context.scope_or_default(),
            );

            match function_info {
                MatchedFunction(info) if contract_call => check_if_correct_type_state_possible(
                    behaviour_context,
                    context.environment.get_contract_state(contract_name),
                    &info.type_states,
                    &call.identifier,
                ),
                MatchedInitializer(info) if contract_call => check_if_correct_type_state_possible(
                    behaviour_context,
                    context.environment.get_contract_state(contract_name),
                    &info.type_states,
                    &call.identifier,
                ),
                Failure(candidates) => fail(candidates, contract_name),
                _ => Ok(()),
            }
        } else {
            match context.environment.match_function_call(
                &call,
                called_on_type,
                &[],
                context.scope_or_default(),
            ) {
                Failure(candidates) => fail(
                    candidates,
                    context.declaration_context_type_id().unwrap_or_default(),
                ),
                _ => Ok(()),
            }
        }
    }

    #[allow(clippy::single_match)]
    fn finish_if_statement(
        &mut self,
        if_statement: &mut IfStatement,
        _context: &mut Context,
    ) -> VResult {
        match &if_statement.condition {
            Expression::BinaryExpression(ref b) => {
                if let Expression::VariableDeclaration(ref v) = *b.lhs_expression {
                    if !v.is_constant() {
                        return Err(Box::from(format!(
                            "Invalid condition type in `if` statement on {}",
                            if_statement.condition.get_line_info()
                        )));
                    }
                }
            }
            _ => {}
        }

        let expression_type = Type::Int;
        //TODO expression type

        if expression_type.is_bool_type() {
            return Err(Box::from(format!(
                "Invalid condition type in `if` statement on {}",
                if_statement.condition.get_line_info()
            )));
        }

        Ok(())
    }

    fn finish_return_statement(
        &mut self,
        statement: &mut ReturnStatement,
        context: &mut crate::context::Context,
    ) -> VResult {
        let function_context = context.function_declaration_context.as_ref().unwrap();
        let enclosing = context.enclosing_type_identifier().unwrap();

        // This means we simply trust the standard library is written correctly TODO better way?
        if enclosing.token.eq("Flint_Global") {
            return Ok(());
        }

        let scope = context.scope_context.as_ref().unwrap();

        if let Some(result) = function_context.declaration.get_result_type() {
            if let Some(ref expression) = statement.expression {
                let expression_type = context.environment.get_expression_type(
                    expression,
                    &enclosing.token,
                    context.type_states(),
                    context.caller_protections(),
                    scope,
                );
                if expression_type != *result {
                    Err(Box::from(format!(
                        "Cannot return value of type `{}` when `{}` expected on {}",
                        expression_type, result, statement.line_info
                    )))
                } else {
                    Ok(())
                }
            } else {
                Err(Box::from(format!(
                    "Must return value when `{}` expected on {}",
                    result, statement.line_info
                )))
            }
        } else if let Some(ref expression) = statement.expression {
            let expression_type = context.environment.get_expression_type(
                expression,
                &enclosing.token,
                context.type_states(),
                context.caller_protections(),
                scope,
            );
            Err(Box::from(format!(
                "No return value expected but `{}` found on {}",
                expression_type, statement.line_info
            )))
        } else {
            Ok(())
        }
    }

    fn start_assertion(&mut self, assertion: &mut Assertion, context: &mut Context) -> VResult {
        let enclosing_type = context
            .enclosing_type_identifier()
            .map(|id| &*id.token)
            .unwrap_or_default();
        let (type_states, caller_protections) =
            if let Some(info) = &context.contract_behaviour_declaration_context {
                (info.type_states.clone(), info.caller_protections.clone())
            } else {
                (vec![], vec![])
            };

        if let Type::Bool = context.environment.get_expression_type(
            &assertion.expression,
            enclosing_type,
            &type_states,
            &caller_protections,
            context.scope_or_default(),
        ) {
            Ok(())
        } else {
            Err(Box::from(format!(
                "Assertion expression must evaluate to boolean on {}",
                assertion.line_info
            )))
        }
    }
}

fn check_if_correct_type_state_possible(
    declaration_context: &crate::context::ContractBehaviourDeclarationContext,
    current_state: Option<TypeState>,
    allowed_states: &[TypeState],
    function_id: &Identifier,
) -> VResult {
    let current_possible_states = if let Some(state) = current_state {
        vec![state]
    } else {
        declaration_context.type_states.clone()
    };

    // If any type state is allowed, or if we could be in any type state, then we must check at runtime instead
    if allowed_states.is_empty()
        || current_possible_states.is_empty()
        || current_possible_states
            .iter()
            .all(|state| allowed_states.contains(state))
    {
        Ok(())
    } else {
        let err = format!(
            "Must definitely be in one of the following states to make function call {} on {}: {:?}.",
            function_id.token,
            function_id.line_info,
            allowed_states
                .iter()
                .map(|state| state.identifier.token.clone())
                .collect::<Vec<String>>(),
        );
        Err(Box::from(err))
    }
}

fn is_conformance_repeated<'a, T: IntoIterator<Item = &'a Conformance>>(conformances: T) -> bool {
    !conformances
        .into_iter()
        .map(|c| &c.identifier.token)
        .is_unique()
}

fn code_block_returns(block: &[Statement]) -> bool {
    let mut branches = block
        .iter()
        .filter_map(|statement| {
            if let Statement::IfStatement(branch) = statement {
                Some(branch)
            } else {
                None
            }
        })
        .peekable();

    block
        .iter()
        .any(|statements| matches!(statements, Statement::ReturnStatement(_)))
        || (branches.peek().is_some()
            && branches.all(|branch| {
                code_block_returns(&branch.body) && code_block_returns(&branch.else_body)
            }))
}

fn ensure_mutation_declared(token: &str, ctx: &Context) -> VResult {
    if let Some(function_declaration_context) = ctx.function_declaration_context.as_ref() {
        // Check: Do not allow mutation of identifier if it is not declared mutating
        if !function_declaration_context
            .mutates()
            .iter()
            .any(|id| id.token == token)
            && ctx.is_lvalue
        {
            return Err(Box::from(format!(
                "Mutating identifier `{}` which is not declared mutating at line {}",
                token,
                function_declaration_context
                    .declaration
                    .head
                    .identifier
                    .line_info
                    .line
            )));
        }
    }
    Ok(())
}
