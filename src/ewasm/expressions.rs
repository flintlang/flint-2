use super::inkwell::values::BasicValueEnum;
use crate::ast::Expression;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::function_context::FunctionContext;

// TODO remove this allowance when it gets used
#[allow(dead_code)]
pub struct LLVMExpression<'a> {
    pub expr: &'a Expression,
}

#[allow(dead_code)]
impl<'a> LLVMExpression<'a> {
    // We want to take an expression, create any of the intermediary steps to evaluate it,
    // and then return the tmp variable that stores the evaluated result
    pub fn generate(
        &self,
        _codegen: &Codegen,
        _function_context: &mut FunctionContext,
    ) -> BasicValueEnum {
        unimplemented!()
    }
}
