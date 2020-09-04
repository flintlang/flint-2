use super::inkwell::types::AnyType;
use crate::ast::calls::{ExternalCall, FunctionCall};
use crate::ewasm::codegen::Codegen;
use crate::ewasm::expressions::LLVMExpression;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::inkwell::values::BasicValueEnum;
use crate::ewasm::utils::get_num_pointer_layers;

pub struct LLVMExternalCall<'a> {
    pub external_call: &'a ExternalCall,
}

impl<'a> LLVMExternalCall<'a> {
    pub fn generate<'ctx>(
        &self,
        _codegen: &Codegen<'_, 'ctx>,
        _function_context: &FunctionContext,
    ) -> Option<BasicValueEnum<'ctx>> {
        unimplemented!();
    }
}

pub struct LLVMFunctionCall<'a> {
    pub function_call: &'a FunctionCall,
}

impl<'a> LLVMFunctionCall<'a> {
    pub fn generate<'ctx>(
        &self,
        codegen: &mut Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> Option<BasicValueEnum<'ctx>> {
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
        
        let params = codegen
            .module
            .get_function(self.function_call.identifier.token.as_str())
            .unwrap()
            .get_params();

        let mut arguments: Vec<BasicValueEnum> = self
            .function_call
            .arguments
            .iter()
            .map(|a| {
                LLVMExpression {
                    expression: &a.expression,
                }
                .generate(codegen, function_context)
                .unwrap()
            })
            .collect();

        for (index, argument) in arguments.iter_mut().enumerate() {
            let param_num_pointers =
                get_num_pointer_layers(params.get(index).unwrap().get_type().as_any_type_enum());
            let argument_num_pointers =
                get_num_pointer_layers(argument.get_type().as_any_type_enum());

            if argument_num_pointers == param_num_pointers + 1 {
                *argument = codegen
                    .builder
                    .build_load(argument.into_pointer_value(), "tmp_load");
            } else if argument_num_pointers != param_num_pointers {
                panic!("Invalid argument")
            }
        }

        if let Some(fn_value) = codegen.module.get_function(fn_name) {
            match codegen
                .builder
                .build_call(fn_value, &arguments, fn_name)
                .try_as_basic_value()
                .left()
            {
                Some(val) => return Some(val),
                None => {
                    if self.is_init() {
                        if let Some(this_argument) = arguments.last() {
                            if this_argument.is_pointer_value() {
                                let ptr = this_argument.into_pointer_value();
                                return Some(codegen.builder.build_load(ptr, "initialised"));
                            }
                        }
                    }
                    return None;
                }
            }
        }

        panic!(format!("Function {} is not defined", fn_name))
    }

    fn is_init(&self) -> bool {
        self.function_call.identifier.token.contains("Init")
    }
}
