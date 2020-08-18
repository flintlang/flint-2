use crate::ast::expressions::Expression;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::inkwell::values::BasicValueEnum;

#[derive(Debug)]
pub struct LLVMAssignment<'a> {
    pub lhs: &'a Expression,
    pub rhs: &'a Expression,
}

impl<'a> LLVMAssignment<'a> {
    pub fn generate<'ctx>(
        &self,
        _codegen: &Codegen<'_, 'ctx>,
        _function_context: &mut FunctionContext<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        /*
        a.b.c = contract.val

        lhs = bin_exp | identifier.

        let thing = 5;

        thing_ptr = alloca int*;
        store thing_ptr thing;
        store thing_ptr 10;

        assert(thing == 10);

        func float hyp(thing: Contract) {

        }
         */

        unimplemented!()
    }
}
