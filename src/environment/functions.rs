use crate::ast::{
    do_vecs_match, CallerProtection, FunctionArgument, FunctionCall, FunctionDeclaration,
    FunctionInformation, FunctionSignatureDeclaration, Type, TypeIdentifier, TypeInfo, TypeState,
    VariableDeclaration,
};
use crate::context::ScopeContext;
use crate::environment::*;
use crate::type_checker::ExpressionCheck;

impl Environment {
    pub fn add_function(
        &mut self,
        f: &FunctionDeclaration,
        t: &TypeIdentifier,
        caller_protections: Vec<CallerProtection>,
        type_states: Vec<TypeState>,
    ) {
        let name = f.head.identifier.token.clone();
        let function_information = FunctionInformation {
            declaration: f.clone(),
            mutating: f.is_mutating(),
            caller_protection: caller_protections,
            type_states,
            ..Default::default()
        };
        let type_info = if let Some(type_info) = self.types.get_mut(t) {
            type_info
        } else {
            self.types.insert(t.to_string(), TypeInfo::new());
            self.types.get_mut(t).unwrap()
        };

        if let Some(function_set) = type_info.functions.get_mut(&name) {
            function_set.push(function_information);
        } else {
            type_info.functions.insert(name, vec![function_information]);
        }
    }

    pub fn remove_function(&mut self, function: &FunctionDeclaration, t: &TypeIdentifier) {
        let name = function.head.identifier.token.clone();
        if let Some(type_info) = self.types.get_mut(t) {
            if let Some(function_set) = type_info.functions.remove(&name) {
                let function_set = function_set
                    .into_iter()
                    .filter(|f| {
                        /* The original code had this without the !(..), it was added
                        as it seems to be the desired intent */
                        !(f.declaration.head.identifier.token == name
                            && do_vecs_match(
                                &f.declaration.parameters_and_types(),
                                &function.parameters_and_types(),
                            ))
                    })
                    .collect();
                type_info.functions.insert(name, function_set);
            }
        }
    }

    pub fn add_function_signature(
        &mut self,
        f: &FunctionSignatureDeclaration,
        t: &TypeIdentifier,
        caller_protections: Vec<CallerProtection>,
        type_states: Vec<TypeState>,
        is_external: bool,
    ) {
        let name = f.identifier.token.clone();
        let function_declaration = FunctionDeclaration {
            head: f.clone(),
            body: vec![],
            scope_context: None,
            tags: vec![],
            mangled_identifier: None,
            is_external,
        };

        let function_information = FunctionInformation {
            declaration: function_declaration.clone(),
            mutating: function_declaration.is_mutating(),
            caller_protection: caller_protections,
            type_states,
            is_signature: true,
        };
        if let Some(type_info) = self.types.get_mut(t) {
            if let Some(function_set) = type_info.functions.get_mut(&name) {
                function_set.push(function_information);
            } else {
                type_info.functions.insert(name, vec![function_information]);
            }
        } else {
            self.types.insert(t.to_string(), TypeInfo::new());
            // TODO consider using a map literal crate
            self.types
                .get_mut(t)
                .unwrap()
                .functions
                .insert(name, vec![function_information]);
        }
    }

