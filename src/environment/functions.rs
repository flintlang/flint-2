use crate::ast::{
    CallerProtection, FunctionArgument, FunctionCall, FunctionDeclaration, FunctionInformation,
    FunctionSignatureDeclaration, Type, TypeInfo, TypeState, VariableDeclaration,
};
use crate::context::ScopeContext;
use crate::environment::*;
use crate::type_checker::ExpressionChecker;

impl Environment {
    pub fn add_function(
        &mut self,
        f: FunctionDeclaration,
        type_id: &str,
        caller_protections: Vec<CallerProtection>,
        type_states: Vec<TypeState>,
    ) {
        let name = f.head.identifier.token.clone();
        let mutating = f.is_mutating();
        let function_information = FunctionInformation {
            declaration: f,
            mutating,
            caller_protections,
            type_states,
            ..Default::default()
        };
        let type_info = if let Some(type_info) = self.types.get_mut(type_id) {
            type_info
        } else {
            self.types.insert(type_id.to_string(), TypeInfo::new());
            self.types.get_mut(type_id).unwrap()
        };

        if let Some(function_set) = type_info.functions.get_mut(&name) {
            function_set.push(function_information);
        } else {
            type_info.functions.insert(name, vec![function_information]);
        }
    }

    pub fn remove_function(&mut self, function: &FunctionDeclaration, type_id: &str) {
        let name = function.head.identifier.token.clone();
        if let Some(type_info) = self.types.get_mut(type_id) {
            if let Some(function_set) = type_info.functions.remove(&name) {
                let function_set = function_set
                    .into_iter()
                    .filter(|f| {
                        /* The original code had this without the !(..), it was added
                        as it seems to be the desired intent */
                        !(f.declaration.head.identifier.token == name
                            && f.declaration.parameters_and_types()
                                == function.parameters_and_types())
                    })
                    .collect();
                type_info.functions.insert(name, function_set);
            }
        }
    }

    pub fn add_function_signature(
        &mut self,
        signature: FunctionSignatureDeclaration,
        type_id: &str,
        caller_protections: Vec<CallerProtection>,
        type_states: Vec<TypeState>,
        is_external: bool,
    ) {
        let name = signature.identifier.token.clone();
        let function_declaration = FunctionDeclaration {
            head: signature,
            body: vec![],
            scope_context: None,
            tags: vec![],
            mangled_identifier: None,
            is_external,
        };

        let function_information = FunctionInformation {
            declaration: function_declaration.clone(),
            mutating: function_declaration.is_mutating(),
            caller_protections,
            type_states,
            is_signature: true,
        };
        if let Some(type_info) = self.types.get_mut(type_id) {
            if let Some(function_set) = type_info.functions.get_mut(&name) {
                function_set.push(function_information);
            } else {
                type_info.functions.insert(name, vec![function_information]);
            }
        } else {
            self.types.insert(type_id.to_string(), TypeInfo::new());
            // TODO consider using a map literal crate
            self.types
                .get_mut(type_id)
                .unwrap()
                .functions
                .insert(name, vec![function_information]);
        }
    }

    fn match_regular_function(
        &self,
        call: &FunctionCall,
        type_id: &str,
        protections: &[CallerProtection],
        scope: &ScopeContext,
    ) -> FunctionCallMatchResult {
        let mut candidates = Vec::new();

        let argument_types: Vec<Type> = call
            .arguments
            .iter()
            .map(|a| self.get_expression_type(&a.expression, type_id, &[], &[], &scope))
            .collect();

        if let Some(type_info) = self.types.get(type_id) {
            if let Some(functions) = type_info.all_functions().get(&call.identifier.token) {
                for function in functions {
                    if self.function_call_arguments_compatible(function, call, type_id, scope)
                        && self.compatible_caller_protections(
                            protections,
                            &function.caller_protections,
                        )
                    {
                        return FunctionCallMatchResult::MatchedFunction(function.clone());
                    }
                    candidates.push(function.clone());
                    continue;
                }
            }
        }

        let matched_candidates: Vec<FunctionInformation> = candidates
            .clone()
            .into_iter()
            .filter(|c| {
                let p_types = c.get_parameter_types();
                if p_types.len() != argument_types.len() {
                    return false;
                }
                let mut arg_types = argument_types.clone();
                for p in p_types {
                    if p != arg_types.remove(0) {
                        return false;
                    }
                }
                true
            })
            .collect();

        let matched_candidates: Vec<CallableInformation> = matched_candidates
            .into_iter()
            .map(CallableInformation::FunctionInformation)
            .collect();

        if !matched_candidates.is_empty() {
            //TODO: if matched candidates is not empty then we have insufficient caller protections
            let matched_candidates = Candidates {
                candidates: matched_candidates,
            };
            return FunctionCallMatchResult::MatchedFunctionWithoutCaller(matched_candidates);
        }

        let candidates: Vec<CallableInformation> = candidates
            .into_iter()
            .map(CallableInformation::FunctionInformation)
            .collect();

        let candidates = Candidates { candidates };

        FunctionCallMatchResult::Failure(candidates)
    }

