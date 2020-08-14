use crate::ast::VariableDeclaration;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::inkwell::values::BasicValueEnum;

pub struct LLVMVariableDeclaration<'a> {
    pub declaration: &'a VariableDeclaration,
}

impl<'a> LLVMVariableDeclaration<'a> {
    pub fn generate<'ctx>(
        &self,
        _codegen: &Codegen<'_, 'ctx>,
        _function_context: &FunctionContext,
    ) -> BasicValueEnum<'ctx> {
        unimplemented!("Need to decide if we want these generated in a struct or not")
    }
}
