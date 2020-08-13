use super::inkwell::types::BasicTypeEnum;

use super::inkwell::values::{BasicValue, BasicValueEnum};
use crate::ast::{
    AssetDeclaration, ContractBehaviourDeclaration, ContractBehaviourMember, ContractDeclaration,
    ContractMember, SpecialDeclaration, StructDeclaration, TraitDeclaration,
};
use crate::environment::Environment;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::declaration::LLVMFieldDeclaration;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::statements::LLVMStatement;
use crate::ewasm::structs::LLVMStruct;
use crate::ewasm::types::LLVMType;
use std::collections::HashMap;
use crate::ewasm::function::LLVMFunction;

pub struct LLVMContract<'a> {
    pub contract_declaration: &'a ContractDeclaration,
    pub contract_behaviour_declarations: Vec<&'a ContractBehaviourDeclaration>,
    pub struct_declarations: Vec<&'a StructDeclaration>,
    pub asset_declarations: Vec<&'a AssetDeclaration>,
    pub external_traits: Vec<&'a TraitDeclaration>,
    pub environment: &'a Environment,
}

impl<'a> LLVMContract<'a> {
    pub(crate) fn generate(&self, codegen: &mut Codegen) {
        codegen.ether_imports();

        // Set up the contract data here
        // This will require making a struct for the contract
        // TODO perhaps this could happen here, or maybe in the preprocessor
        self.contract_declaration
            .contract_members
            .iter()
            .filter_map(|m| {
                if let ContractMember::VariableDeclaration(v, _) = m {
                    if !v.variable_type.is_dictionary_type() {
                        return Some(v);
                    }
                }
                None
            })
            .for_each(|declaration| LLVMFieldDeclaration { declaration }.generate(codegen));

        // Set up struct definitions here
        self.struct_declarations.iter().for_each(|dec| {
            LLVMStruct {
                struct_declaration: dec,
            }
                .generate(codegen)
        });

        // Set up contract initialiser here
        let initialiser = self
            .contract_behaviour_declarations
            .iter()
            .map(|dec| &dec.members)
            .flatten()
            .filter_map(|member| {
                if let ContractBehaviourMember::SpecialDeclaration(dec) = member {
                    if dec.is_init() && dec.is_public() {
                        return Some(dec);
                    }
                }
                None
            })
            .collect::<Vec<&SpecialDeclaration>>();

        // There should only be one contract initialiser
        assert_eq!(initialiser.len(), 1);
        let initialiser = initialiser[0];
        self.generate_initialiser(codegen, initialiser);

        // Generate all contract functions
        self
            .contract_behaviour_declarations
            .iter()
            .flat_map(|declaration| declaration
                .members
                .iter()
                .filter_map(|m| {
                    if let ContractBehaviourMember::FunctionDeclaration(fd) = m {
                        Some(fd)
                    } else {
                        None
                    }
                }))
            .for_each(|func| LLVMFunction { function_declaration: func }.generate(codegen));


        // TODO Asset declarations?
    }

    fn generate_initialiser(&self, codegen: &Codegen, initialiser: &SpecialDeclaration) {
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

        let fn_type = codegen.context.void_type().fn_type(&param_types, false);

        let contract_name = initialiser.head.enclosing_type.as_ref();
        let contract_name = contract_name.unwrap();

        let init_name = &format!("{}Init", contract_name);
        let init_func = codegen.module.add_function(init_name, fn_type, None);

        let param_names = params
            .iter()
            .map(|p| p.identifier.token.as_str())
            .collect::<Vec<&str>>();

        for (i, param) in init_func.get_param_iter().enumerate() {
            param.set_name(param_names[i]);
        }

        let body = codegen.context.append_basic_block(init_func, "entry");
        codegen.builder.position_at_end(body);

        let params = param_names
            .into_iter()
            .zip(init_func.get_params().into_iter().map(|param| param))
            .collect::<HashMap<&str, BasicValueEnum>>();

        let mut function_context = FunctionContext::new(params);
        for statement in initialiser.body.iter() {
            LLVMStatement { statement }.generate(codegen, &mut function_context);
        }

        codegen.verify_and_optimise(&init_func);
    }
}
