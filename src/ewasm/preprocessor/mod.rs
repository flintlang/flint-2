mod utils;

use crate::ast::calls::{FunctionArgument, FunctionCall};
use crate::ast::declarations::{
    ContractBehaviourDeclaration, ContractBehaviourMember, FunctionDeclaration, Parameter,
    VariableDeclaration,
};
use crate::ast::expressions::{BinaryExpression, Expression, Identifier, InoutExpression};
use crate::ast::operators::BinOp;
use crate::ast::statements::{ReturnStatement, Statement};
use crate::ast::types::{InoutType, Type};
use crate::ast::{
    ContractDeclaration, ContractMember, Literal, Modifier, SpecialDeclaration,
    SpecialSignatureDeclaration, StructDeclaration, StructMember, VResult,
};
use crate::ast::{FunctionSignatureDeclaration, Property};
use crate::context::Context;
use crate::ewasm::preprocessor::utils::*;
use crate::utils::is_init_declaration;
use crate::visitor::Visitor;
use itertools::Itertools;

pub struct LLVMPreProcessor {}

impl Visitor for LLVMPreProcessor {
    fn start_contract_declaration(
        &mut self,
        dec: &mut ContractDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        // Add type states field to the contract
        if !dec.type_states.is_empty() {
            dec.contract_members
                .push(ContractMember::VariableDeclaration(
                    VariableDeclaration {
                        declaration_token: None,
                        identifier: Identifier::generated(Identifier::TYPESTATE_VAR_NAME),
                        variable_type: Type::TypeState,
                        expression: None,
                    },
                    None,
                ));
        }

        Ok(())
    }

    fn finish_contract_behaviour_declaration(
        &mut self,
        declaration: &mut ContractBehaviourDeclaration,
        ctx: &mut Context,
    ) -> VResult {
        // If we are in the declaration that contains the initialiser, then that is where we will insert the
        // getters and setters since there are no caller protections or type state restrictions
        // TODO the above explanation is somewhat hacky
        if declaration
            .members
            .iter()
            .any(|dec| is_init_declaration(dec))
        {
            let non_private_contract_members = ctx
                .environment
                .property_declarations(&declaration.identifier.token)
                .into_iter()
                // Some(_) ensures it has some modifier, and is therefore not private
                .filter(|property| property.get_modifier().is_some())
                .collect::<Vec<Property>>();

            for non_private_contract_member in non_private_contract_members {
                match non_private_contract_member.get_modifier().as_ref().unwrap() {
                    Modifier::Public => {
                        generate_and_add_getter(&non_private_contract_member, declaration, ctx);
                        generate_and_add_setter(&non_private_contract_member, declaration);
                    }
                    Modifier::Visible => {
                        generate_and_add_getter(&non_private_contract_member, declaration, ctx)
                    }
                }
            }
        }

        declaration.members = declaration
            .members
            .clone()
            .into_iter()
            .flat_map(|member| {
                if let ContractBehaviourMember::FunctionDeclaration(mut function) = member {
                    let wrapper = generate_contract_wrapper(&mut function, declaration, ctx);
                    let wrapper = ContractBehaviourMember::FunctionDeclaration(wrapper);
                    function.head.modifiers.retain(|x| x != &Modifier::Public);
                    vec![
                        ContractBehaviourMember::FunctionDeclaration(function),
                        wrapper,
                    ]
                } else {
                    vec![member]
                }
            })
            .collect();

        Ok(())
    }

    fn start_struct_declaration(
        &mut self,
        dec: &mut StructDeclaration,
        ctx: &mut Context,
    ) -> VResult {
        let vars_with_assignments = dec
            .members
            .iter()
            .filter_map(|m| {
                if let StructMember::VariableDeclaration(vd, _) = m {
                    if vd.expression.is_some() {
                        return Some(vd);
                    }
                }
                None
            })
            .collect::<Vec<&VariableDeclaration>>();

        let mut assignments = vec![];
        for default_assignment in vars_with_assignments {
            let assignment = BinaryExpression {
                lhs_expression: Box::new(Expression::Identifier(
                    default_assignment.identifier.clone(),
                )),
                rhs_expression: Box::new(*default_assignment.expression.clone().unwrap()),
                op: BinOp::Equal,
                line_info: Default::default(),
            };

            assignments.push(Statement::Expression(Expression::BinaryExpression(
                assignment,
            )));
        }

        for mut declaration in &mut dec.members {
            if let StructMember::SpecialDeclaration(sd) = &mut declaration {
                sd.head.parameters.push(Parameter {
                    identifier: Identifier::generated("this"),
                    type_assignment: Type::InoutType(InoutType {
                        key_type: Box::new(Type::UserDefinedType(dec.identifier.clone())),
                    }),
                    expression: None,
                    line_info: Default::default(),
                });
            }
        }

        if let Some(init) = ctx
            .environment
            .get_public_initialiser(dec.identifier.token.as_str())
        {
            init.body.extend(assignments);
        } else {
            // Create and add a public initialiser, and then push assignments to it
            let default_init = SpecialDeclaration {
                head: SpecialSignatureDeclaration {
                    special_token: "init".to_string(),
                    enclosing_type: dec.identifier.enclosing_type.clone(),
                    attributes: vec![],
                    modifiers: vec![Modifier::Public],
                    mutates: vec![],
                    parameters: vec![Parameter {
                        identifier: Identifier::generated("this"),
                        type_assignment: Type::InoutType(InoutType {
                            key_type: Box::new(Type::UserDefinedType(dec.identifier.clone())),
                        }),
                        expression: None,
                        line_info: Default::default(),
                    }],
                },
                body: assignments,
                scope_context: Default::default(),
                generated: false,
            };

            dec.members
                .push(StructMember::SpecialDeclaration(default_init.clone()));
            ctx.environment.add_special(
                default_init,
                dec.identifier.token.as_str(),
                vec![],
                vec![],
            );
        }

        Ok(())
    }

