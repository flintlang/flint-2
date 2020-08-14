use crate::ast::VariableDeclaration;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::inkwell::values::BasicValueEnum;

pub struct LLVMFieldDeclaration<'a> {
    pub declaration: &'a VariableDeclaration,
}

impl<'a> LLVMFieldDeclaration<'a> {
    pub(crate) fn generate(&self, _codegen: &Codegen) {
        unimplemented!("Need to decide if we want these generated in a struct or not")
    }
}

#[allow(dead_code)]
pub struct LLVMVariableDeclaration<'a> {
    pub declaration: &'a VariableDeclaration,
}

#[allow(dead_code)]
impl<'a> LLVMVariableDeclaration<'a> {
    pub fn generate<'ctx>(&self, _codegen: &Codegen<'_, 'ctx>, _function_context: &FunctionContext) -> BasicValueEnum<'ctx> {
        unimplemented!("Need to decide if we want these generated in a struct or not")
    }
}
