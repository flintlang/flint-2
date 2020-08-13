use crate::ast::VariableDeclaration;
use crate::ewasm::codegen::Codegen;

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
    pub fn generate(&self, _codegen: &Codegen) {
        unimplemented!("Need to decide if we want these generated in a struct or not")
    }
}
