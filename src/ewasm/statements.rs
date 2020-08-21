use super::inkwell::values::IntValue;
use crate::ast::{Assertion, Expression, IfStatement, ReturnStatement, Statement};
use crate::ewasm::codegen::Codegen;
use crate::ewasm::expressions::LLVMExpression;
use crate::ewasm::function_context::FunctionContext;
use std::convert::TryFrom;

pub struct LLVMStatement<'a> {
    pub statement: &'a Statement,
}

impl<'a> LLVMStatement<'a> {
    pub fn generate<'ctx>(
        &self,
        codegen: &mut Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) {
        match self.statement {
            Statement::ReturnStatement(r) => {
                LLVMReturnStatement { statement: r }.generate(codegen, function_context);
            }
            Statement::Expression(expression) => {
                LLVMExpression { expression }.generate(codegen, function_context);
            }
            Statement::BecomeStatement(_) => {
                panic!("This should have been done in the preprocessor")
            }
            // TODO what is an emit statement as opposed to an expression function call?
            Statement::EmitStatement(_) => unimplemented!(),
            Statement::ForStatement(_) => unimplemented!(),
            Statement::IfStatement(if_statement) => {
                LLVMIfStatement { if_statement }.generate(codegen, function_context);
            }
            Statement::DoCatchStatement(_) => unimplemented!(),
            Statement::Assertion(assertion) => {
                LLVMAssertion { assertion }.generate(codegen, function_context)
            }
        }
    }
}

struct LLVMReturnStatement<'a> {
    statement: &'a ReturnStatement,
}

impl<'a> LLVMReturnStatement<'a> {
    fn generate<'ctx>(
        &self,
        codegen: &mut Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) {
        if let Some(return_expression) = &self.statement.expression {
            let expr = LLVMExpression {
                expression: return_expression,
            };
            let expr = expr.generate(codegen, function_context);
            codegen.builder.build_return(Some(&expr));
        } else {
            codegen.builder.build_return(None);
        }
    }
}

struct LLVMIfStatement<'a> {
    if_statement: &'a IfStatement,
}

impl<'a> LLVMIfStatement<'a> {
    fn generate<'ctx>(
        &self,
        codegen: &mut Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) {
        // TODO I suspect this will not work, but it is likely along the right lines. It will
        // be difficult to fix until we can run the code and see, however, so I will leave it for now
        let IfStatement {
            condition,
            body,
            else_body,
            ..
        } = self.if_statement;
        let condition = condition_to_int_value(condition, codegen, function_context);
        let this_func = function_context.get_current_func();
        let then_bb = codegen.context.append_basic_block(this_func, "then");
        let else_bb = codegen.context.append_basic_block(this_func, "else");
        let continue_bb = codegen.context.append_basic_block(this_func, "after_if");

        codegen
            .builder
            .build_conditional_branch(condition, then_bb, else_bb);

        // Build then block
        codegen.builder.position_at_end(then_bb);
        for statement in body {
            LLVMStatement { statement }.generate(codegen, function_context);
        }

        if let Some(Statement::ReturnStatement(_)) = body.last() {
        } else {
            codegen.builder.build_unconditional_branch(continue_bb);
        }

        // Build else block
        codegen.builder.position_at_end(else_bb);
        for statement in else_body {
            LLVMStatement { statement }.generate(codegen, function_context);
        }

        if let Some(Statement::ReturnStatement(_)) = else_body.last() {
        } else {
            codegen.builder.build_unconditional_branch(continue_bb);
        }

        // Reposition after if
        codegen.builder.position_at_end(continue_bb);
    }
}

struct LLVMAssertion<'a> {
    assertion: &'a Assertion,
}

impl<'a> LLVMAssertion<'a> {
    fn generate<'ctx>(
        &self,
        codegen: &mut Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) {
        let condition =
            condition_to_int_value(&self.assertion.expression, codegen, function_context);
        let this_func = function_context.get_current_func();
        let fail_block = codegen.context.append_basic_block(this_func, "fail");
        let continue_block = codegen
            .context
            .append_basic_block(this_func, "passed_check");

        codegen
            .builder
            .build_conditional_branch(condition, continue_block, fail_block);

        // Build fail block
        codegen.builder.position_at_end(fail_block);
        codegen.builder.build_unreachable();

        // Build continue block
        codegen.builder.position_at_end(continue_block);
    }
}

fn condition_to_int_value<'ctx>(
    condition: &Expression,
    codegen: &mut Codegen<'_, 'ctx>,
    function_context: &mut FunctionContext<'ctx>,
) -> IntValue<'ctx> {
    let condition = LLVMExpression {
        expression: condition,
    }
    .generate(codegen, function_context);

    // Evaluated conditions should be boolean, which in llvm is represented by a one bit int
    assert!(condition.is_int_value());
    let condition = IntValue::try_from(condition).expect("Could not convert condition to int");
    assert_eq!(condition.get_type().get_bit_width(), 1);

    condition
}
