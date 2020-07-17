use super::ast::*;
use super::context::*;
use super::visitor::*;

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

        if is_conformance_repeated(_t.conformances.clone()) {
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

        if is_conformance_repeated(_t.conformances.clone()) {
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
        _t: &mut VariableDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        let type_declared = match &_t.variable_type {
            Type::UserDefinedType(t) => _ctx.environment.is_type_declared(&t.token.clone()),
            _ => true,
        };

        if !type_declared {
            return Err(Box::from(format!(
                "Type {:?} is not declared",
                _t.variable_type
            )));
        }

        if _ctx.in_function_or_special() {
            if _ctx.has_scope_context() {
                let scope_context = _ctx.scope_context.as_mut().unwrap();

                let redeclaration = scope_context.declaration(_t.identifier.token.clone());
                if redeclaration.is_some() {
                    return Err(Box::from(format!(
                        "Redeclaration of identifier {}",
                        _t.identifier.token
                    )));
                }
                scope_context.local_variables.push(_t.clone());
            }
        } else if _ctx.enclosing_type_identifier().is_some() {
            let identifier = &_ctx.enclosing_type_identifier().unwrap().token.clone();
            if _ctx
                .environment
                .conflicting_property_declaration(&_t.identifier, identifier)
            {
                return Err(Box::from("Conflicting property declarations".to_owned()));
            }
        }

        Ok(())
    }

    fn start_function_declaration(
        &mut self,
        _t: &mut FunctionDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        if _ctx.enclosing_type_identifier().is_some() {
            let identifier = &_ctx.enclosing_type_identifier().unwrap().token.clone();
            if _ctx
                .environment
                .is_conflicting_function_declaration(&_t, identifier)
            {
                return Err(Box::from(format!(
                    "Conflicting Function Declarations for {}",
                    _t.head.identifier.token
                )));
            }

            if identifier == "Libra" || identifier == "Wei" {
                return Ok(());
            }
        }

        let parameters: Vec<String> = _t
            .head
            .parameters
            .clone()
            .into_iter()
            .map(|p| p.identifier.token)
            .collect();
        let duplicates =
            (1..parameters.len()).any(|i| parameters[i..].contains(&parameters[i - 1]));

        if duplicates {
            return Err(Box::from(format!(
                "Fuction {} has duplicate parameters",
                _t.head.identifier.token
            )));
        }

        let payable_parameters = _t.head.parameters.clone();
        let remaining_parameters: Vec<Parameter> = payable_parameters
            .into_iter()
            .filter(|p| p.is_payable())
            .collect();
        if _t.is_payable() {
            if remaining_parameters.is_empty() {
                return Err(Box::from(format!(
                    "Payable Function {} does not have payable parameter",
                    _t.head.identifier.token
                )));
            } else if remaining_parameters.len() > 1 {
                return Err(Box::from(format!(
                    "Payable parameter is ambiguous in function {}",
                    _t.head.identifier.token
                )));
            }
        } else if !remaining_parameters.is_empty() {
            let params = remaining_parameters.clone();
            return Err(Box::from(format!(
                "Function not marked payable but has payable parameter: {:?}",
                params
            )));
        }

        if _t.is_public() {
            let parameters: Vec<Parameter> = _t
                .head
                .parameters
                .clone()
                .into_iter()
                .filter(|p| p.is_dynamic() && !p.is_payable())
                .collect();
            if !parameters.is_empty() {
                return Err(Box::from(format!(
                    "Public Function {} has dynamic parameters",
                    _t.head.identifier.token
                )));
            }
        }

        let return_type = &_t.head.result_type;
        if return_type.is_some() {
            match return_type.as_ref().unwrap() {
                Type::UserDefinedType(_) => {
                    return Err(Box::from(format!(
                        "Not allowed to return struct in function {}",
                        _t.head.identifier.token
                    )));
                }
                _ => (),
            }
        }

        let statements = _t.body.clone();
        let mut return_statements = Vec::new();
        let mut become_statements = Vec::new();

        let remaining = statements
            .into_iter()
            .skip_while(|s| !is_return_or_become_statement(s.clone()));

        for statement in _t.body.clone() {
            match statement {
                Statement::ReturnStatement(ret) => return_statements.push(ret),
                Statement::BecomeStatement(bec) => become_statements.push(bec),
                _ => continue,
            }
        }

        let remaining_after_end = remaining.filter(|s| !is_return_or_become_statement(s.clone()));
        if remaining_after_end.count() > 0 {
            return Err(Box::from(format!(
                "Statements after Return in {}",
                _t.head.identifier.token
            )));
        }

        if _t.head.result_type.is_some() && return_statements.is_empty() {
            return Err(Box::from(format!(
                "Missing Return in Function {}",
                _t.head.identifier.token
            )));
        }

        if return_statements.len() > 1 {
            return Err(Box::from(format!(
                "Multiple Returns in function {}",
                _t.head.identifier.token
            )));
        }

        if become_statements.len() > 1 {
            return Err(Box::from(format!(
                "Multiple Become Statements in {}",
                _t.head.identifier.token
            )));
        }

        for become_statement in &become_statements {
            for return_statement in &return_statements {
                if return_statement.line_info.line
                    > become_statement.state.identifier.line_info.line
                {
                    return Err(Box::from(format!(
                        "Return statement after Become in function {}",
                        _t.head.identifier.token
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
        _t: &mut SpecialDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        if _t.is_fallback() && _t.head.has_parameters() {
            return Err(Box::from(format!(
                "Fallback {} declared with arguments",
                _t.head.special_token
            )));
            // TODO check body only has simple statements bit long
        }

        Ok(())
    }

    fn start_identifier(&mut self, _t: &mut Identifier, _ctx: &mut Context) -> VResult {
        let token = _t.token.clone();
        if token.contains('@') {
            return Err(Box::from(
                "Invalid @ character used in Identifier".to_string(),
            ));
        }

        if _ctx.is_property_default_assignment
            && !_ctx.environment.is_struct_declared(&_t.token)
            && !_ctx.environment.is_asset_declared(&_t.token)
            && _ctx.enclosing_type_identifier().is_some()
        {
            return if _ctx.environment.is_property_defined(
                _t.token.clone(),
                &_ctx.enclosing_type_identifier().unwrap().token,
            ) {
                Err(Box::from(
                    "State property used withing property initiliaser".to_owned(),
                ))
            } else {
                Err(Box::from("Use of undeclared identifier".to_owned()))
            };
        }

        if _ctx.is_function_call_context || _ctx.is_function_call_argument_label {
        } else if _ctx.in_function_or_special() && !_ctx.in_become && !_ctx.in_emit {
            let is_l_value = _ctx.is_lvalue;
            if _t.enclosing_type.is_none() {
                let scope = _ctx.scope_context.is_some();
                if scope {
                    let variable_declaration =
                        _ctx.scope_context().unwrap().declaration(_t.token.clone());
                    if variable_declaration.is_some() {
                        let variable_declaration = _ctx
                            .scope_context()
                            .unwrap()
                            .declaration(_t.token.clone())
                            .unwrap();
                        if variable_declaration.is_constant()
                            && !variable_declaration.variable_type.is_inout_type()
                            && is_l_value
                            && _ctx.in_subscript
                        {
                            return Err(Box::from("Reassignment to constant".to_string()));
                        }
                    } else if !_ctx.environment.is_enum_declared(&_t.token) {
                        let enclosing = _ctx.enclosing_type_identifier();
                        let enclosing = enclosing.unwrap();
                        _t.enclosing_type = Option::from(enclosing.token);
                    } else if !_ctx.is_enclosing {
                        return Err(Box::from("Invalid reference".to_string()));
                    }
                }
            }

            if _t.enclosing_type.is_some()
                && _t.enclosing_type.as_ref().unwrap() != "Quartz$ErrorType"
            {
                let enclosing = _t.enclosing_type.clone();
                let enclosing = enclosing.unwrap();
                if enclosing == "Libra".to_string() || enclosing == "Wei".to_string() {
                    return Ok(());
                }
                if !_ctx
                    .environment
                    .is_property_defined(_t.token.clone(), &_t.enclosing_type.as_ref().unwrap())
                {
                    let identifier = _t.token.clone();
                    return Err(Box::from(format!(
                        "Use of Undeclared Identifier {ident}",
                        ident = identifier
                    )));
                    //TODO add add used undefined variable to env
                } else if is_l_value && !_ctx.in_subscript {
                    if _ctx.environment.is_property_constant(
                        _t.token.clone(),
                        &_t.enclosing_type.as_ref().unwrap(),
                    ) {}

                    if _ctx.is_special_declaration_context() {}

                    if _ctx.is_function_declaration_context() {
                        let mutated = _ctx
                            .function_declaration_context
                            .as_ref()
                            .unwrap()
                            .mutates()
                            .clone();
                        let mutated: Vec<String> = mutated.into_iter().map(|i| i.token).collect();
                        if !mutated.contains(&_t.token) {
                            let i = _t.token.clone();
                            let i = format!(
                                "Mutating {i} identifier that is declared non mutating in {f}",
                                i = i,
                                f = enclosing
                            );

                            return Err(Box::from(format!(
                                "{}, {}",
                                i,
                                _ctx.function_declaration_context
                                    .as_ref()
                                    .unwrap()
                                    .declaration
                                    .head
                                    .identifier
                                    .token
                            )));
                        }
                    }
                }
            }
        } else if _ctx.in_become {
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
                "Undeclared Trait {} Used",
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
        match _t.op {
            BinOp::Dot => {}
            BinOp::Equal => {
                let rhs = _t.rhs_expression.clone();
                match *rhs {
                    Expression::ExternalCall(_) => {}
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn start_function_call(&mut self, _t: &mut FunctionCall, _ctx: &mut Context) -> VResult {
        Ok(())
    }

    fn finish_if_statement(&mut self, _t: &mut IfStatement, _ctx: &mut Context) -> VResult {
        let condition = _t.condition.clone();

        match condition {
            Expression::BinaryExpression(b) => {
                let lhs = *b.lhs_expression.clone();

                if let Expression::VariableDeclaration(v) = lhs {
                    if !v.is_constant() {
                        return Err(Box::from(
                            "Invalid Condition Type in If statement".to_owned(),
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
                "Invalid Condition Type in If statement".to_owned(),
            ));
        }
        Ok(())
    }

    fn finish_statement(&mut self, _t: &mut Statement, _ctx: &mut Context) -> VResult {
        //TODO make recevier call trail empty
        Ok(())
    }
}

fn is_conformance_repeated(conformances: Vec<Conformance>) -> bool {
    let slice: Vec<String> = conformances
        .into_iter()
        .map(|c| c.identifier.token)
        .collect();
    (1..slice.len()).any(|i| slice[i..].contains(&slice[i - 1]))
}
