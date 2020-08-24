use crate::ast::calls::FunctionArgument;
use crate::ast::calls::FunctionCall;
use crate::ast::declarations::Parameter;
use crate::ast::declarations::{ContractBehaviourDeclaration, FunctionDeclaration};
use crate::ast::expressions::Expression;
use crate::ast::expressions::Identifier;
use crate::ast::statements::{ReturnStatement, Statement};
use crate::ast::types::Type;
use crate::ast::{Assertion, BinOp, BinaryExpression, InoutType, VariableDeclaration};
use crate::context::Context;
use crate::utils::type_states::{extract_allowed_states, generate_type_state_condition};

pub fn generate_contract_wrapper(
    function: &mut FunctionDeclaration,
    contract_behaviour_declaration: &ContractBehaviourDeclaration,
    ctx: &mut Context,
) -> FunctionDeclaration {
    let mut wrapper = function.clone();
    wrapper.is_external = true;
    wrapper.mangled_identifier = None;
    wrapper.body = vec![];

    let contract_name = contract_behaviour_declaration.identifier.token.as_str();

    // Add type state assertions
    if !contract_behaviour_declaration.type_states.is_empty() {
        let type_state_var = BinaryExpression {
            lhs_expression: Box::new(Expression::Identifier(Identifier::generated(contract_name))),
            rhs_expression: Box::new(Expression::Identifier(Identifier::generated(
                Identifier::TYPESTATE_VAR_NAME,
            ))),
            op: BinOp::Dot,
            line_info: Default::default(),
        };

        let type_state_var = VariableDeclaration {
            declaration_token: None,
            identifier: Identifier::generated(Identifier::TYPESTATE_VAR_NAME),
            variable_type: Type::TypeState,
            expression: Some(Box::from(Expression::BinaryExpression(type_state_var))),
        };

        wrapper
            .body
            .push(Statement::Expression(Expression::VariableDeclaration(
                type_state_var,
            )));

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
        type_assignment: Type::InoutType(InoutType {
            key_type: Box::new(Type::UserDefinedType(Identifier::generated(contract_name))),
        }),
        expression: None,
        line_info: Default::default(),
    };

    let mut arguments = function
        .head
        .parameters
        .clone()
        .into_iter()
        .map(|p| FunctionArgument {
            identifier: None,
            expression: Expression::Identifier(p.identifier),
        })
        .collect::<Vec<FunctionArgument>>();

    arguments.push(FunctionArgument {
        identifier: None,
        expression: Expression::Identifier(Identifier::generated(contract_name)),
    });

    function.head.parameters.push(contract_parameter);

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
    format!("inner_{}", function_name)
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
