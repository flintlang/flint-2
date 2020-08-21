use crate::ast::VariableDeclaration;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::expressions::LLVMExpression;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::inkwell::types::BasicTypeEnum::*;
use crate::ewasm::inkwell::values::BasicValueEnum;
use crate::ewasm::types::LLVMType;

pub struct LLVMVariableDeclaration<'a> {
    pub declaration: &'a VariableDeclaration,
}

impl<'a> LLVMVariableDeclaration<'a> {
    pub fn generate<'ctx>(
        &self,
        codegen: &mut Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        let name = self.declaration.identifier.token.as_str();
        let expression = if let Some(expr) = &self.declaration.expression {
            LLVMExpression { expression: expr }.generate(codegen, function_context)
        } else {
            // creates dummy value for variable assignment to be overwritten
            let variable_type = LLVMType {
                ast_type: &self.declaration.variable_type,
            }
            .generate(codegen);

            match variable_type {
                ArrayType(a) => BasicValueEnum::ArrayValue(a.const_zero()),
                FloatType(f) => BasicValueEnum::FloatValue(f.const_zero()),
                IntType(i) => BasicValueEnum::IntValue(i.const_zero()),
                PointerType(p) => BasicValueEnum::PointerValue(p.const_null()),
                StructType(s) => BasicValueEnum::StructValue(s.const_zero()),
                VectorType(v) => BasicValueEnum::VectorValue(v.const_zero()),
            }
        };

        function_context.add_local(name, expression);
        expression
    }
}
