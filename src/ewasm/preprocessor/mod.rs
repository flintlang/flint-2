mod utils;

use crate::ast::declarations::{
    ContractBehaviourDeclaration, ContractBehaviourMember, FunctionDeclaration, VariableDeclaration,
};
use crate::ast::expressions::Identifier;
use crate::ast::expressions::{BinaryExpression, Expression};
use crate::ast::operators::BinOp;
use crate::ast::statements::{ReturnStatement, Statement};
use crate::ast::types::{InoutType, Type};
use crate::ast::{
    ContractDeclaration, ContractMember, Modifier, SpecialDeclaration, SpecialSignatureDeclaration,
    StructDeclaration, StructMember, VResult,
};
use crate::context::Context;
use crate::ewasm::preprocessor::utils::*;
use crate::visitor::Visitor;

pub struct LLVMPreprocessor {}

impl Visitor for LLVMPreprocessor {
    fn start_contract_declaration(
        &mut self,
        dec: &mut ContractDeclaration,
        ctx: &mut Context,
    ) -> VResult {
        let vars_with_assignments = dec
            .contract_members
            .iter()
            .filter_map(|m| {
                if let ContractMember::VariableDeclaration(vd, _) = m {
                    if vd.expression.is_some() {
                        return Some(vd);
                    }
                }
                None
            })
            .collect::<Vec<&VariableDeclaration>>();

        // Every contract must have an initialiser
        let init = ctx
            .environment
            .get_public_initialiser(dec.identifier.token.as_str())
            .unwrap();
        for default_assignment in vars_with_assignments {
            let assignment = BinaryExpression {
                lhs_expression: Box::new(Expression::Identifier(
                    default_assignment.identifier.clone(),
                )),
                rhs_expression: Box::new(*default_assignment.expression.clone().unwrap()),
                op: BinOp::Equal,
                line_info: Default::default(),
            };

            init.body
                .push(Statement::Expression(Expression::BinaryExpression(
                    assignment,
                )));
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
                    parameters: vec![],
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

    fn finish_statement(&mut self, _statement: &mut Statement, _ctx: &mut Context) -> VResult {
        unimplemented!(); // TODO need to do become statements
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
}