    fn start_function_declaration(
        &mut self,
        declaration: &mut FunctionDeclaration,
        ctx: &mut Context,
    ) -> VResult {
        let mangled_name = mangle_ewasm_function(
            &declaration.head.identifier.token,
            declaration.head.identifier.enclosing_type.as_ref().unwrap(),
        );

        declaration.mangled_identifier = Some(mangled_name);

        // construct self parameter for struct
        if let Some(ref struct_ctx) = ctx.struct_declaration_context {
            let self_param = construct_parameter(
                "this".to_string(),
                Type::InoutType(InoutType {
                    key_type: Box::new(Type::UserDefinedType(Identifier::generated(
                        &struct_ctx.identifier.token,
                    ))),
                }),
            );

            declaration.head.parameters.push(self_param);
            // TODO: add to scope?
        }
        // TODO: dynamic parameters?

        Ok(())
    }

    fn finish_function_declaration(
        &mut self,
        declaration: &mut FunctionDeclaration,
        _: &mut Context,
    ) -> VResult {
        if declaration.is_void() {
            if let Some(Statement::ReturnStatement(_)) = declaration.body.last() {
                return Ok(());
            } else {
                declaration
                    .body
                    .push(Statement::ReturnStatement(ReturnStatement {
                        expression: None,
                        ..Default::default()
                    }));
            }
        }

        Ok(())
    }

    fn start_special_declaration(
        &mut self,
        dec: &mut SpecialDeclaration,
        ctx: &mut Context,
    ) -> VResult {
        // Push default variable assignments to the initialiser
        if let Some(contract_name) = &dec.head.enclosing_type {
            let vars_with_assignments = &ctx
                .environment
                .types
                .get(contract_name)
                .unwrap()
                .properties
                .iter()
                .filter_map(|(_, p_info)| {
                    if let Property::VariableDeclaration(dec, _) = &p_info.property {
                        if dec.expression.is_some() {
                            return Some(dec);
                        }
                    }
                    None
                })
                .collect::<Vec<&VariableDeclaration>>();

            for default_assignment in vars_with_assignments {
                let assignment = BinaryExpression {
                    lhs_expression: Box::new(Expression::Identifier(
                        default_assignment.identifier.clone(),
                    )),
                    rhs_expression: Box::new(*default_assignment.expression.clone().unwrap()),
                    op: BinOp::Equal,
                    line_info: Default::default(),
                };

                dec.body
                    .push(Statement::Expression(Expression::BinaryExpression(
                        assignment,
                    )));
            }
        }

        Ok(())
    }

    fn finish_special_declaration(
        &mut self,
        declaration: &mut SpecialDeclaration,
        _: &mut Context,
    ) -> VResult {
        if let Some(Statement::ReturnStatement(_)) = declaration.body.last() {
            return Ok(());
        } else {
            declaration
                .body
                .push(Statement::ReturnStatement(ReturnStatement {
                    expression: None,
                    ..Default::default()
                }));
        }

        Ok(())
    }

    fn finish_statement(&mut self, statement: &mut Statement, ctx: &mut Context) -> VResult {
        if let Statement::BecomeStatement(bs) = statement {
            // MID we should be in a contract behaviour context since we are using type states
            let contract_name = &ctx
                .contract_behaviour_declaration_context
                .as_ref()
                .unwrap()
                .identifier
                .token;

            let declared_states = ctx.environment.get_contract_type_states(contract_name);
            // We immediately unwrap as all become statements should have been checked for having a declared typestate
            let type_state_as_u8 = declared_states
                .iter()
                .position(|state| state == &bs.state)
                .unwrap() as u8;

            let type_state_var_id = Identifier {
                token: Identifier::TYPESTATE_VAR_NAME.to_string(),
                enclosing_type: Some(contract_name.to_string()),
                line_info: Default::default(),
            };

            let state_variable = Expression::BinaryExpression(BinaryExpression {
                lhs_expression: Box::new(Expression::SelfExpression),
                rhs_expression: Box::new(Expression::Identifier(type_state_var_id)),
                op: BinOp::Dot,
                line_info: Default::default(),
            });

            *statement = Statement::Expression(Expression::BinaryExpression(BinaryExpression {
                lhs_expression: Box::new(state_variable),
                rhs_expression: Box::new(Expression::Literal(Literal::U8Literal(type_state_as_u8))),
                op: BinOp::Equal,
                line_info: Default::default(),
            }));
        }

        Ok(())
    }

