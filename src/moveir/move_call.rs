use crate::moveir::*;

pub struct MoveExternalCall {
    pub external_call: ExternalCall,
}

impl MoveExternalCall {
    pub fn generate(&self, function_context: &FunctionContext) -> MoveIRExpression {
        if let Expression::FunctionCall(f) =
            *self.external_call.function_call.rhs_expression.clone()
        {
            let mut lookup = f.clone();
            if !lookup.arguments.is_empty() {
                lookup.arguments.remove(0);
            }
            let enclosing = f.identifier.enclosing_type.clone();
            let enclosing = enclosing.unwrap_or(function_context.enclosing_type.clone());

            let result = function_context.environment.match_function_call(
                lookup,
                &enclosing,
                vec![],
                function_context.scope_context.clone(),
            );

            if let FunctionCallMatchResult::MatchedFunction(_) = result {
            } else if let FunctionCallMatchResult::Failure(c) = result {
                let candidate = c.clone();
                let mut candidate = candidate.candidates.clone();
                if candidate.is_empty() {
                    panic!("Cannot match function signature of external call")
                } else {
                    let candidate = candidate.remove(0);

                    if let CallableInformation::FunctionInformation(_) = candidate {
                    } else {
                        panic!("Cannot match function signature of external call")
                    }
                }
            } else {
                panic!("Cannot match function signature of external call")
            }

            if self.external_call.external_trait_name.is_some() {
                let external_trait_name = self.external_call.external_trait_name.clone();
                let external_trait_name = external_trait_name.unwrap();

                let type_info = function_context.environment.types.get(&external_trait_name);

                if type_info.is_some() {
                    let type_info = type_info;
                    let type_info = type_info.unwrap();

                    if type_info.is_external_module() {
                        return MoveFunctionCall {
                            function_call: f.clone(),
                            module_name: external_trait_name,
                        }
                        .generate(function_context);
                    }
                }
            }

            let mut function_call = f.clone();

            if self.external_call.external_trait_name.is_some() {
                let external_trait_name = self.external_call.external_trait_name.clone();
                let external_trait_name = external_trait_name.unwrap();
                let ident = function_call.mangled_identifier.clone();
                let ident = ident.unwrap_or(function_call.identifier.clone());
                let ident = ident.token;
                function_call.mangled_identifier = Option::from(Identifier {
                    token: format!("{ext}_{i}", ext = external_trait_name, i = ident),
                    enclosing_type: None,
                    line_info: Default::default(),
                });
            }

            MoveFunctionCall {
                function_call,
                module_name: "Self".to_string(),
            }
            .generate(function_context)
        } else {
            panic!("Cannot match external call with function")
        }
    }
}

pub struct MoveFunctionCall {
    pub function_call: FunctionCall,
    pub module_name: String,
}

impl MoveFunctionCall {
    pub fn generate(&self, function_context: &FunctionContext) -> MoveIRExpression {
        let mut look_up = self.function_call.clone();
        if !self.function_call.arguments.is_empty() {
            let mut args = self.function_call.arguments.clone();
            let arg1 = args.remove(0);
            let expression = arg1.expression;
            if let Expression::SelfExpression = expression {
                look_up.arguments = args;
            }
        }

        let mut module = self.module_name.clone();
        let mut call = if self.function_call.mangled_identifier.is_some() {
            let mangled = self.function_call.mangled_identifier.clone();
            let mangled = mangled.unwrap();

            mangled.token
        } else {
            self.function_call.identifier.token.clone()
        };

        if function_context
            .environment
            .is_trait_declared(&self.function_call.identifier.token)
        {
            let type_info = function_context
                .environment
                .types
                .get(&self.function_call.identifier.token);
            if type_info.is_some() {
                let type_info = type_info.unwrap();
                if type_info.is_external_struct() {
                    if type_info.is_external_module() {
                        module = look_up.identifier.token.clone();
                        call = "new".to_string();
                    }
                } else {
                    let external_address = look_up.arguments.remove(0).expression;
                    return MoveExpression {
                        expression: external_address,
                        position: Default::default(),
                    }
                    .generate(function_context);
                }
            }
        }

        let arguments: Vec<MoveIRExpression> = self
            .function_call
            .arguments
            .clone()
            .into_iter()
            .map(|a| {
                if let Expression::Identifier(i) = a.expression.clone() {
                    MoveIdentifier {
                        identifier: i,
                        position: Default::default(),
                    }
                    .generate(function_context, false, true)
                } else {
                    MoveExpression {
                        expression: a.expression,
                        position: Default::default(),
                    }
                    .generate(function_context)
                }
            })
            .collect();
        let identifier = format!("{module}.{function}", module = module, function = call);
        MoveIRExpression::FunctionCall(MoveIRFunctionCall {
            identifier,
            arguments,
        })
    }
}