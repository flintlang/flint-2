use super::ast::*;
use super::context::*;
use super::visitor::*;
use crate::environment::FunctionCallMatchResult::{MatchedFunction, MatchedInitializer};
use crate::type_checker::ExpressionChecker;
use crate::utils::unique::Unique;

pub struct SemanticAnalysis {}

impl Visitor for SemanticAnalysis {
    fn start_contract_declaration(
        &mut self,
        _t: &mut ContractDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        if !_ctx
            .environment
            .has_public_initialiser(&_t.identifier.token)
        {
            return Err(Box::from(format!(
                "No public Initialiser for contract {}",
                _t.identifier.token
            )));
        }

        if _ctx.environment.is_conflicting(&_t.identifier) {
            return Err(Box::from(format!(
                "Conflicting Declarations for {}",
                _t.identifier.token
            )));
        }

        if is_conformance_repeated(&_t.conformances) {
            return Err(Box::from("Conformances are repeated".to_owned()));
        }

        if _ctx
            .environment
            .conflicting_trait_signatures(&_t.identifier.token)
        {
            return Err(Box::from("Conflicting traits".to_owned()));
        }
        Ok(())
    }

    fn start_contract_behaviour_declaration(
        &mut self,
        _t: &mut ContractBehaviourDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        if !_ctx.environment.is_contract_declared(&_t.identifier.token) {
            return Err(Box::from(format!(
                "Undeclared contract {}",
                _t.identifier.token
            )));
        }

        let stateful = _ctx.environment.is_contract_stateful(&_t.identifier.token);
        let states = _t.type_states.clone();
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

        if !_ctx.is_trait_declaration_context() {
            let members = _t.members.clone();
            for member in members {
                match member {
                    ContractBehaviourMember::SpecialSignatureDeclaration(_)
                    | ContractBehaviourMember::FunctionSignatureDeclaration(_) => {
                        return Err(Box::from(format!(
                            "Signature Declaration in Contract {}",
                            _t.identifier.token
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
        _t: &mut StructDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        if _ctx.environment.is_conflicting(&_t.identifier) {
            let i = _t.identifier.token.clone();
            return Err(Box::from(format!("Conflicting Declarations for {}", i)));
        }

        if _ctx.environment.is_recursive_struct(&_t.identifier.token) {
            return Err(Box::from(format!(
                "Recusive Struct Definition for {}",
                _t.identifier.token
            )));
        }

        if is_conformance_repeated(&_t.conformances) {
            return Err(Box::from("Conformances are repeated".to_owned()));
        }

        if _ctx
            .environment
            .conflicting_trait_signatures(&_t.identifier.token)
        {
            return Err(Box::from("Conflicting Traits".to_owned()));
        }
        Ok(())
    }

    fn start_asset_declaration(
        &mut self,
        _t: &mut AssetDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        if _ctx.environment.is_conflicting(&_t.identifier) {
            return Err(Box::from(format!(
                "Conflicting Declarations for {}",
                _t.identifier.token.clone()
            )));
        }

        Ok(())
    }

    fn start_trait_declaration(
        &mut self,
        _t: &mut TraitDeclaration,
        _ctx: &mut Context,
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
                "Type {:?} is not declared",
                declaration.variable_type
            )));
        }

        if ctx.in_function_or_special() {
            if let Some(ref mut scope_context) = ctx.scope_context {
                let redeclaration = scope_context.declaration(&declaration.identifier.token);
                if redeclaration.is_some() {
                    return Err(Box::from(format!(
                        "Redeclaration of identifier {} on line {}",
                        declaration.identifier.token, declaration.identifier.line_info.line,
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

            if &identifier.token == "Libra" || &identifier.token == "Wei" {
                return Ok(());
            }
        }

        if !declaration
            .head
            .parameters
            .iter()
            .map(|p| &p.identifier.token)
            .unique()
        {
            return Err(Box::from(format!(
                "Fuction {} has duplicate parameters",
                declaration.head.identifier.token
            )));
        }

        let remaining_parameters = declaration
            .head
            .parameters
            .iter()
            .filter(|p| p.is_payable())
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
                .filter(|p| p.is_dynamic() && !p.is_payable())
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
        _t: &mut FunctionDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        _t.scope_context = _ctx.scope_context.clone();
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
                if enclosing_type == "Libra" || enclosing_type == "Wei" {
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
                        if let Some(context) = &ctx.function_declaration_context {
                            context.declaration.head.identifier.enclosing_type.clone()
                        } else if let Some(context) = &ctx.special_declaration_context {
                            context.declaration.head.enclosing_type.clone()
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

    fn start_range_expression(&mut self, _t: &mut RangeExpression, _ctx: &mut Context) -> VResult {
        let start = _t.start_expression.clone();
        let end = _t.end_expression.clone();

        if is_literal(start.as_ref()) && is_literal(end.as_ref()) {
        } else {
            return Err(Box::from(format!("Invalid Range Declaration: {:?}", _t)));
        }

        Ok(())
    }

    fn start_caller_protection(
        &mut self,
        _t: &mut CallerProtection,
        _ctx: &mut Context,
    ) -> VResult {
        if _ctx.enclosing_type_identifier().is_some()
            && !_t.is_any()
            && !_ctx
                .environment
                .contains_caller_protection(_t, &_ctx.enclosing_type_identifier().unwrap().token)
        {
            return Err(Box::from(format!(
                "Undeclared caller protection {}",
                _t.identifier.token
            )));
        }

        Ok(())
    }

    fn start_conformance(&mut self, _t: &mut Conformance, _ctx: &mut Context) -> VResult {
        if !_ctx.environment.is_trait_declared(&_t.name()) {
            return Err(Box::from(format!(
                "Undeclared trait `{}` used",
                _t.identifier.token
            )));
        }
        Ok(())
    }

    fn start_attempt_expression(
        &mut self,
        _t: &mut AttemptExpression,
        _ctx: &mut Context,
    ) -> VResult {
        if _t.is_soft() {}

        Ok(())
    }

    fn start_binary_expression(
        &mut self,
        _t: &mut BinaryExpression,
        _ctx: &mut Context,
    ) -> VResult {
        /* unused code: match _t.op {
            BinOp::Dot => {}
            BinOp::Equal => {
                let rhs = _t.rhs_expression.clone();
                match *rhs {
                    Expression::ExternalCall(_) => {}
                    _ => {}
                }
            }
            _ => {}
        }*/
        Ok(())
    }

    fn start_function_call(&mut self, t: &mut FunctionCall, ctx: &mut Context) -> VResult {
        let contract_name = ctx.contract_behaviour_declaration_context.clone();
        if let Some(context) = contract_name {
            let contract_name = context.identifier.token.clone();

            let function_info = ctx.environment.match_function_call(
                &t,
                &contract_name,
                &context.caller_protections,
                ctx.scope_context.as_ref().unwrap_or_default(),
            );
            return match function_info {
                MatchedFunction(info) => check_if_correct_type_state_possible(
                    context,
                    ctx.environment.get_contract_state(&contract_name),
                    info.type_states,
                    t.identifier.clone(),
                ),
                MatchedInitializer(info) => check_if_correct_type_state_possible(
                    context,
                    ctx.environment.get_contract_state(&contract_name),
                    info.type_states,
                    t.identifier.clone(),
                ),
                // Otherwise we are not calling a contract method so type states do not apply
                _ => Ok(()),
            };
        }

        Ok(())
    }

    #[allow(clippy::single_match)]
    fn finish_if_statement(&mut self, _t: &mut IfStatement, _ctx: &mut Context) -> VResult {
        let condition = _t.condition.clone();

        match condition {
            Expression::BinaryExpression(b) => {
                if let Expression::VariableDeclaration(ref v) = *b.lhs_expression {
                    if !v.is_constant() {
                        return Err(Box::from(
                            "Invalid condition type in `if` statement".to_owned(),
                        ));
                    }
                }
            }
            _ => {}
        }

        let expression_type = Type::Int;
        //TODO expression type

        if expression_type.is_bool_type() {
            return Err(Box::from(
                "Invalid condition type in `if` statement".to_owned(),
            ));
        }

        Ok(())
    }

    fn finish_statement(&mut self, _t: &mut Statement, _ctx: &mut Context) -> VResult {
        //TODO make receiver call trail empty
        Ok(())
    }

    fn start_assertion(&mut self, assertion: &mut Assertion, ctx: &mut Context) -> VResult {
        let enclosing_type = ctx
            .enclosing_type_identifier()
            .map(|id| &*id.token)
            .unwrap_or_default();
        let (type_states, caller_protections) =
            if let Some(info) = &ctx.contract_behaviour_declaration_context {
                (info.type_states.clone(), info.caller_protections.clone())
            } else {
                (vec![], vec![])
            };

        if let Type::Bool = ctx.environment.get_expression_type(
            &assertion.expression,
            enclosing_type,
            &type_states,
            &caller_protections,
            ctx.scope_context.as_ref().unwrap_or_default(),
        ) {
            Ok(())
        } else {
            Err(Box::from(format!(
                "Assertion expression at line {} must evaluate to boolean",
                assertion.line_info.line
            )))
        }
    }
}

fn check_if_correct_type_state_possible(
    context: ContractBehaviourDeclarationContext,
    current_state: Option<TypeState>,
    allowed_states: Vec<TypeState>,
    function_id: Identifier,
) -> VResult {
    let current_possible_states = if let Some(state) = current_state {
        vec![state]
    } else {
        context.type_states
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
            "Must definitely be in one of the following states to make function call {} on line {}: {:?}.",
            function_id.token,
            function_id.line_info.line,
            allowed_states
                .iter()
                .map(|state| state.identifier.token.clone())
                .collect::<Vec<String>>(),
        );
        Err(Box::from(err))
    }
}

fn is_conformance_repeated<'a, T: IntoIterator<Item=&'a Conformance>>(conformances: T) -> bool {
    !conformances
        .into_iter()
        .map(|c| &c.identifier.token)
        .unique()
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
