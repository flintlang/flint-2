use super::inkwell::values::BasicValueEnum;
use crate::ast::Expression;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::Codegen;

// TODO remove this allowance when it gets used
#[allow(dead_code)]
pub struct EWASMExpression<'a> {
    pub expr: &'a Expression,
}

#[allow(dead_code)]
impl<'a> EWASMExpression<'a> {
    // We want to take an expression, create any of the intermediary steps to evaluate it,
    // and then return the tmp variable that stores the evaluated result
    pub fn generate(
        &self,
        _expr: &Expression,
        _codegen: &Codegen,
        _function_context: &mut FunctionContext,
    ) -> BasicValueEnum {
        unimplemented!()
    }
}
