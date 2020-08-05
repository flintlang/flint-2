use super::expression::MoveExpression;
use super::function::FunctionContext;
use super::identifier::MoveIdentifier;
use super::ir::{MoveIRExpression, MoveIRFunctionCall};
use crate::ast::{Expression, ExternalCall, FunctionCall, Identifier, CallerProtection};
use crate::ast::calls::FunctionArgument;
use crate::environment::{CallableInformation, FunctionCallMatchResult};

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
                &lookup,
                &enclosing,
                &[],
                &function_context.scope_context,
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
        let mut arguments = self.function_call.arguments.clone();

        if let Some(context) = function_context.environment.types.get(
            &function_context
                .environment
                .contract_declarations
                .get(0)
                .unwrap()
                .token,
        ) {
            if let Some(function_call) = context.functions.get(&self.function_call.identifier.token)
            {
                if !function_call.get(0).unwrap().caller_protections.is_empty()
                    && !caller_protections_is_any(&function_call.get(0).unwrap().caller_protections)
                    && !contains_caller_argument(&arguments)
                {
                    arguments.push(FunctionArgument {
                        identifier: None,
                        expression: Expression::Identifier(Identifier {
                            token: function_context.scope_context.parameters.clone().pop().unwrap().identifier.token,
                            enclosing_type: None,
                            line_info: Default::default(),
                        }),
                    });
                }
            }
        }

        let arguments: Vec<MoveIRExpression> = arguments
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

fn contains_caller_argument(arguments: &Vec<FunctionArgument>) -> bool {
    !arguments
        .into_iter()
        .filter(|argument| {
            if let Some(identifier) = argument.identifier.as_ref() {
                identifier.token == "caller"
            } else {
                false
            }
        })
        .collect::<Vec<&FunctionArgument>>()
        .is_empty()
}

fn caller_protections_is_any(caller_protections: &Vec<CallerProtection>) -> bool {
    caller_protections
        .into_iter()
        .filter(|caller_protection| caller_protection.identifier.token != "any")
        .collect::<Vec<&CallerProtection>>()
        .is_empty()
}
