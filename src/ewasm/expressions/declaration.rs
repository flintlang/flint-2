use crate::ast::VariableDeclaration;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::expressions::LLVMExpression;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::types::LLVMType;
use inkwell::types::BasicTypeEnum::*;
use inkwell::values::BasicValue;
use inkwell::values::BasicValueEnum;

pub struct LLVMVariableDeclaration<'a> {
    pub declaration: &'a VariableDeclaration,
}

impl<'a> LLVMVariableDeclaration<'a> {
    pub fn generate<'ctx>(
        &self,
        codegen: &mut Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> Option<BasicValueEnum<'ctx>> {
        let name = self.declaration.identifier.token.as_str();
        let expression = if let Some(expr) = &self.declaration.expression {
            let value = LLVMExpression { expression: expr }
                .generate(codegen, function_context)
                .unwrap();
            let ptr = codegen.builder.build_alloca(value.get_type(), name);
            codegen.builder.build_store(ptr, value);
            ptr.as_basic_value_enum()
        } else {
            // creates dummy value for variable assignment to be overwritten
            let variable_type = LLVMType {
                ast_type: &self.declaration.variable_type,
            }
            .generate(codegen);

            match variable_type {
                ArrayType(a) => {
                    let value = BasicValueEnum::ArrayValue(a.const_zero());
                    let ptr = codegen.builder.build_array_alloca(
                        a.get_element_type(),
                        a.size_of()
                            .unwrap_or_else(|| codegen.context.i32_type().const_int(10, false)),
                        "arr_ptr",
                    );
                    codegen.builder.build_store(ptr, value);
                    ptr.as_basic_value_enum()
                }
                FloatType(f) => {
                    let value = BasicValueEnum::FloatValue(f.const_zero());
                    let ptr = codegen.builder.build_alloca(f, name);
                    codegen.builder.build_store(ptr, value);
                    ptr.as_basic_value_enum()
                }
                IntType(i) => {
                    let value = BasicValueEnum::IntValue(i.const_zero());
                    let ptr = codegen.builder.build_alloca(i, name);
                    codegen.builder.build_store(ptr, value);
                    ptr.as_basic_value_enum()
                }
                PointerType(p) => {
                    let value = BasicValueEnum::PointerValue(p.const_null());
                    let ptr = codegen.builder.build_alloca(p, name);
                    codegen.builder.build_store(ptr, value);
                    ptr.as_basic_value_enum()
                }
                StructType(s) => {
                    let value = BasicValueEnum::StructValue(s.const_zero());
                    let ptr = codegen.builder.build_alloca(s, name);
                    codegen.builder.build_store(ptr, value);
                    ptr.as_basic_value_enum()
                }
                VectorType(_) => panic!("Vector types are unsupported"),
            }
        };

        function_context.add_local(name, expression);
        Some(expression)
    }
}
