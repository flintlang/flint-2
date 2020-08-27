use super::inkwell::types::BasicTypeEnum;
use super::inkwell::values::BasicValue;
use crate::ast::declarations::{FunctionDeclaration, VariableDeclaration};
use crate::ast::expressions::Identifier;
use crate::ast::{
    AssetDeclaration, ContractBehaviourDeclaration, ContractBehaviourMember, ContractDeclaration,
    ContractMember, SpecialDeclaration, StructDeclaration, StructMember, TraitDeclaration,
};
use crate::environment::Environment;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::function::{generate_function_type, LLVMFunction};
use crate::ewasm::structs::utils::{add_initialiser_function_declaration, generate_initialiser};
use crate::ewasm::structs::{create_type, LLVMStruct};
use crate::ewasm::types::LLVMType;

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
        codegen.runtime_functions();

        // Add each struct to the list of known types
        self.struct_declarations
            .iter()
            .for_each(|dec| create_type(dec, codegen));

        // setting up a struct to contain the contract data
        let members = self
            .contract_declaration
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
            .collect::<Vec<&VariableDeclaration>>();

        let member_names = members
            .iter()
            .map(|member| member.identifier.token.clone())
            .collect::<Vec<String>>();

        let member_types = &members
            .iter()
            .map(|member| {
                LLVMType {
                    ast_type: &member.variable_type,
                }
                .generate(codegen)
            })
            .collect::<Vec<BasicTypeEnum>>();

        let struct_type = codegen.context.opaque_struct_type(codegen.contract_name);
        struct_type.set_body(member_types, false);

        // add contract initialiser declaration
        codegen.types.insert(
            codegen.contract_name.to_string(),
            (member_names, struct_type),
        );

        // add global var declaration of struct
        let global = codegen
            .module
            .add_global(struct_type, None, codegen.contract_name);
        // Required so that the global variable is safe to access in memory. Note this is garbage
        // data but this should not matter since an initialiser will overwrite it

        // Create initialiser for contract
        global.set_initializer(&struct_type.const_zero().as_basic_value_enum());

        // Set up struct definitions here

        self.struct_declarations.iter().for_each(|dec| {
            let initialiser = dec
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
            add_initialiser_function_declaration(initialiser, codegen);
        });

        self.struct_declarations.iter().for_each(|dec| {
            LLVMStruct {
                struct_declaration: dec,
            }
            .generate(codegen)
        });

        let initialiser = self
            .contract_behaviour_declarations
            .iter()
            .flat_map(|behaviour_dec| {
                behaviour_dec.members.iter().filter_map(|m| {
                    if let ContractBehaviourMember::SpecialDeclaration(sp) = m {
                        if sp.is_public() && sp.is_init() {
                            return Some(sp);
                        }
                    }
                    None
                })
            })
            .collect::<Vec<&SpecialDeclaration>>();

        // There should only be one contract initialiser
        assert_eq!(initialiser.len(), 1);
        let initialiser = initialiser[0];
        // TODO: check caller protections
        let (caller_binding, _) = self
            .contract_behaviour_declarations
            .iter()
            .find_map(|dec| {
                if dec.members.iter().any(|m| {
                    if let ContractBehaviourMember::SpecialDeclaration(sp) = m {
                        sp.is_public() && sp.is_init()
                    } else {
                        false
                    }
                }) {
                    Some((&dec.caller_binding, &dec.caller_protections))
                } else {
                    None
                }
            })
            .unwrap();

        add_initialiser_function_declaration(initialiser, codegen);
        generate_initialiser(initialiser, codegen, caller_binding.clone());

        // Generate all contract functions
        let function_declarations = self
            .contract_behaviour_declarations
            .iter()
            .flat_map(|declaration| {
                declaration.members.iter().filter_map(|m| {
                    if let ContractBehaviourMember::FunctionDeclaration(fd) = m {
                        Some(fd)
                    } else {
                        None
                    }
                })
            })
            .collect::<Vec<&FunctionDeclaration>>();

        let bindings = self
            .contract_behaviour_declarations
            .iter()
            .flat_map(|dec| {
                dec.members.iter().filter_map(move |m| {
                    if let ContractBehaviourMember::FunctionDeclaration(_) = m {
                        Some(&dec.caller_binding)
                    } else {
                        None
                    }
                })
            })
            .collect::<Vec<&Option<Identifier>>>();

        function_declarations
            .iter()
            .for_each(|func| generate_function_type(func, codegen));

        function_declarations
            .iter()
            .enumerate()
            .for_each(|(index, func)| {
                LLVMFunction {
                    function_declaration: func,
                    caller_binding: bindings.get(index).unwrap(),
                }
                .generate(codegen);
            });
        // TODO Asset declarations?
        codegen.module.print_to_stderr();
    }
}
