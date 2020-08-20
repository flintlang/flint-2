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
use crate::ast::Property;
use crate::ast::{
    ContractDeclaration, ContractMember, Literal, Modifier, SpecialDeclaration,
    SpecialSignatureDeclaration, StructDeclaration, StructMember, VResult,
};
use crate::context::Context;
use crate::ewasm::preprocessor::utils::*;
use crate::visitor::Visitor;

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
                        type_assignment: Type::UserDefinedType(dec.identifier.clone()),
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
        let mangled_name = mangle_ewasm_function(&declaration.head.identifier.token);

        declaration.mangled_identifier = Some(mangled_name);

        // construct self parameter for struct
        if let Some(ref struct_ctx) = ctx.struct_declaration_context {
            let self_param = construct_parameter(
                Identifier::SELF.to_string(),
                Type::InoutType(InoutType {
                    key_type: Box::new(Type::UserDefinedType(Identifier::generated(
                        &struct_ctx.identifier.token,
                    ))),
                }),
            );

            declaration.head.parameters.insert(0, self_param);
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

            let state_variable = if ctx.special_declaration_context.is_some() {
                // Special declarations have no 'this' yet as it is being constructed
                Expression::Identifier(Identifier::generated(Identifier::TYPESTATE_VAR_NAME))
            } else {
                Expression::BinaryExpression(BinaryExpression {
                    lhs_expression: Box::new(Expression::SelfExpression),
                    rhs_expression: Box::new(Expression::Identifier(Identifier::generated(
                        Identifier::TYPESTATE_VAR_NAME,
                    ))),
                    op: BinOp::Dot,
                    line_info: Default::default(),
                })
            };

            *statement = Statement::Expression(Expression::BinaryExpression(BinaryExpression {
                lhs_expression: Box::new(state_variable),
                rhs_expression: Box::new(Expression::Literal(Literal::U8Literal(type_state_as_u8))),
                op: BinOp::Equal,
                line_info: Default::default(),
            }));
        }

        Ok(())
    }

    fn start_binary_expression(&mut self, expr: &mut BinaryExpression, _: &mut Context) -> VResult {
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
                    expression: Box::new(Expression::Identifier(Identifier::generated("this"))),
                }),
            });
        } else {
            call.identifier.token = mangle_ewasm_function(&function_name);

            let contract_argument = FunctionArgument {
                identifier: None,
                expression: Expression::Identifier(Identifier::generated("this")),
            };

            call.arguments.push(contract_argument);
        }

        Ok(())
    }
}
