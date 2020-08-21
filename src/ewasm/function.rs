use crate::ast::{FunctionDeclaration, Modifier};
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::inkwell::types::{BasicType, BasicTypeEnum};
use crate::ewasm::inkwell::values::{BasicValue, BasicValueEnum};
use crate::ewasm::statements::LLVMStatement;
use crate::ewasm::types::LLVMType;
use crate::ewasm::Codegen;
use std::collections::HashMap;

pub struct LLVMFunction<'a> {
    pub function_declaration: &'a FunctionDeclaration,
}

impl<'a> LLVMFunction<'a> {
    pub fn generate(&self, codegen: &mut Codegen) {
        // TODO: declare function context and scope context?
        // TODO: how do we treat modifiers?
        let _modifiers: Vec<&Modifier> = self
            .function_declaration
            .head
            .modifiers
            .iter()
            .filter(|s| s == &&Modifier::Public)
            .collect();

        let function_name = &self.function_declaration.head.identifier.token;
        let function_name = self
            .function_declaration
            .mangled_identifier
            .as_ref()
            .unwrap_or(&function_name);

        let parameter_types = &self
            .function_declaration
            .head
            .parameters
            .iter()
            .map(|param| {
                LLVMType {
                    ast_type: &param.type_assignment,
                }
                .generate(codegen)
            })
            .collect::<Vec<BasicTypeEnum>>();

        let parameter_names: Vec<String> = self
            .function_declaration
            .head
            .parameters
            .iter()
            .map(|param| param.identifier.token.clone())
            .collect();

        let func_type = if let Some(result_type) = self.function_declaration.get_result_type() {
            // TODO: should is_var_args be false?
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

        // set argument names

        for (i, arg) in func_val.get_param_iter().enumerate() {
            arg.set_name(parameter_names[i].as_str())
        }

        let body = codegen.context.append_basic_block(func_val, "entry");
        codegen.builder.position_at_end(body);

        let parameter_names: Vec<&str> = parameter_names
            .iter()
            .map(|p_name| p_name.as_str())
            .collect();

        let local_parameters = parameter_names
            .iter()
            .map(|name| name.to_string())
            .zip(func_val.get_params().into_iter().map(|param| param))
            .collect::<HashMap<String, BasicValueEnum>>();

        let mut function_context = FunctionContext::new(func_val, local_parameters);

        // TODO: add tags
        let _tags = &self.function_declaration.tags;
        // add dictionary to tags?

        for statement in &self.function_declaration.body {
            LLVMStatement {
                statement: &statement,
            }
            .generate(codegen, &mut function_context);
        }

        // TODO: add statement to context?

        codegen.verify_and_optimise(&func_val);
    }
}
