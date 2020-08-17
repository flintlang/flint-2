use crate::ast::expressions::Expression;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::expressions::LLVMExpression;
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
        codegen: &Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        // TODO decide how to assign to things, taking into account that they might be a contract variable,
        // or a local, or have a non identifier expression on the lhs
        // Need to end up with a pointer to the lhs

        function_context.assigning = true;
        // TODO this will not return a correct pointer at the moment
        let _lhs_ptr = LLVMExpression { expression: self.lhs }.generate(codegen, function_context);
        function_context.assigning = false;

        unimplemented!()

        // if let Expression::Identifier(id) = &self.lhs {
        //     let rhs_val = LLVMExpression {
        //         expression: self.rhs,
        //     }
        //     .generate(codegen, function_context);
        //
        //     function_context.assigning = true;
        //     let lhs_ptr = LLVMIdentifier {
        //         identifier: id
        //     }.generate(codegen, function_context);
        //     function_context.assigning = false;
        //
        //     codegen.builder.build_store(lhs_ptr.into_pointer_value(), rhs_val);
        //     function_context.update_declaration(id.token.as_str(), rhs_val);
        // }
        //
        // panic!("Invalid assignment")
    }
}