    #[allow(dead_code)]
    fn match_fallback_function(&self, call: &FunctionCall, protections: &[CallerProtection]) {
        let mut candidates = Vec::new();
        if let Some(type_info) = self.types.get(&call.identifier.token) {
            let fallbacks = &type_info.fallbacks;
            for fallback in fallbacks {
                if self.compatible_caller_protections(protections, &fallback.caller_protections) {
                    // TODO Return MatchedFallBackFunction
                } else {
                    candidates.push(fallback);
                    continue;
                }
            }
        }
        // TODO return failure
    }

    fn match_initialiser_function(
        &self,
        call: &FunctionCall,
        argument_types: &[Type],
        protections: &[CallerProtection],
    ) -> FunctionCallMatchResult {
        let mut candidates = Vec::new();

        if let Some(type_info) = self.types.get(&call.identifier.token) {
            for initialiser in &type_info.initialisers {
                let parameter_types = initialiser.parameter_types();
                let mut equal_types = true;
                for argument_type in argument_types {
                    if !parameter_types.contains(argument_type) {
                        equal_types = false;
                    }
                }

                if equal_types
                    && self
                        .compatible_caller_protections(protections, &initialiser.caller_protections)
                {
                    return FunctionCallMatchResult::MatchedInitializer(initialiser.clone());
                } else {
                    candidates.push(initialiser);
                    continue;
                }
            }
        }
        let candidates: Vec<CallableInformation> = candidates
            .into_iter()
            .map(|i| CallableInformation::SpecialInformation(i.clone()))
            .collect();

        let candidates = Candidates { candidates };
        FunctionCallMatchResult::Failure(candidates)
    }

    fn match_global_function(
        &self,
        call: &FunctionCall,
        argument_types: &[Type],
        protections: &[CallerProtection],
    ) -> FunctionCallMatchResult {
        let token = call.identifier.token.clone();
        let mut candidates = Vec::new();
        if let Some(type_info) = self.types.get(&"Quartz_Global".to_string()) {
            if let Some(functions) = type_info.functions.get(&call.identifier.token) {
                for function in functions {
                    let parameter_types = function.get_parameter_types();
                    let equal_types = argument_types
                        .iter()
                        .all(|argument_type| parameter_types.contains(argument_type));
                    if equal_types
                        && self.compatible_caller_protections(
                            protections,
                            &function.caller_protections,
                        )
                    {
                        return FunctionCallMatchResult::MatchedGlobalFunction(function.clone());
                    } else {
                        candidates.push(function);
                        continue;
                    }
                }
            }
        }
        let candidates: Vec<CallableInformation> = candidates
            .into_iter()
            .map(|i| CallableInformation::FunctionInformation(i.clone()))
            .collect();
        if token == "fatalError" {
            unimplemented!()
        }
        FunctionCallMatchResult::Failure(Candidates { candidates })
    }

    pub fn is_runtime_function_call(function_call: &FunctionCall) -> bool {
        function_call.identifier.token.starts_with("Quartz_")
    }

