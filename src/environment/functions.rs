use crate::ast::{
    CallerProtection, FunctionArgument, FunctionCall, FunctionDeclaration, FunctionInformation,
    FunctionSignatureDeclaration, Type, TypeInfo, TypeState,
};
use crate::context::ScopeContext;
use crate::environment::*;
use crate::type_checker::ExpressionChecker;
use itertools::{EitherOrBoth, Itertools};

impl Environment {
    pub fn add_function(
        &mut self,
        f: FunctionDeclaration,
        type_id: &str,
        caller_protections: Vec<CallerProtection>,
        type_states: Vec<TypeState>,
    ) {
        let name = &f.head.identifier.token;
        let mutating = f.is_mutating();
        let type_info = if let Some(type_info) = self.types.get_mut(type_id) {
            type_info
        } else {
            self.types.insert(type_id.to_string(), TypeInfo::new());
            self.types.get_mut(type_id).unwrap()
        };

        // Allow further references without premature moving or code duplication
        let function_information = |f| FunctionInformation {
            declaration: f,
            mutating,
            caller_protections,
            type_states,
            ..Default::default()
        };

        if let Some(function_set) = type_info.functions.get_mut(name) {
            function_set.push(function_information(f));
        } else {
            type_info
                .functions
                .insert(name.clone(), vec![function_information(f)]);
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

        if let Some(type_info) = self.types.get(type_id) {
            if let Some(functions) = type_info.all_functions().get(&call.identifier.token) {
                for function in functions {
                    if self.function_call_arguments_compatible(function, call, type_id, scope)
                        && compatible_caller_protections(protections, &function.caller_protections)
                    {
                        return FunctionCallMatchResult::MatchedFunction(function.clone());
                    }

                    candidates.push(function.clone());
                    continue;
                }
            }
        }

        let argument_types: Vec<_> = self.argument_types(call, type_id, scope).collect();

        let matched_candidates: Vec<FunctionInformation> = candidates
            .clone()
            .into_iter()
            .filter(|c| {
                c.get_parameter_types().eq(argument_types.iter())
                    && compatible_caller_protections(protections, &c.caller_protections)
            })
            .collect();

        let matched_candidates: Vec<CallableInformation> = matched_candidates
            .into_iter()
            .map(CallableInformation::FunctionInformation)
            .collect();

        if !matched_candidates.is_empty() {
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
                let equal_types = parameter_types == argument_types;

                if equal_types
                    && compatible_caller_protections(protections, &initialiser.caller_protections)
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
        if let Some(type_info) = self.types.get(crate::environment::FLINT_GLOBAL) {
            if let Some(functions) = type_info.functions.get(&call.identifier.token) {
                for function in functions {
                    let parameter_types: Vec<_> = function.get_parameter_types().collect();
                    let equal_types = argument_types
                        .iter()
                        .all(|argument_type| parameter_types.contains(&argument_type));
                    if equal_types
                        && compatible_caller_protections(protections, &function.caller_protections)
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

    pub fn argument_types<'a>(
        &'a self,
        call: &'a FunctionCall,
        type_id: &'a str,
        scope: &'a ScopeContext,
    ) -> impl Iterator<Item = Type> + 'a {
        call.arguments
            .iter()
            .map(move |a| self.get_expression_type(&a.expression, type_id, &[], &[], scope))
    }

    pub fn is_runtime_function_call(function_call: &FunctionCall) -> bool {
        function_call
            .identifier
            .token
            .starts_with(FLINT_RUNTIME_PREFIX)
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

        let initialiser_match =
            self.match_initialiser_function(call, &argument_types, caller_protections);
        let global_match = self.match_global_function(call, &argument_types, caller_protections);

        result
            .merge(regular_match)
            .merge(initialiser_match)
            .merge(global_match)
    }

    fn function_call_arguments_compatible(
        &self,
        source: &FunctionInformation,
        target: &FunctionCall,
        type_id: &str,
        scope: &ScopeContext,
    ) -> bool {
        let no_self_declaration_type: Vec<_> =
            Environment::replace_self(source.get_parameter_types(), type_id).collect();

        let parameters: &[_] = &source.declaration.head.parameters;

        if target.arguments.len() <= source.parameter_identifiers().count()
            && target.arguments.len() >= source.required_parameter_identifiers().count()
        {
            self.check_parameter_compatibility(
                &target.arguments,
                parameters,
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
        parameters: &[Parameter],
        enclosing: &str,
        scope: &ScopeContext,
        declared_types: &[Type],
    ) -> bool {
        // Cannot use itertools::izip! as it's important to consume parameters, to ensure no required parameters are left hanging
        for element in parameters
            .iter()
            .zip_longest(arguments.iter().zip(declared_types))
        {
            match element {
                EitherOrBoth::Both(parameter, (argument, declared_type)) => {
                    if !matches!(argument.identifier, Some(ref argument) if argument.token == parameter.identifier.token)
                    {
                        return false;
                    }

                    let argument_type =
                        self.get_expression_type(&argument.expression, enclosing, &[], &[], &scope);

                    if argument_type != *declared_type {
                        return false;
                    }
                }
                EitherOrBoth::Left(parameter) if parameter.expression.is_some() => (),
                _ => return false,
            }
        }
        true
    }

    pub fn replace_self<'a, I: 'a + Iterator<Item = &'a Type>>(
        iterator: I,
        enclosing: &'a str,
    ) -> impl Iterator<Item = Type> + 'a {
        iterator.map(move |t| t.replacing_self(enclosing))
    }
}

pub fn compatible_caller_protections(
    source: &[CallerProtection],
    target: &[CallerProtection],
) -> bool {
    // each caller protection in the source must match at least one caller protection in the target

    for caller_protection in source {
        let mut matched_any_parent = target.is_empty();

        for parent in target {
            matched_any_parent |= caller_protection.is_sub_protection(parent);
        }

        if !matched_any_parent {
            return false;
        }
    }

    true
}
