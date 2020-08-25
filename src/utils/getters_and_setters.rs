use crate::ast::{
    BinOp, BinaryExpression, ContractBehaviourDeclaration, ContractBehaviourMember, Expression,
    FunctionDeclaration, FunctionSignatureDeclaration, Identifier, Modifier, Parameter, Property,
    ReturnStatement, Statement,
};
use crate::context::Context;

pub fn generate_and_add_getters_and_setters(behaviour_declaration: &mut ContractBehaviourDeclaration, ctx: &mut Context, mangler: &dyn Fn(&str) -> String) {
    let non_private_contract_members = ctx
        .environment
        .property_declarations(&behaviour_declaration.identifier.token)
        .into_iter()
        // Some(_) ensures it has some modifier, and is therefore not private
        .filter(|property| property.get_modifier().is_some())
        .collect::<Vec<Property>>();

    for non_private_contract_member in non_private_contract_members {
        match non_private_contract_member.get_modifier().as_ref().unwrap() {
            Modifier::Public => {
                generate_and_add_getter(
                    &non_private_contract_member,
                    behaviour_declaration,
                    ctx,
                    &mangler,
                );
                generate_and_add_setter(
                    &non_private_contract_member,
                    behaviour_declaration,
                    ctx,
                    &mangler,
                );
            }
            Modifier::Visible => generate_and_add_getter(
                &non_private_contract_member,
                behaviour_declaration,
                ctx,
                &mangler,
            ),
        }
    }
}

fn generate_and_add_getter(
    member: &Property,
    behaviour_declaration: &mut ContractBehaviourDeclaration,
    ctx: &mut Context,
    mangler: &dyn Fn(&str) -> String,
) {
    let mut member_identifier = member.get_identifier();
    member_identifier.enclosing_type = Some(behaviour_declaration.identifier.token.clone());

    // converts the name to start with a capital, so value becomes getValue
    let getter_name = format!(
        "get{}{}",
        member_identifier
            .token
            .chars()
            .next()
            .unwrap()
            .to_ascii_uppercase(),
        member_identifier.token.chars().skip(1).collect::<String>()
    );

    let member_type = member.get_type();

    let return_statement = Statement::ReturnStatement(ReturnStatement {
        expression: Some(Expression::BinaryExpression(BinaryExpression {
            lhs_expression: Box::new(Expression::SelfExpression),
            rhs_expression: Box::new(Expression::Identifier(member_identifier)),
            op: BinOp::Dot,
            line_info: Default::default(),
        })),
        cleanup: vec![],
        line_info: Default::default(),
    });

    let mangled_name = mangler(getter_name.as_str());

    let getter_signature = FunctionSignatureDeclaration {
        func_token: "func".to_string(),
        attributes: vec![],
        modifiers: vec![Modifier::Public],
        mutates: vec![],
        identifier: Identifier {
            token: getter_name,
            enclosing_type: Some(behaviour_declaration.identifier.token.clone()),
            line_info: Default::default(),
        },
        parameters: vec![],
        result_type: Some(member_type),
        payable: false,
    };

    let getter = FunctionDeclaration {
        head: getter_signature,
        body: vec![return_statement],
        scope_context: Some(Default::default()),
        tags: vec![],
        mangled_identifier: Some(mangled_name),
        is_external: false,
    };

    behaviour_declaration
        .members
        .push(ContractBehaviourMember::FunctionDeclaration(getter.clone()));

    ctx.environment.add_function(
        getter,
        &behaviour_declaration.identifier.token,
        vec![], // These should be empty anyway as we should only make getters and setters
        vec![], // In restriction free zones
    );
}

fn generate_and_add_setter(
    member: &Property,
    behaviour_declaration: &mut ContractBehaviourDeclaration,
    ctx: &mut Context,
    mangler: &dyn Fn(&str) -> String,
) {
    let member_identifier = member.get_identifier();

    // converts the name to start with a capital, so value becomes setValue
    let setter_name = format!(
        "set{}{}",
        member_identifier
            .token
            .chars()
            .next()
            .unwrap()
            .to_ascii_uppercase(),
        member_identifier.token.chars().skip(1).collect::<String>()
    );

    let parameter_identifier = Identifier::generated(member_identifier.token.as_str());
    let parameter = Parameter {
        identifier: parameter_identifier.clone(),
        type_assignment: member.get_type(),
        expression: None,
        line_info: Default::default(),
    };

    let assignment = BinaryExpression {
        lhs_expression: Box::new(Expression::BinaryExpression(BinaryExpression {
            lhs_expression: Box::new(Expression::SelfExpression),
            rhs_expression: Box::new(Expression::Identifier(member_identifier.clone())),
            op: BinOp::Dot,
            line_info: Default::default(),
        })),
        rhs_expression: Box::new(Expression::Identifier(parameter_identifier)),
        op: BinOp::Equal,
        line_info: Default::default(),
    };

    let assignment = Statement::Expression(Expression::BinaryExpression(assignment));
    let return_statement = Statement::ReturnStatement(ReturnStatement {
        expression: None,
        cleanup: vec![],
        line_info: Default::default(),
    });

    let mangled_name = mangler(setter_name.as_str());

    let setter_signature = FunctionSignatureDeclaration {
        func_token: "func".to_string(),
        attributes: vec![],
        modifiers: vec![Modifier::Public],
        mutates: vec![member_identifier],
        identifier: Identifier {
            token: setter_name,
            enclosing_type: Some(behaviour_declaration.identifier.token.clone()),
            line_info: Default::default(),
        },
        parameters: vec![parameter],
        result_type: None,
        payable: false,
    };

    let setter_declaration = FunctionDeclaration {
        head: setter_signature,
        body: vec![assignment, return_statement],
        scope_context: Some(Default::default()),
        tags: vec![],
        mangled_identifier: Some(mangled_name),
        is_external: false,
    };

    behaviour_declaration
        .members
        .push(ContractBehaviourMember::FunctionDeclaration(
            setter_declaration.clone(),
        ));

    ctx.environment.add_function(
        setter_declaration,
        &behaviour_declaration.identifier.token,
        vec![], // These should be empty anyway as we should only make getters and setters
        vec![], // In restriction free zones
    );
}
