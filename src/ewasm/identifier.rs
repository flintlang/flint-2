use crate::ast::expressions::Identifier;
use crate::ewasm::Codegen;

pub struct LLVMIdentifier<'a> {
    pub identifier: &'a Identifier,
}

impl<'a> LLVMIdentifier<'a> {
    pub fn generate(&self, _codegen: &Codegen) {
        unimplemented!();
    }
}