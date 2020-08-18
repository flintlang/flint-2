use crate::ast::expressions::Expression;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::expressions::LLVMExpression;
use crate::ewasm::inkwell::values::BasicValueEnum;

#[derive(Debug)]
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
        function_context.assigning = true;
        let lhs = LLVMExpression { expression: self.lhs }.generate(codegen, function_context);
        function_context.assigning = false;
        let rhs = LLVMExpression { expression: self.rhs }.generate(codegen, function_context);

        codegen.builder.build_store(lhs.into_pointer_value(), rhs);
        // TODO: what should we return?
        rhs
    }
}
