use crate::ast::calls::{ExternalCall, FunctionCall};
use crate::ewasm::codegen::Codegen;
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
        _codegen: &Codegen<'_, 'ctx>,
        _function_context: &FunctionContext,
    ) -> BasicValueEnum<'ctx> {
        unimplemented!();
    }
}
