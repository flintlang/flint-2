use crate::ast::expressions::Expression;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::expressions::LLVMExpression;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::inkwell::values::BasicValueEnum;

pub struct LLVMAssignment<'a> {
    pub lhs: &'a Expression,
    pub rhs: &'a Expression,
}

impl<'a> LLVMAssignment<'a> {
    pub fn generate<'ctx>(
        &self,
        codegen: &Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        if let Expression::Identifier(id) = self.lhs {
            let identifier = &id.token;
            let rhs = LLVMExpression {
                expression: self.rhs,
            }
            .generate(codegen, function_context);

            function_context.update_declaration(&identifier, rhs);
            // TODO index into the struct with the LLVMStructAccess generate, and then assign the correct
            // value to that
            unimplemented!();
        }

        panic!("Invalid assignment")
    }
}