    pub fn match_function_call(
        &self,
        call: &FunctionCall,
        type_id: &str,
        caller_protections: &[CallerProtection],
        scope: &ScopeContext,
    ) -> FunctionCallMatchResult {
        let result = FunctionCallMatchResult::Failure(Candidates {
            ..Default::default()
        });

        let argument_types: Vec<_> = call
            .arguments
            .iter()
            .map(|a| self.get_expression_type(&a.expression, type_id, &[], &[], &scope))
            .collect();

        let regular_match = self.match_regular_function(&call, type_id, caller_protections, scope);

        let initaliser_match =
            self.match_initialiser_function(call, &argument_types, caller_protections);

        let global_match = self.match_global_function(call, &argument_types, caller_protections);

        let result = result.merge(regular_match);
        let result = result.merge(initaliser_match);
        result.merge(global_match)
    }

    fn compatible_caller_protections(
        &self,
        source: &[CallerProtection],
        target: &[CallerProtection],
    ) -> bool {
        // each caller protection in the source must match at least one caller protection in the target

        for caller_protection in source {
            let mut matched_any_parent = false;

            for parent in target {
                if caller_protection.is_sub_protection(parent) {
                    matched_any_parent = true;
                }
            }

            if !matched_any_parent {
                return false;
            }
        }

        true
    }

    fn function_call_arguments_compatible(
        &self,
        source: &FunctionInformation,
        target: &FunctionCall,
        type_id: &str,
        scope: &ScopeContext,
    ) -> bool {
        let no_self_declaration_type =
            Environment::replace_self(source.get_parameter_types(), type_id);

        let parameters: Vec<_> = source
            .declaration
            .head
            .parameters
            .iter()
            .map(|p| p.as_variable_declaration())
            .collect();

        if target.arguments.len() <= source.parameter_identifiers().len()
            && target.arguments.len() >= source.required_parameter_identifiers().len()
        {
            self.check_parameter_compatibility(
                &target.arguments,
                &parameters,
                type_id,
                scope,
                &no_self_declaration_type,
            )
        } else {
            false
        }
    }

    fn check_parameter_compatibility(
        &self,
        arguments: &[FunctionArgument],
        parameters: &[VariableDeclaration],
        enclosing: &str,
        scope: &ScopeContext,
        declared_types: &[Type],
    ) -> bool {
        let required_parameters: Vec<&VariableDeclaration> = parameters
            .iter()
            .filter(|f| f.expression.is_none())
            .collect();

        for (index, _) in required_parameters.iter().enumerate() {
            if let Some(ref argument) = arguments[index].identifier {
                if argument.token != parameters[index].identifier.token {
                    return false;
                }
            } else {
                return false;
            }

            // Check Types
            let declared_type = &declared_types[index];
            let argument_expression = &arguments[index].expression;
            let argument_type =
                self.get_expression_type(argument_expression, enclosing, &[], &[], &scope);

            if argument_type != *declared_type {
                return false;
            }
        }

        let mut index = required_parameters.len();
        let mut argument_index = index;

        while index < required_parameters.len() && argument_index < arguments.len() {
            if arguments[argument_index].identifier.is_none() {
                let declared_type = &declared_types[index];

                let argument_type = self.get_expression_type(
                    &arguments[argument_index].expression,
                    enclosing,
                    &[],
                    &[],
                    scope,
                );
                //TODO replacing self
                if argument_type != *declared_type {
                    return false;
                }
                index += 1;
                argument_index += 1;
                continue;
            }

            while index < parameters.len() {
                if let Some(ref argument) = arguments[argument_index].identifier {
                    if argument.token != parameters[index].identifier.token {
                        index += 1;
                    }
                } else {
                    break;
                }
            }

            if index == parameters.len() {
                // Identifier was not found
                return false;
            }

            // Check Types
            let declared_type = &declared_types[index];
            let argument_type = self.get_expression_type(
                &arguments[argument_index].expression,
                enclosing,
                &[],
                &[],
                scope,
            );

            if *declared_type != argument_type {
                return false;
            }

            index += 1;
            argument_index += 1;
        }

        if argument_index < arguments.len() {
            return false;
        }
        true
    }

    pub fn replace_self(list: Vec<Type>, enclosing: &str) -> Vec<Type> {
        list.into_iter()
            .map(|t| t.replacing_self(enclosing))
            .collect()
    }
}
