use crate::ast::Identifier;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::preprocessor::LLVMPreProcessor;
use inkwell::types::AnyTypeEnum;

pub fn get_num_pointer_layers(val_type: AnyTypeEnum) -> u8 {
    let mut num_pointers = 0;
    let mut val_type = val_type;
    while val_type.is_pointer_type() {
        num_pointers += 1;
        val_type = val_type.into_pointer_type().get_element_type();
    }
    num_pointers
}

pub fn generate_caller_variable<'ctx>(
    codegen: &mut Codegen<'_, 'ctx>,
    function_context: &mut FunctionContext<'ctx>,
    caller_binding: Option<Identifier>,
) {
    let caller_address = codegen
        .builder
        .build_call(
            codegen.module.get_function(LLVMPreProcessor::CALLER_WRAPPER_NAME).unwrap(),
            &[],
            "tmp_call",
        )
        .try_as_basic_value()
        .left()
        .unwrap();

    if let Some(caller) = caller_binding {
        function_context.add_local(&caller.token, caller_address);
    } else {
        function_context.add_local(LLVMPreProcessor::CALLER_PROTECTIONS_PARAM, caller_address);
    }
}
