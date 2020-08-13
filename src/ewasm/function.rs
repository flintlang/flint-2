use crate::ast::{Expression, FunctionDeclaration, Modifier, Statement, VariableDeclaration};
use crate::environment::Environment;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::inkwell::types::{BasicType, BasicTypeEnum};
use crate::ewasm::inkwell::values::BasicValue;
use crate::ewasm::statements::LLVMStatement;
use crate::ewasm::types::LLVMType;
use crate::ewasm::Codegen;

pub struct LLVMFunction<'a> {
    pub function_declaration: &'a FunctionDeclaration,
    pub environment: &'a Environment,
}

impl<'a> LLVMFunction<'a> {
    pub fn generate(&self, codegen: &Codegen) {
        // TODO: declare function context and scope context?
        // TODO: how do we treat modifiers?
        let modifiers = self
            .function_declaration
            .head
            .modifiers
            .clone()
            .into_iter()
            .filter(|s| s == &Modifier::Public)
            .collect();

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
            .into_iter()
            .map(|param| param.identifier.token)
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
        func_val
            .get_param_iter()
            .enumerate()
            .map(|(i, arg)| arg.set_name(parameter_names[i].as_str()));

        let body = codegen.context.append_basic_block(func_val, "entry");
        codegen.builder.position_at_end(body);

        let function_context = FunctionContext::from(self.environment);

        let parameter_names = parameter_names.iter();

        for parameter in self.function_declaration.head.parameters {
            if let Some(parameter_name) = parameter_names.next() {
                function_context.add_local(&parameter_name, parameter);
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

        variables.into_iter().map(|var| {
            function_context.add_local(&var.identifier.token, var);
        });

        // TODO: add tags
        let tags = self.function_declaration.tags;
        // add dictionary to tags?

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
