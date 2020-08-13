use super::inkwell::values::BasicValueEnum;
use crate::ast::FunctionDeclaration;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::inkwell::types::{BasicType, BasicTypeEnum};
use crate::ewasm::inkwell::values::BasicValue;
use crate::ewasm::statements::LLVMStatement;
use crate::ewasm::types::LLVMType;
use crate::ewasm::Codegen;
use std::collections::HashMap;

#[allow(dead_code)]
pub struct LLVMFunction<'a> {
    pub function_declaration: &'a FunctionDeclaration,
}

#[allow(dead_code)]
impl<'a> LLVMFunction<'a> {
    pub fn generate(&self, codegen: &Codegen) {
        let function_name = self.function_declaration.head.identifier.token.as_str();

        let params = &self.function_declaration.head.parameters;
        let parameter_types = &params
            .iter()
            .map(|param| {
                LLVMType {
                    ast_type: &param.type_assignment,
                }
                    .generate(codegen)
            })
            .collect::<Vec<BasicTypeEnum>>();

        let func_type = if let Some(result_type) = self.function_declaration.get_result_type() {
            // should is_var_args be false?
            LLVMType {
                ast_type: &result_type,
            }
                .generate(codegen)
                .fn_type(parameter_types, false)
        } else {
            codegen.context.void_type().fn_type(parameter_types, false)
        };

        // add function type to module
        let func_val = codegen.module.add_function(&function_name, func_type, None);

        let param_names = params
            .iter()
            .map(|param| param.identifier.token.as_str())
            .collect::<Vec<&str>>();

        // set argument names
        for (i, param) in func_val.get_param_iter().enumerate() {
            param.set_name(param_names[i]);
        }

        let body = codegen.context.append_basic_block(func_val, "entry");
        codegen.builder.position_at_end(body);

        let params = param_names
            .into_iter()
            .zip(func_val.get_params().into_iter().map(|param| param))
            .collect::<HashMap<&str, BasicValueEnum>>();

        let mut function_context = FunctionContext::new(params);
        for statement in self.function_declaration.body.iter() {
            LLVMStatement { statement }.generate(codegen, &mut function_context);
        }

        codegen.verify_and_optimise(&func_val);
    }
}