    fn start_binary_expression(
        &mut self,
        expr: &mut BinaryExpression,
        ctx: &mut Context,
    ) -> VResult {
        // Removes assignment shorthand expressions, e.g. += and *=
        if expr.op.is_assignment_shorthand() {
            let op = expr.op.get_assignment_shorthand();
            expr.op = BinOp::Equal;

            let rhs = BinaryExpression {
                lhs_expression: expr.lhs_expression.clone(),
                rhs_expression: expr.rhs_expression.clone(),
                op,
                line_info: expr.line_info.clone(),
            };

            expr.rhs_expression = Box::from(Expression::BinaryExpression(rhs));
        } else if expr.op == BinOp::Dot {
            if let Some(scope_ctx) = &mut ctx.scope_context {
                if let Expression::Identifier(id) = *expr.lhs_expression.clone() {
                    if !scope_ctx.is_declared(&id.token) {
                        let rhs = BinaryExpression {
                            lhs_expression: expr.lhs_expression.clone(),
                            rhs_expression: expr.rhs_expression.clone(),
                            op: BinOp::Dot,
                            line_info: expr.line_info.clone(),
                        };

                        expr.lhs_expression = Box::from(Expression::SelfExpression);
                        expr.rhs_expression = Box::from(Expression::BinaryExpression(rhs));
                        // TODO: check if this is the correct local variable to add
                        scope_ctx.local_variables.push(VariableDeclaration {
                            declaration_token: None,
                            identifier: id,
                            variable_type: Type::Int,
                            expression: None,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn start_function_call(&mut self, call: &mut FunctionCall, ctx: &mut Context) -> VResult {
        let function_name = &call.identifier.token;
        if ctx.environment.types.get(function_name).is_some() {
            call.identifier.token = format!("{}Init", function_name);

            call.arguments.push(FunctionArgument {
                identifier: None,
                expression: Expression::InoutExpression(InoutExpression {
                    ampersand_token: "&".to_string(),
                    expression: Box::new(Expression::Identifier(Identifier::generated("tmp_var"))),
                }),
            });
        } else {
            // Gets the contract name / struct name
            let enclosing_type = if let Some(enclosing) = &call.identifier.enclosing_type {
                enclosing
            } else {
                ctx.enclosing_type_identifier().unwrap().token.as_str()
            };

            // mangles name
            call.identifier.token = mangle_ewasm_function(&function_name, enclosing_type);

            // Pass in the parameter for the function to operate on. If it is a struct function,
            // it should be an instance of that struct. Otherwise it will be the contract variable
            let contract_argument = if ctx.function_call_receiver_trail.is_empty() {
                FunctionArgument {
                    identifier: None,
                    expression: Expression::Identifier(Identifier::generated("this")),
                }
            } else {
                FunctionArgument {
                    identifier: None,
                    expression: Expression::InoutExpression(InoutExpression {
                        ampersand_token: "&".to_string(),
                        expression: Box::from(
                            ctx.function_call_receiver_trail
                                .clone()
                                .into_iter()
                                .fold1(|lhs, next| {
                                    Expression::BinaryExpression(BinaryExpression {
                                        lhs_expression: Box::new(lhs),
                                        rhs_expression: Box::new(next),
                                        op: BinOp::Dot,
                                        line_info: Default::default(),
                                    })
                                })
                                .unwrap(),
                        ),
                    }),
                }
            };

            call.arguments.push(contract_argument);
        }

        Ok(())
    }
}

// TODO abstract this out as it is almost identical to that created in move preprocessor
fn generate_and_add_getter(
    member: &Property,
    behaviour_declaration: &mut ContractBehaviourDeclaration,
    ctx: &mut Context,
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

    let mangled_name = mangle_ewasm_function(
        getter_name.as_str(),
        behaviour_declaration.identifier.token.as_str(),
    );

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

    let setter_signature = FunctionSignatureDeclaration {
        func_token: "func".to_string(),
        attributes: vec![],
        modifiers: vec![Modifier::Public],
        mutates: vec![member_identifier],
        identifier: Identifier::generated(&setter_name),
        parameters: vec![parameter],
        result_type: None,
        payable: false,
    };

    let setter_declaration = FunctionDeclaration {
        head: setter_signature,
        body: vec![assignment],
        scope_context: Some(Default::default()),
        tags: vec![],
        mangled_identifier: None,
        is_external: false,
    };

    behaviour_declaration
        .members
        .push(ContractBehaviourMember::FunctionDeclaration(
            setter_declaration,
        ));
}
