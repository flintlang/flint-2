use crate::ast::expressions::Identifier;
use crate::ast::{CallerProtection, FunctionDeclaration};
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::statements::LLVMStatement;
use crate::ewasm::types::LLVMType;
use crate::ewasm::utils::generate_caller_variable;
use crate::ewasm::Codegen;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValue, BasicValueEnum};
use std::collections::HashMap;

pub struct LLVMFunction<'a> {
    pub function_declaration: &'a FunctionDeclaration,
    pub caller_binding: &'a Option<Identifier>,
    pub caller_protections: &'a Vec<CallerProtection>,
}

impl<'a> LLVMFunction<'a> {
    pub fn generate(&self, codegen: &mut Codegen) {
        let function_name = &self.function_declaration.head.identifier.token;
        let function_name = self
            .function_declaration
            .mangled_identifier
            .as_ref()
            .unwrap_or(&function_name);

        let func_val = codegen.module.get_function(function_name).unwrap();
        let body = codegen.context.append_basic_block(func_val, "entry");
        codegen.builder.position_at_end(body);

        let parameter_names: Vec<&str> = self
            .function_declaration
            .head
            .parameters
            .iter()
            .map(|param| param.identifier.token.as_str())
            .collect();

        let local_parameters = parameter_names
            .iter()
            .map(|name| name.to_string())
            .zip(func_val.get_params().into_iter().map(|param| param))
            .collect::<HashMap<String, BasicValueEnum>>();

        let mut function_context = FunctionContext::new(func_val, local_parameters);

        // Here, we add the contract global variable to the context as a pointer, under the name of the contract
        // We do this only if it is a contract wrapper function (and thus does not already have the contract in scope)
        if let Some(enclosing) = &self.function_declaration.head.identifier.enclosing_type {
            if enclosing.eq(codegen.contract_name) && self.function_declaration.is_external {
                let contract_global = codegen
                    .module
                    .get_global(codegen.contract_name)
                    .unwrap()
                    .as_pointer_value()
                    .as_basic_value_enum();
                function_context.add_local(Identifier::SELF, contract_global);
            }

            if (!self.caller_protections.iter().any(|c| c.is_any())
                && !self.caller_protections.is_empty()
                && (self.function_declaration.is_external
                    || contains_function(self.caller_protections, codegen, enclosing)))
                || self.caller_binding.is_some()
            {
                generate_caller_variable(
                    codegen,
                    &mut function_context,
                    self.caller_binding.clone(),
                );
            }
        }

        for statement in &self.function_declaration.body {
            if statement.eq(self.function_declaration.body.last().unwrap()) {
                function_context.is_last_statement = true;
            }

            LLVMStatement {
                statement: &statement,
            }
            .generate(codegen, &mut function_context);
        }

        codegen.verify_and_optimise(&func_val);
    }
}

pub fn generate_function_type(function_declaration: &FunctionDeclaration, codegen: &mut Codegen) {
    let function_name = &function_declaration.head.identifier.token;
    let function_name = function_declaration
        .mangled_identifier
        .as_ref()
        .unwrap_or(&function_name);

    let parameter_types = function_declaration
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

    let parameter_names: Vec<String> = function_declaration
        .head
        .parameters
        .iter()
        .map(|param| param.identifier.token.clone())
        .collect();

    // TODO: If one of our parameters is of dictionary type, we have to give the length of the array representing a dictionary, which is a problem
    // because we cannot tell the length of a dictionary just from its AST type.
    let func_type = if let Some(result_type) = function_declaration.get_result_type() {
        LLVMType {
            ast_type: &result_type,
        }
        .generate(codegen)
        .fn_type(&parameter_types, false)
    } else {
        codegen.context.void_type().fn_type(&parameter_types, false)
    };

    // add function type to module
    let func_val = codegen.module.add_function(&function_name, func_type, None);

    // set argument names
    for (i, arg) in func_val.get_param_iter().enumerate() {
        arg.set_name(parameter_names[i].as_str())
    }
}

fn contains_function(
    caller_protections: &[CallerProtection],
    codegen: &Codegen,
    enclosing: &str,
) -> bool {
    caller_protections.iter().any(|c| {
        codegen
            .module
            .get_function(&mangle_ewasm_function(&c.identifier.token, enclosing))
            .is_some()
    })
}

fn mangle_ewasm_function(function_name: &str, enclosing: &str) -> String {
    format!("{}_{}", enclosing, function_name)
}
