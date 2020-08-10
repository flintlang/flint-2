use crate::ast::{mangle_function_move, FunctionCall, Property, Statement, Type};
use crate::context::Context;
use crate::environment::{CallableInformation, FunctionCallMatchResult};
use crate::solidity::preprocessor::*;

pub fn mangle_solidity_function_name(
    string: String,
    param_type: Vec<Type>,
    type_id: &str,
) -> String {
    let parameters: Vec<String> = param_type.into_iter().map(|p| p.name()).collect();
    let dollar = if parameters.is_empty() {
        "".to_string()
    } else {
        "$".to_string()
    };
    let parameters = parameters.join("_");

    format!(
        "{t}${name}{dollar}{parameters}",
        t = type_id,
        name = string,
        dollar = dollar,
        parameters = parameters
    )
}

pub fn mangle_function_call_name(function_call: &FunctionCall, ctx: &Context) -> Option<String> {
    if !is_ether_runtime_function_call(function_call) {
        let enclosing_type = if function_call.identifier.enclosing_type.is_some() {
            let enclosing = function_call.identifier.enclosing_type.clone();
            enclosing.unwrap()
        } else {
            let enclosing = ctx.enclosing_type_identifier().clone();
            let enclosing = enclosing.unwrap();
            enclosing.token.clone()
        };
        let call = function_call.clone();

        let caller_protections = if ctx.contract_behaviour_declaration_context.is_some() {
            let behaviour = ctx.contract_behaviour_declaration_context.clone();
            let behaviour = behaviour.unwrap();
            behaviour.caller_protections
        } else {
            vec![]
        };

        let scope = ctx.scope_context.as_ref().unwrap_or_default();

        let match_result =
            ctx.environment
                .match_function_call(&call, &enclosing_type, &caller_protections, scope);

        match match_result.clone() {
            FunctionCallMatchResult::MatchedFunction(fi) => {
                let declaration = fi.declaration.clone();
                let param_types = fi.get_parameter_types().clone();

                Some(mangle_solidity_function_name(
                    declaration.head.identifier.token.clone(),
                    param_types,
                    &enclosing_type,
                ))
            }
            FunctionCallMatchResult::MatchedFunctionWithoutCaller(c) => {
                if c.candidates.len() != 1 {
                    panic!("Unable to find function declaration")
                }

                let candidate = c.candidates.clone().remove(0);

                if let CallableInformation::FunctionInformation(fi) = candidate {
                    let declaration = fi.declaration;
                    let param_types = declaration.head.parameters;
                    let _param_types: Vec<Type> = param_types
                        .clone()
                        .into_iter()
                        .map(|p| p.type_assignment)
                        .collect();

                    Some(mangle_function_move(
                        &declaration.head.identifier.token,
                        &enclosing_type,
                        false,
                    ))
                } else {
                    panic!("Non-function CallableInformation where function expected")
                }
            }
            FunctionCallMatchResult::MatchedInitializer(i) => {
                let declaration = i.declaration.clone();
                let param_types = declaration.head.parameters.clone();
                let param_types: Vec<Type> =
                    param_types.into_iter().map(|p| p.type_assignment).collect();

                Some(mangle_solidity_function_name(
                    "init".to_string(),
                    param_types,
                    &function_call.identifier.token.clone(),
                ))
            }
            FunctionCallMatchResult::MatchedFallback(_) => unimplemented!(),
            FunctionCallMatchResult::MatchedGlobalFunction(fi) => {
                let param_types = fi.get_parameter_types().clone();

                Some(mangle_solidity_function_name(
                    function_call.identifier.token.clone(),
                    param_types,
                    &"Quartz_Global".to_string(),
                ))
            }
            FunctionCallMatchResult::Failure(_) => None,
        }
    } else {
        Some(function_call.identifier.token.clone())
    }
}

pub fn is_ether_runtime_function_call(function_call: &FunctionCall) -> bool {
    let ident = function_call.identifier.token.clone();
    ident.starts_with("Quartz$")
}

pub fn construct_expression(expressions: Vec<Expression>) -> Expression {
    let mut expression = expressions.clone();
    if expression.len() > 1 {
        let first = expression.remove(0);
        Expression::BinaryExpression(BinaryExpression {
            lhs_expression: Box::new(first),
            rhs_expression: Box::new(construct_expression(expression)),
            op: BinOp::Dot,
            line_info: Default::default(),
        })
    } else {
        expression.remove(0)
    }
}

pub fn construct_parameter(name: String, t: Type) -> Parameter {
    let identifier = Identifier {
        token: name,
        enclosing_type: None,
        line_info: Default::default(),
    };
    Parameter {
        identifier,
        type_assignment: t,
        expression: None,
        line_info: Default::default(),
    }
}

pub fn is_global_function_call(function_call: &FunctionCall, ctx: &Context) -> bool {
    let enclosing = ctx
        .enclosing_type_identifier()
        .map(|id| &*id.token)
        .unwrap_or_default();
    let caller_protections: &[_] =
        if let Some(ref behaviour) = ctx.contract_behaviour_declaration_context {
            &behaviour.caller_protections
        } else {
            &[]
        };

    let scope = ctx.scope_context.as_ref().unwrap_or_default();

    let result =
        ctx.environment
            .match_function_call(function_call, &enclosing, caller_protections, scope);

    if let FunctionCallMatchResult::MatchedGlobalFunction(_) = result {
        return true;
    }

    false
}

pub fn default_assignments(ctx: &Context) -> Vec<Statement> {
    let enclosing = ctx
        .enclosing_type_identifier()
        .map(|id| &*id.token)
        .unwrap_or_default();

    let properties_in_enclosing = ctx.environment.property_declarations(&enclosing);
    let properties_in_enclosing: Vec<Property> = properties_in_enclosing
        .into_iter()
        .filter(|p| p.get_value().is_some())
        .collect();

    properties_in_enclosing
        .into_iter()
        .map(|p| {
            let mut identifier = p.get_identifier();
            identifier.enclosing_type = Some(enclosing.to_string());
            Statement::Expression(Expression::BinaryExpression(BinaryExpression {
                lhs_expression: Box::new(Expression::Identifier(identifier)),
                rhs_expression: Box::new(p.get_value().unwrap()),
                op: BinOp::Equal,
                line_info: Default::default(),
            }))
        })
        .collect()
}
