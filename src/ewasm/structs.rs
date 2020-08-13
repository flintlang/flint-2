use super::inkwell::types::BasicTypeEnum;
use super::inkwell::values::{BasicValue, BasicValueEnum};
use crate::ast::{SpecialDeclaration, StructDeclaration, StructMember, VariableDeclaration};
use crate::ewasm::codegen::Codegen;
use crate::ewasm::function::LLVMFunction;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::statements::LLVMStatement;
use crate::ewasm::types::LLVMType;
use std::collections::HashMap;

pub struct LLVMStruct<'a> {
    pub struct_declaration: &'a StructDeclaration,
}

impl<'a> LLVMStruct<'a> {
    pub fn generate(&self, codegen: &mut Codegen) {
        self.create_type(codegen);
        self.generate_initialiser(codegen);
        self.generate_functions(codegen);
    }

    fn create_type(&self, codegen: &mut Codegen) {
        let fields = &self
            .struct_declaration
            .members
            .iter()
            .filter_map(|f| {
                if let StructMember::VariableDeclaration(vd, _) = f {
                    Some(vd)
                } else {
                    None
                }
            })
            .collect::<Vec<&VariableDeclaration>>();

        let field_names = fields
            .iter()
            .map(|dec| dec.identifier.token.clone())
            .collect::<Vec<String>>();

        let field_types = &fields
            .iter()
            .map(|dec| {
                LLVMType {
                    ast_type: &dec.variable_type,
                }
                    .generate(codegen)
            })
            .collect::<Vec<BasicTypeEnum>>();

        let struct_type = codegen.context.struct_type(field_types, false);
        let struct_info = (field_names, struct_type);

        codegen.types.insert(
            self.struct_declaration.identifier.token.clone(),
            struct_info,
        );
    }

    fn generate_functions(&self, codegen: &Codegen) {
        self.struct_declaration
            .members
            .iter()
            .filter_map(|m| {
                if let StructMember::FunctionDeclaration(fd) = m {
                    Some(fd)
                } else {
                    None
                }
            })
            .for_each(|func| {
                LLVMFunction {
                    function_declaration: func,
                }
                    .generate(codegen)
            });
    }

    fn generate_initialiser(&self, codegen: &Codegen) {
        let initialiser = self
            .struct_declaration
            .members
            .iter()
            .filter_map(|m| {
                if let StructMember::SpecialDeclaration(sp) = m {
                    if sp.is_public() && sp.is_init() {
                        return Some(sp);
                    }
                }
                None
            })
            .collect::<Vec<&SpecialDeclaration>>();

        assert_eq!(initialiser.len(), 1);
        let initialiser = initialiser[0];

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
            .into_iter()
            .zip(init_func.get_params().into_iter())
            .collect::<HashMap<&str, BasicValueEnum>>();

        let mut function_context = FunctionContext::new(params);
        let block = codegen.context.append_basic_block(init_func, "entry");
        codegen.builder.position_at_end(block);
        for statement in initialiser.body.iter() {
            LLVMStatement { statement }.generate(codegen, &mut function_context);
        }

        codegen.verify_and_optimise(&init_func);
    }
}
