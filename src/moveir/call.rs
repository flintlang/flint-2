use crate::ast::{ExternalCall, Expression, Identifier, FunctionCall};
use super::function::FunctionContext;
use super::ir::{MoveIRExpression, MoveIRFunctionCall};
use crate::environment::{FunctionCallMatchResult, CallableInformation};
use super::expression::MoveExpression;
use super::identifier::MoveIdentifier;

pub(crate) struct MoveExternalCall {
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
            let enclosing = if let Some(ref enclosing) = f.identifier.enclosing_type {
                enclosing
            } else {
                &function_context.enclosing_type
            };

            let result = function_context.environment.match_function_call(
                lookup,
                &enclosing,
                vec![],
                function_context.scope_context.clone(),
            );

            if let FunctionCallMatchResult::MatchedFunction(_) = result {
            } else if let FunctionCallMatchResult::Failure(candidate) = result {
                let mut candidate = candidate.candidates;
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

            if let Some(ref external_trait_name) = self.external_call.external_trait_name {
                if let Some(type_info) = function_context.environment.types.get(external_trait_name)
                {
                    if type_info.is_external_module() {
                        return MoveFunctionCall {
                            function_call: f,
                            module_name: external_trait_name.clone(),
                        }
                            .generate(function_context);
                    }
                }
            }

            let mut function_call = f.clone();

            if let Some(ref external_trait_name) = self.external_call.external_trait_name {
                let ident = &if let Some(ref mangled) = function_call.mangled_identifier {
                    mangled
                } else {
                    &function_call.identifier
                }
                    .token;
                function_call.mangled_identifier = Option::from(Identifier {
                    token: format!("{ext}_{i}", ext = *external_trait_name, i = *ident),
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

pub(crate) struct MoveFunctionCall {
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

        let mut module: &str = &self.module_name;
        let mut call: &str = if let Some(ref mangled) = self.function_call.mangled_identifier {
            &mangled.token
        } else {
            &self.function_call.identifier.token
        };

        if function_context
            .environment
            .is_trait_declared(&self.function_call.identifier.token)
        {
            if let Some(type_info) = function_context
                .environment
                .types
                .get(&self.function_call.identifier.token)
            {
                if type_info.is_external_struct() {
                    if type_info.is_external_module() {
                        module = &look_up.identifier.token;
                        call = "new";
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
