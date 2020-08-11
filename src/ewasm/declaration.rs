use crate::ast::VariableDeclaration;
use crate::ewasm::Codegen;

pub struct EWASMFieldDeclaration<'a> {
    pub declaration: &'a VariableDeclaration,
}

impl<'a> EWASMFieldDeclaration<'a> {
    pub(crate) fn generate(&self, _codegen: &Codegen) {
        unimplemented!("Need to decide if we want these generated in a struct or not")
    }
}