    fn match_regular_function(
        &self,
        f: FunctionCall,
        t: &TypeIdentifier,
        c: Vec<CallerProtection>,
        scope: ScopeContext,
    ) -> FunctionCallMatchResult {
        let mut candidates = Vec::new();

        let arguments = f.arguments.clone();

        let argument_types: Vec<Type> = arguments
            .into_iter()
            .map(|a| {
                self.get_expression_type(a.expression.clone(), t, vec![], vec![], scope.clone())
            })
            .collect();

        if let Some(type_info) = self.types.get(t) {
            if let Some(functions) = type_info.all_functions().get(&f.identifier.token) {
                for function in functions {
                    if self.function_call_arguments_compatible(
                        function.clone(),
                        f.clone(),
                        t,
                        scope.clone(),
                    ) {
                        if self.compatible_caller_protections(
                            c.clone(),
                            function.caller_protection.clone(),
                        ) {
                            return FunctionCallMatchResult::MatchedFunction(function.clone());
                        }
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
            .map(|i| CallableInformation::FunctionInformation(i.clone()))
            .collect();

        if !matched_candidates.is_empty() {
            let matched_candidates = Candidates {
                candidates: matched_candidates,
            };
            return FunctionCallMatchResult::MatchedFunctionWithoutCaller(matched_candidates);
        }

        let candidates: Vec<CallableInformation> = candidates
            .into_iter()
            .map(|i| CallableInformation::FunctionInformation(i.clone()))
            .collect();

        let candidates = Candidates { candidates };

        FunctionCallMatchResult::Failure(candidates)
    }

    #[allow(dead_code)]
    fn match_fallback_function(&self, f: FunctionCall, c: Vec<CallerProtection>) {
        let mut candidates = Vec::new();
        let type_info = self.types.get(&f.identifier.token.clone());
        if type_info.is_some() {
            let fallbacks = &type_info.unwrap().fallbacks;
            for fallback in fallbacks {
                if self
                    .compatible_caller_protections(c.clone(), fallback.caller_protections.clone())
                {
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
        f: FunctionCall,
        argument_types: Vec<Type>,
        c: Vec<CallerProtection>,
    ) -> FunctionCallMatchResult {
        let mut candidates = Vec::new();

        let type_info = self.types.get(&f.identifier.token.clone());
        if type_info.is_some() {
            let initialisers = &type_info.unwrap().initialisers;
            for initialiser in initialisers {
                let parameter_types = initialiser.parameter_types();
                let mut equal_types = true;
                for argument_type in argument_types.clone() {
                    if !parameter_types.contains(&argument_type) {
                        equal_types = false;
                    }
                }

                if equal_types
                    && self.compatible_caller_protections(
                        c.clone(),
                        initialiser.caller_protections.clone(),
                    )
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
        f: FunctionCall,
        argument_types: Vec<Type>,
        c: Vec<CallerProtection>,
    ) -> FunctionCallMatchResult {
        let token = f.identifier.token.clone();
        let mut candidates = Vec::new();
        let type_info = self.types.get(&"Quartz_Global".to_string());
        if type_info.is_some() {
            let functions = &type_info.unwrap().functions;
            let functions = functions.get(&f.identifier.token.clone());

            if functions.is_some() {
                let functions = functions.unwrap();

                for function in functions {
                    let parameter_types = function.get_parameter_types();
                    let mut equal_types = true;
                    for argument_type in argument_types.clone() {
                        if !parameter_types.contains(&argument_type) {
                            equal_types = false;
                        }
                    }
                    if equal_types
                        && self.compatible_caller_protections(
                            c.clone(),
                            function.caller_protection.clone(),
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
        let candidates = Candidates { candidates };
        if token == "fatalError".to_string() {
            unimplemented!()
        }
        FunctionCallMatchResult::Failure(candidates)
    }

    pub fn is_runtime_function_call(function_call: &FunctionCall) -> bool {
        let ident = function_call.identifier.token.clone();
        ident.starts_with("Quartz_")
    }

    pub fn match_function_call(
        &self,
        f: FunctionCall,
        t: &TypeIdentifier,
        caller_protections: Vec<CallerProtection>,
        scope: ScopeContext,
    ) -> FunctionCallMatchResult {
        let result = FunctionCallMatchResult::Failure(Candidates {
            ..Default::default()
        });

        let arguments = f.arguments.clone();

        let argument_types: Vec<Type> = arguments
            .into_iter()
            .map(|a| {
                self.get_expression_type(a.expression.clone(), t, vec![], vec![], scope.clone())
            })
            .collect();

        let regular_match =
            self.match_regular_function(f.clone(), t, caller_protections.clone(), scope.clone());

        let initaliser_match = self.match_initialiser_function(
            f.clone(),
            argument_types.clone(),
            caller_protections.clone(),
        );

        let global_match = self.match_global_function(
            f.clone(),
            argument_types.clone(),
            caller_protections.clone(),
        );

        let result = result.merge(regular_match);
        let result = result.merge(initaliser_match);
        result.merge(global_match)
    }

    fn compatible_caller_protections(
        &self,
        source: Vec<CallerProtection>,
        target: Vec<CallerProtection>,
    ) -> bool {
        if target.is_empty() {
            return true;
        }
        for caller_protection in source {
            for parent in &target {
                if !caller_protection.is_sub_protection(parent.clone()) {
                    return false;
                }
            }
        }
        true
    }

    fn function_call_arguments_compatible(
        &self,
        source: FunctionInformation,
        target: FunctionCall,
        t: &TypeIdentifier,
        scope: ScopeContext,
    ) -> bool {
        let no_self_declaration_type = Environment::replace_self(source.get_parameter_types(), t);

        let parameters: Vec<VariableDeclaration> = source
            .declaration
            .head
            .parameters
            .clone()
            .into_iter()
            .map(|p| p.as_variable_declaration())
            .collect();

        if target.arguments.len() <= source.parameter_identifiers().len()
            && target.arguments.len() >= source.required_parameter_identifiers().len()
        {
            self.check_parameter_compatibility(
                target.arguments.clone(),
                parameters.clone(),
                t,
                scope.clone(),
                no_self_declaration_type,
            )
        } else {
            false
        }
    }

    fn check_parameter_compatibility(
        &self,
        arguments: Vec<FunctionArgument>,
        parameters: Vec<VariableDeclaration>,
        enclosing: &TypeIdentifier,
        scope: ScopeContext,
        declared_types: Vec<Type>,
    ) -> bool {
        let mut index = 0;
        let mut argument_index = 0;

        let required_parameters = parameters.clone();
        let required_parameters: Vec<VariableDeclaration> = required_parameters
            .into_iter()
            .filter(|f| f.expression.is_none())
            .collect();

        while index < required_parameters.len() {
            if arguments[argument_index].identifier.is_some() {
                let argument_name = arguments[argument_index]
                    .identifier
                    .as_ref()
                    .unwrap()
                    .token
                    .clone();

                if argument_name != parameters[index].identifier.token {
                    return false;
                }
            } else {
                return false;
            }

            // Check Types
            let declared_type = declared_types[index].clone();
            let argument_expression = arguments[argument_index].expression.clone();
            let argument_type = self.get_expression_type(
                argument_expression,
                enclosing,
                vec![],
                vec![],
                scope.clone(),
            );

            if declared_type != argument_type {
                return false;
            }

            index += 1;
            argument_index += 1;
        }

        while index < required_parameters.len() && argument_index < arguments.len() {
            if arguments[argument_index].identifier.is_some() {
            } else {
                let declared_type = declared_types[index].clone();

                let argument_expression = arguments[argument_index].expression.clone();
                let argument_type = self.get_expression_type(
                    argument_expression,
                    enclosing,
                    vec![],
                    vec![],
                    scope.clone(),
                );
                //TODO replacing self
                if declared_type != argument_type {
                    return false;
                }
                index += 1;
                argument_index += 1;
                continue;
            }

            while index < parameters.len() {
                if arguments[argument_index].identifier.is_some() {
                    let argument_name = arguments[argument_index]
                        .identifier
                        .as_ref()
                        .unwrap()
                        .token
                        .clone();
                    if argument_name != parameters[index].identifier.token {
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
            let declared_type = declared_types[index].clone();
            let argument_expression = arguments[argument_index].expression.clone();
            let argument_type = self.get_expression_type(
                argument_expression,
                enclosing,
                vec![],
                vec![],
                scope.clone(),
            );

            if declared_type != argument_type {
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

    pub fn replace_self(list: Vec<Type>, enclosing: &TypeIdentifier) -> Vec<Type> {
        list.into_iter()
            .map(|t| t.replacing_self(enclosing))
            .collect()
    }
}
