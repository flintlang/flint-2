use super::inkwell::values::BasicValueEnum;
use crate::ast::Expression;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::function_context::FunctionContext;

pub struct LLVMExpression<'a> {
    pub expr: &'a Expression,
}

impl<'a> LLVMExpression<'a> {
    // We want to take an expression, create any of the intermediary steps to evaluate it,
    // and then return the tmp variable that stores the evaluated result
    pub fn generate<'ctx>(
        &self,
        _codegen: &Codegen<'_, 'ctx>,
        _function_context: &mut FunctionContext,
    ) -> BasicValueEnum<'ctx> {
        unimplemented!()
    }
}
