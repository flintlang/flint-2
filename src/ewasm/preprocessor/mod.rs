mod utils;

use crate::ast::declarations::{FunctionDeclaration, VariableDeclaration};
use crate::ast::expressions::Identifier;
use crate::ast::types::{Type, InoutType};
use crate::ast::VResult;
use crate::ast::operators::BinOp;
use crate::ast::statements::{Statement, ReturnStatement};
use crate::ast::expressions::{Expression, BinaryExpression};
use crate::ast::declarations::Parameter;
use crate::context::Context;
use crate::visitor::Visitor;
use crate::ewasm::preprocessor::utils::*;

pub struct LLVMPreprocessor<> {}

impl<'ctx> Visitor for LLVMPreprocessor<> {
    fn start_variable_declaration(
        &mut self,
        declaration: &mut VariableDeclaration,
        ctx: &mut Context,
    ) -> VResult {
        if ctx.in_function_or_special() {
            if let Some(ref mut scope_context) = ctx.scope_context {
                scope_context.local_variables.push(*declaration);
            }

            // If is function declaration context
            if let Some(ref mut function_declaration_context) = ctx.function_declaration_context {
                function_declaration_context
                    .local_variables
                    .push(declaration.clone());
            
            // If it is special declaration context
            } else if let Some(ref mut special_declaration_context) = ctx.special_declaration_context {
                special_declaration_context
                    .local_variables
                    .push(declaration.clone());
            }
        }

        Ok(())
    }

    fn start_function_declaration(
        &mut self,
        declaration: &mut FunctionDeclaration,
        ctx: &mut Context,
    ) -> VResult {
        let enclosing_identifier = ctx
            .enclosing_type_identifier()
            .map(|id| id.token.to_string())
            .unwrap_or_default();

        let mangled_name = mangle_ewasm_function (
            &declaration.head.identifier.token,
        );

        declaration.mangled_identifier = Some(mangled_name);

        // construct self parameter for struct
        if let Some(ref struct_ctx) = ctx.struct_declaration_context {
            if enclosing_identifier != "Quartz_Global" {
                let self_param = construct_parameter(
                    "QuartzSelf".to_string(),
                    Type::InoutType(InoutType {
                        key_type: Box::new(Type::UserDefinedType(Identifier::generated(
                            &struct_ctx.identifier.token,
                        ))),
                    })
                );

                declaration.head.parameters.insert(0, self_param);
                // TODO: add to scope?
            }
        }

        if let Some(ref contract_ctx) = ctx.contract_behaviour_declaration_context {
            let identifier = &contract_ctx.identifier;
            let parameter_type = Type::InoutType(InoutType {
                key_type: Box::new(Type::UserDefinedType(identifier.clone())),
            });
            let parameter = Parameter {
                identifier: Identifier::generated(Identifier::SELF),
                type_assignment: parameter_type,
                expression: None,
                line_info: Default::default(),
            };

            declaration.head.parameters.insert(0, parameter.clone());

            // TODO: add to scope?
        }

        // TODO: dynamic parameters?

        Ok(())
    }

    fn finish_function_declaration(&mut self, declaration: &mut FunctionDeclaration, ctx: &mut Context) -> VResult {
        if declaration.is_void() {
            let statement = declaration.body.last();
            if !declaration.body.is_empty() {
                if let Statement::ReturnStatement(_) = statement.unwrap() {
                } else {
                    declaration
                        .body
                        .push(Statement::ReturnStatement(ReturnStatement {
                            expression: None,
                            ..Default::default()
                        }));
                }
            } else {
                declaration
                    .body
                    .push(Statement::ReturnStatement(ReturnStatement {
                        expression: None,
                        ..Default::default()
                    }));
            }
        } else {
            let variable_declaration = VariableDeclaration {
                declaration_token: None,
                identifier: Identifier::generated("ret"),
                variable_type: declaration
                    .head
                    .result_type
                    .as_ref()
                    .unwrap()
                    .clone(),
                expression: None,
            };

            declaration.body.insert(
                0,
                Statement::Expression(Expression::VariableDeclaration(variable_declaration)),
            )
        }

        Ok(())
    }

    fn start_expression(&mut self, expr: &mut Expression, ctx: &mut Context) -> VResult {
        Ok(())
    }

    fn start_binary_expression(&mut self, expr: &mut BinaryExpression, ctx: &mut Context) -> VResult {
        // removes assignment shorthand expressions, e.g. += and *=
        if expr.op.is_assignment_shorthand() {
            let op = expr.op.get_assignment_shorthand();
            expr.op = BinOp::Equal;

            let rhs = BinaryExpression {
                lhs_expression: expr.lhs_expression,
                rhs_expression: expr.rhs_expression,
                op,
                line_info: expr.line_info
            };

            expr.rhs_expression = Box::from(Expression::BinaryExpression(rhs));   
        } else if let BinOp::Dot = expr.op {
            let mut trail = &ctx.function_call_receiver_trail;
            trail.push(*expr.lhs_expression);
            ctx.function_call_receiver_trail = trail.to_vec();
        }

        match expr.op {
            BinOp::LessThanOrEqual => {
                let lhs = Expression::BinaryExpression(BinaryExpression {
                    lhs_expression: expr.lhs_expression,
                    rhs_expression: expr.rhs_expression,
                    op: BinOp::LessThan,
                    line_info: expr.line_info,
                });
                let rhs = Expression::BinaryExpression(BinaryExpression {
                    lhs_expression: expr.lhs_expression,
                    rhs_expression: expr.rhs_expression,
                    op: BinOp::DoubleEqual,
                    line_info: expr.line_info,
                });
                expr.lhs_expression = Box::from(lhs);
    
                expr.rhs_expression = Box::from(rhs);
                expr.op = BinOp::Or;
            },
    
            BinOp::GreaterThanOrEqual => {
                let lhs = Expression::BinaryExpression(BinaryExpression {
                    lhs_expression: expr.lhs_expression,
                    rhs_expression: expr.rhs_expression,
                    op: BinOp::GreaterThan,
                    line_info: expr.line_info.clone(),
                });
                let rhs = Expression::BinaryExpression(BinaryExpression {
                    lhs_expression: expr.lhs_expression,
                    rhs_expression: expr.rhs_expression,
                    op: BinOp::DoubleEqual,
                    line_info: expr.line_info,
                });
                expr.lhs_expression = Box::from(lhs);
    
                expr.rhs_expression = Box::from(rhs);
                expr.op = BinOp::Or;
            },
    
            _ => ()
        }
    
        Ok(())
    
    }
}
