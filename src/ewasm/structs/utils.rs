use crate::ast::{Identifier, SpecialDeclaration, Type};
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

    let type_names = initialiser.head.parameters.iter().map(|param| {
        if let Type::UserDefinedType(Identifier {
            token: type_name, ..
        }) = &param.type_assignment
        {
            Some(type_name.to_string())
        } else {
            None
        }
    });

    let params = param_names
        .iter()
        .map(|name| name.to_string())
        .zip(type_names.zip(init_func.get_params().into_iter()))
        .collect::<HashMap<String, (Option<String>, BasicValueEnum)>>();

    let mut function_context = FunctionContext::new(init_func, params);
    let block = codegen.context.append_basic_block(init_func, "entry");
    codegen.builder.position_at_end(block);
    for statement in initialiser.body.iter() {
        LLVMStatement { statement }.generate(codegen, &mut function_context);
    }

    codegen.verify_and_optimise(&init_func);
}
