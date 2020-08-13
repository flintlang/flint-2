use super::inkwell::values::InstructionValue;
use crate::ast::Statement;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::function_context::FunctionContext;

pub struct LLVMStatement<'a> {
    pub statement: &'a Statement,
}

impl<'a> LLVMStatement<'a> {
    pub fn generate(
        &self,
        _codegen: &Codegen,
        _function_context: &mut FunctionContext,
    ) -> InstructionValue {
        unimplemented!()
    }
}
