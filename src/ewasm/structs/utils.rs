use crate::ast::SpecialDeclaration;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::inkwell::types::BasicTypeEnum;
use crate::ewasm::inkwell::values::{BasicValue, BasicValueEnum};
use crate::ewasm::statements::LLVMStatement;
use crate::ewasm::types::LLVMType;
use std::collections::HashMap;

pub fn generate_initialiser(initialiser: &SpecialDeclaration, codegen: &Codegen) {
    let params = &initialiser.head.parameters;
    let param_types = params
        .iter()
        .map(|param| {
            LLVMType {
                ast_type: &param.type_assignment,
            }
                .generate(codegen)
        })
        .collect::<Vec<BasicTypeEnum>>();

    let void_type = codegen.context.void_type().fn_type(&param_types, false);

    let func_name = &format!("{}Init", initialiser.head.enclosing_type.as_ref().unwrap());
    let init_func = codegen.module.add_function(func_name, void_type, None);

    let param_names = params
        .iter()
        .map(|param| param.identifier.token.as_str())
        .collect::<Vec<&str>>();

    for (i, param) in init_func.get_param_iter().enumerate() {
        param.set_name(param_names[i]);
    }

    let params = param_names
        .iter()
        .map(|name| name.to_string())
        .zip(init_func.get_params().into_iter())
        .collect::<HashMap<String, BasicValueEnum>>();

    let mut function_context = FunctionContext::new(init_func, params);
    let block = codegen.context.append_basic_block(init_func, "entry");
    codegen.builder.position_at_end(block);
    for statement in initialiser.body.iter() {
        LLVMStatement { statement }.generate(codegen, &mut function_context);
    }

    codegen.verify_and_optimise(&init_func);
}
