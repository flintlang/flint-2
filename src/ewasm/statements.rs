use super::inkwell::values::InstructionValue;
use crate::ast::Statement;
use crate::ewasm::Codegen;

pub struct EWASMStatement<'a> {
    pub statement: &'a Statement,
}

impl<'a> EWASMStatement<'a> {
    pub fn generate(&self, _codegen: &Codegen) -> InstructionValue {
        unimplemented!()
    }
}
