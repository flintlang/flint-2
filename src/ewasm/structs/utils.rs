use crate::ast::expressions::Identifier;
use crate::ast::types::Type;
use crate::ast::SpecialDeclaration;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::statements::LLVMStatement;
use crate::ewasm::types::LLVMType;
use crate::ewasm::utils::generate_caller_variable;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValue, BasicValueEnum};
use std::collections::HashMap;

pub fn generate_initialiser(
    initialiser: &SpecialDeclaration,
    codegen: &mut Codegen,
    caller_binding: Option<Identifier>,
) {
    let func_name = get_function_name(initialiser);
    let init_func = codegen.module.get_function(&func_name).unwrap();
    let params = &initialiser.head.parameters;

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

    if let Some(enclosing_type) = &initialiser.head.enclosing_type {
        if enclosing_type == codegen.contract_name {
            let global = codegen
                .module
                .get_global(codegen.contract_name)
                .unwrap()
                .as_pointer_value();
            function_context.add_local("this", global.as_basic_value_enum());

            if caller_binding.is_some() {
                generate_caller_variable(codegen, &mut function_context, caller_binding);
            }
        }
    }

    for statement in initialiser.body.iter() {
        LLVMStatement { statement }.generate(codegen, &mut function_context);
    }

    codegen.verify_and_optimise(&init_func);
}

pub fn add_initialiser_function_declaration(
    initialiser: &SpecialDeclaration,
    codegen: &mut Codegen,
) {
    let params = &initialiser.head.parameters;
    let param_types = params
        .iter()
        .map(|param| {
            // TODO: If one of our parameters is of dictionary type, we have to give the length of the array representing a dictionary, which is a problem
            // because we cannot tell the length of a dictionary just from its AST type.
            LLVMType {
                ast_type: &param.type_assignment,
            }
                .generate(codegen)
        })
        .collect::<Vec<BasicTypeEnum>>();

    let void_type = codegen.context.void_type().fn_type(&param_types, false);
    let func_name = get_function_name(initialiser);
    codegen.module.add_function(&func_name, void_type, None);
}

fn get_function_name(initialiser: &SpecialDeclaration) -> String {
    if let Some(contract_name) = initialiser.head.enclosing_type.as_ref() {
        return format!("{}Init", contract_name);
    } else if let Some(self_argument) = initialiser.head.parameters.last() {
        if self_argument.identifier.token == "this" {
            if let Type::InoutType(t) = &self_argument.type_assignment {
                if let Type::UserDefinedType(t) = &*t.key_type {
                    return format!("{}Init", t.token.clone());
                }
            }
        }
    }

    panic!("Invalid initialiser function")
}
