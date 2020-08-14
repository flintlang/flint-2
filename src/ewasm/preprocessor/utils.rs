use crate::ast::calls::FunctionCall;
use crate::ast::declarations::Parameter;
use crate::ast::expressions::Identifier;
use crate::ast::types::Type;
use crate::ast::expressions::Expression;
use crate::ast::calls::FunctionArgument;
use crate::ast::statements::{Statement, ReturnStatement};
use crate::ast::declarations::{ContractBehaviourDeclaration, FunctionDeclaration, VariableDeclaration};
use crate::context::Context;

pub fn generate_contract_wrapper(
    function: &mut FunctionDeclaration,
    contract_behaviour_declaration: &ContractBehaviourDeclaration,
    _ctx: &mut Context,
) -> FunctionDeclaration {
    let mut wrapper = function.clone();
    wrapper.mangled_identifier = Option::from(mangle_ewasm_function(
        &function.head.identifier.token));

    wrapper.body = vec![];
    
    // TODO: does this add the return statement?
    if !function.is_void() && !function.body.is_empty() {
        let mut func = function.clone();
        wrapper.body.push(func.body.remove(0));
    }

    let self_declaration = VariableDeclaration {
        declaration_token: None,
        identifier: Identifier::generated(Identifier::SELF),
        variable_type: Type::UserDefinedType(contract_behaviour_declaration.identifier.clone()),
        expression: None,
    };

    wrapper
        .body
        .push(Statement::Expression(Expression::VariableDeclaration(
            self_declaration,
        )));

    // TODO: caller protections
    // TODO: type states

    let contract_parameter = Parameter {
        identifier: Identifier::generated("this"),
        type_assignment: Type::UserDefinedType(contract_behaviour_declaration.identifier.clone()),
        expression: None,
        line_info: Default::default()
    };

    function.head.parameters.push(contract_parameter);
 
    let arguments: Vec<FunctionArgument> = function
        .head
        .parameters
        .clone()
        .into_iter()
        .map(|p| FunctionArgument {
            identifier: None,
            expression: Expression::Identifier(p.identifier),
        })
        .collect();

    let name = function.mangled_identifier.clone();
    let function_call = Expression::FunctionCall(FunctionCall {
        identifier: Identifier {
            token: name.unwrap_or_default(),
            enclosing_type: None,
            line_info: Default::default(),
        },
        arguments,
        mangled_identifier: None,
    });

    if function.is_void() {
        wrapper
            .body
            .push(Statement::Expression(function_call.clone()))
    }

    wrapper
        .body
        .push(Statement::ReturnStatement(ReturnStatement {
            expression: {
                if function.is_void() {
                    None
                } else {
                    Some(function_call)
                }
            },
            ..Default::default()
        }));

    wrapper
}

pub fn mangle_ewasm_function(_function_name: &str) -> String {
    unimplemented!();
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

pub fn is_ether_runtime_function_call(function_call: &FunctionCall) -> bool {
    let ident = function_call.identifier.token.clone();
    ident.starts_with("Quartz$")
}
