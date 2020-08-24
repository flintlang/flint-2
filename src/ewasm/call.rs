use crate::ast::calls::{ExternalCall, FunctionCall};
use crate::ewasm::codegen::Codegen;
use crate::ewasm::expressions::LLVMExpression;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::inkwell::values::BasicValueEnum;

pub struct LLVMExternalCall<'a> {
    pub external_call: &'a ExternalCall,
}

impl<'a> LLVMExternalCall<'a> {
    pub fn generate<'ctx>(
        &self,
        _codegen: &Codegen<'_, 'ctx>,
        _function_context: &FunctionContext,
    ) -> BasicValueEnum<'ctx> {
        unimplemented!();
    }
}

pub struct LLVMFunctionCall<'a> {
    pub function_call: &'a FunctionCall,
    pub module_name: &'a str,
}

impl<'a> LLVMFunctionCall<'a> {
    pub fn generate<'ctx>(
        &self,
        codegen: &mut Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        let fn_name = &self.function_call.identifier.token;

        if self.is_init() {
            // add local variable of struct field

            let struct_type = codegen
                .module
                .get_function(self.function_call.identifier.token.as_str())
                .unwrap()
                .get_params()
                .last()
                .unwrap()
                .get_type()
                .into_pointer_type()
                .get_element_type()
                .into_struct_type();

            let struct_var = struct_type.const_zero();

            // add local variable to function call arguments
            function_context.add_local("tmp_var", BasicValueEnum::StructValue(struct_var));
        }

        let arguments: Vec<BasicValueEnum> = self
            .function_call
            .arguments
            .clone()
            .into_iter()
            .map(|a| {
                LLVMExpression {
                    expression: &a.expression,
                }
                .generate(codegen, function_context)
            })
            .collect();

        if let Some(fn_value) = codegen.module.get_function(fn_name) {
            // TODO: if the function returns void then we shouldn't return a BasicValueEnum
            match codegen
                .builder
                .build_call(fn_value, &arguments, fn_name)
                .try_as_basic_value()
                .left()
            {
                Some(val) => return val,
                None => {
                    if self.is_init() {
                        if let Some(this_argument) = arguments.last() {
                            if this_argument.is_pointer_value() {
                                let ptr = this_argument.into_pointer_value();
                                return codegen.builder.build_load(ptr, "initialised");
                            }
                        }
                    }

                    return BasicValueEnum::IntValue(codegen.context.i8_type().const_zero());
                }
            }
        }

        panic!(format!("Function {} is not defined", fn_name))
    }

    fn is_init(&self) -> bool {
        self.function_call.identifier.token.contains("Init")
    }
}
