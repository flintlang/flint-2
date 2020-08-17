use crate::ast::calls::FunctionArgument;
use crate::ast::calls::FunctionCall;
use crate::ast::declarations::Parameter;
use crate::ast::declarations::{
    ContractBehaviourDeclaration, FunctionDeclaration, VariableDeclaration,
};
use crate::ast::expressions::Expression;
use crate::ast::expressions::Identifier;
use crate::ast::statements::{ReturnStatement, Statement};
use crate::ast::types::Type;
use crate::ast::Assertion;
use crate::context::Context;
use crate::utils::type_states::{extract_allowed_states, generate_type_state_condition};

pub fn generate_contract_wrapper(
    function: &mut FunctionDeclaration,
    contract_behaviour_declaration: &ContractBehaviourDeclaration,
    ctx: &mut Context,
) -> FunctionDeclaration {
    let mut wrapper = function.clone();
    wrapper.mangled_identifier =
        Option::from(mangle_ewasm_function(&function.head.identifier.token));

    wrapper.body = vec![];

    let self_declaration = VariableDeclaration {
        declaration_token: None,
        identifier: Identifier::generated("this"),
        variable_type: Type::UserDefinedType(contract_behaviour_declaration.identifier.clone()),
        expression: Some(Box::new(Expression::Identifier(Identifier::generated(
            contract_behaviour_declaration.identifier.token.as_str(),
        )))),
    };

    // give variable declaration an expression

    wrapper
        .body
        .push(Statement::Expression(Expression::VariableDeclaration(
            self_declaration,
        )));

    // Add type state assertions
    if !contract_behaviour_declaration.type_states.is_empty() {
        let contract_name = contract_behaviour_declaration.identifier.token.as_str();
        let allowed_type_states_as_u8s = extract_allowed_states(
            &contract_behaviour_declaration.type_states,
            &ctx.environment.get_contract_type_states(contract_name),
        )
        .collect::<Vec<u8>>();

        let condition = generate_type_state_condition(
            Identifier::generated(Identifier::TYPESTATE_VAR_NAME),
            &allowed_type_states_as_u8s,
        );

        wrapper.body.push(Statement::Assertion(Assertion {
            expression: Expression::BinaryExpression(condition),
            line_info: contract_behaviour_declaration.identifier.line_info.clone(),
        }))
    }

    // TODO: caller protections

    let contract_parameter = Parameter {
        identifier: Identifier::generated("this"),
        type_assignment: Type::UserDefinedType(contract_behaviour_declaration.identifier.clone()),
        expression: None,
        line_info: Default::default(),
    };

    function.head.parameters.push(contract_parameter);

    let arguments = function
        .head
        .parameters
        .clone()
        .into_iter()
        .map(|p| FunctionArgument {
            identifier: None,
            expression: Expression::Identifier(p.identifier),
        })
        .collect::<Vec<FunctionArgument>>();

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
        wrapper.body.push(Statement::Expression(function_call));
        wrapper
            .body
            .push(Statement::ReturnStatement(ReturnStatement {
                expression: None,
                ..Default::default()
            }));
    } else {
        wrapper
            .body
            .push(Statement::ReturnStatement(ReturnStatement {
                expression: Some(function_call),
                ..Default::default()
            }));
    }

    wrapper
}

pub fn mangle_ewasm_function(function_name: &str) -> String {
    // TODO implement properly
    function_name.to_string()
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
