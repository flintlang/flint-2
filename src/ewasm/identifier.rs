use crate::ast::expressions::Identifier;
use crate::ewasm::Codegen;

#[allow(dead_code)]
pub struct LLVMIdentifier<'a> {
    pub identifier: &'a Identifier,
}

#[allow(dead_code)]
impl<'a> LLVMIdentifier<'a> {
    pub fn generate(&self, _codegen: &Codegen) {
        unimplemented!();
    }
}
