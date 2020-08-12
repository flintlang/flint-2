use super::inkwell::values::InstructionValue;
use crate::ast::Statement;
use crate::ewasm::codegen::Codegen;

pub struct LLVMStatement<'a> {
    pub statement: &'a Statement,
}

impl<'a> LLVMStatement<'a> {
    pub fn generate(&self, _codegen: &Codegen) -> InstructionValue {
        unimplemented!()
    }
}
