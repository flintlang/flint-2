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
        codegen: &Codegen<'_, 'ctx>,
        function_context: &mut FunctionContext<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        let fn_name = &self.function_call.identifier.token;

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
            match codegen
                .builder
                .build_call(fn_value, &arguments, fn_name)
                .try_as_basic_value()
                .left()
            {
                Some(val) => return val,
                None => panic!("Invalid function call"),
            }
        }

        panic!(format!("Function {} is not defined", fn_name))
    }
}
