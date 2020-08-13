use std::collections::HashMap;
use crate::ewasm::inkwell::types::{BasicTypeEnum, BasicType};
use crate::ewasm::inkwell::values::BasicValue;
use crate::ast::{FunctionDeclaration, VariableDeclaration, Statement, Expression};
use crate::environment::Environment;
use crate::ewasm::Codegen;
use crate::ewasm::statements::LLVMStatement;
use crate::ewasm::types::LLVMType;
use crate::ewasm::function_context::FunctionContext;

pub struct LLVMFunction<'a> {
    pub function_declaration: &'a FunctionDeclaration,
    pub environment: &'a Environment
}

impl<'a> LLVMFunction<'a> {
    pub fn generate(&self, codegen: &Codegen) {
        // TODO: declare function context and scope context?
        // TODO: declare modifiers?
        
        let function_name = self.function_declaration.head.identifier.token;
        let function_name = self
            .function_declaration
            .mangled_identifier
            .as_ref()
            .unwrap_or(&function_name);

        let parameter_types = &self
            .function_declaration
            .head
            .parameters
            .into_iter()
            .map(|param| LLVMType { ast_type: &param.type_assignment }.generate(codegen))
            .collect::<Vec<BasicTypeEnum>>();

        let parameter_names: Vec<String> = self
            .function_declaration
            .head
            .parameters
            .into_iter()
            .map(|param| { param.identifier.token })
            .collect();

        // TODO: do I need to check if it returns?
        
        let func_type = if let Some(result_type) = self.function_declaration.get_result_type() {
            // should is_var_args be false?
            LLVMType { ast_type: &result_type }.generate(codegen).fn_type(parameter_types, false)
        } else {
            codegen.context.void_type().fn_type(parameter_types, false)
        };

        // add function type to module
        let func_val = codegen.module.add_function(&function_name, func_type, None);

        // set argument names
        func_val
            .get_param_iter()
            .enumerate()
            .map(|(i, arg)| {
                arg.set_name(parameter_names[i].as_str())
            });

        let body = codegen.context.append_basic_block(func_val, "entry");
        codegen.builder.position_at_end(body);


        let function_context = FunctionContext::from(self.environment);

        let parameter_types = parameter_types.iter();
        
        for parameter in parameter_names {
            if let Some(parameter_type) = parameter_types.next() {
                function_context.add_local(&parameter, *parameter_type);
            } else {
                panic!("Mismatched parameter names and parameter types")
            }
        }
        
        let variables: Vec<Expression> = self
            .function_declaration
            .body
            .clone()
            .into_iter()
            .filter_map(|v| {
                if let Statement::Expression(e) = v {
                    Some(e)
                } else {
                    None
                }
            })
            .collect();

        let mut variables: Vec<VariableDeclaration> = variables
            .into_iter()
            .filter_map(|v| {
                if let Expression::VariableDeclaration(e) = v {
                    Some(e)
                } else {
                    None
                }
            })
            .collect();

        variables
            .into_iter()
            .map(|var| {
                function_context.add_local(&var.identifier.token, var);
            });

        // TODO: add tags

        let statements = self.function_declaration.body.iter(); 

        statements
            .into_iter()
            .map(|statement| LLVMStatement { statement }.generate(codegen));
        
        for statement in self.function_declaration.body.iter() {
            let instr = LLVMStatement { statement }.generate(codegen);
            // Add to context now
        }
        
        codegen.verify_and_optimise(&func_val);
    } 
}