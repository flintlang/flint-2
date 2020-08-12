use super::inkwell::types::BasicTypeEnum;
use crate::ast::{
    AssetDeclaration, ContractBehaviourDeclaration, ContractBehaviourMember, ContractDeclaration,
    ContractMember, SpecialDeclaration, StructDeclaration, TraitDeclaration,
};
use crate::environment::Environment;
use crate::ewasm::declaration::EWASMFieldDeclaration;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::statements::LLVMStatement;
use crate::ewasm::types::to_llvm_type;
use crate::ewasm::Codegen;

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
        // TODO do we want all contract data wrapped in a struct? More complicated since we need to
        // worry about pointers into the struct and also create instance of the struct, but also
        // it seems to make sense to group the data together
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
            .map(|param| to_llvm_type(&param.type_assignment, codegen.context))
            .collect::<Vec<BasicTypeEnum>>();

        let fn_type = codegen.context.void_type().fn_type(&parameter_types, false);

        let contract_name = initialiser.head.enclosing_type.as_ref();
        let contract_name = contract_name.unwrap();

        let init_name = &format!("{}Init", contract_name);
        let init_func = codegen.module.add_function(init_name, fn_type, None);
        let body = codegen.context.append_basic_block(init_func, "entry");
        codegen.builder.position_at_end(body);

        // TODO we need some sort of function (or scope) context so that we can access previous statements,
        // parameters, local vars etc. The One defined for move is almost right but does not translate
        // since we need to be able to get to the actual values, not just the name
        let function_context = FunctionContext::from(self.environment);
        for statement in initialiser.body.iter() {
            let instr = LLVMStatement { statement }.generate(codegen);
            // Add to context now
        }

        codegen.verify_and_optimise(&init_func);
    }
}
