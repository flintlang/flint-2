use super::inkwell::types::BasicTypeEnum;

use super::inkwell::values::BasicValue;
use crate::ast::{
    AssetDeclaration, ContractBehaviourDeclaration, ContractBehaviourMember, ContractDeclaration,
    ContractMember, SpecialDeclaration, StructDeclaration, TraitDeclaration,
};
use crate::environment::Environment;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::declaration::EWASMFieldDeclaration;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::statements::LLVMStatement;
use crate::ewasm::types::LLVMType;

pub struct EWASMContract<'a> {
    pub contract_declaration: &'a ContractDeclaration,
    pub contract_behaviour_declarations: Vec<&'a ContractBehaviourDeclaration>,
    pub struct_declarations: Vec<&'a StructDeclaration>,
    pub asset_declarations: Vec<&'a AssetDeclaration>,
    pub external_traits: Vec<&'a TraitDeclaration>,
    pub environment: &'a Environment,
}

impl<'a> EWASMContract<'a> {
    pub(crate) fn generate(&self, codegen: &Codegen) {
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
            .for_each(|declaration| EWASMFieldDeclaration { declaration }.generate(codegen));

        // TODO Set up struct stuff here

        // Set up initialiser here
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

        // TODO All other functions and declarations etc.
    }

    fn generate_initialiser(&self, codegen: &Codegen, initialiser: &SpecialDeclaration) {
        let parameter_types = initialiser
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

        // TODO name the parameter types

        let fn_type = codegen.context.void_type().fn_type(&parameter_types, false);

        let contract_name = initialiser.head.enclosing_type.as_ref();
        let contract_name = contract_name.unwrap();

        let init_name = &format!("{}Init", contract_name);
        let init_func = codegen.module.add_function(init_name, fn_type, None);

        for (_, param) in init_func.get_param_iter().enumerate() {
            param.set_name("this is a name");
        }

        let body = codegen.context.append_basic_block(init_func, "entry");
        codegen.builder.position_at_end(body);

        let mut _function_context = FunctionContext::from(self.environment);
        for statement in initialiser.body.iter() {
            let _instr = LLVMStatement { statement }.generate(codegen);
            // Add to context now
        }

        codegen.verify_and_optimise(&init_func);
    }
}
