use super::inkwell::types::BasicTypeEnum;
use super::inkwell::values::{BasicValue, BasicValueEnum};
use crate::ast::declarations::{StructMember, VariableDeclaration};
use crate::ast::{
    AssetDeclaration, ContractBehaviourDeclaration, ContractBehaviourMember, ContractDeclaration,
    ContractMember, SpecialDeclaration, StructDeclaration, TraitDeclaration,
};
use crate::environment::Environment;
use crate::ewasm::codegen::Codegen;
use crate::ewasm::declaration::LLVMFieldDeclaration;
use crate::ewasm::function::LLVMFunction;
use crate::ewasm::function_context::FunctionContext;
use crate::ewasm::statements::LLVMStatement;
use crate::ewasm::structs::LLVMStruct;
use crate::ewasm::types::LLVMType;
use crate::ewasm::structs::utils::generate_initialiser;
use std::collections::HashMap;

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

        // TODO: runtime functions?

        // setting up a struct to contain the contract data
        let variable_declarations = &self
            .contract_declaration
            .contract_members
            .clone()
            .into_iter()
            .filter_map(|m| {
                if let ContractMember::VariableDeclaration(v, _) = m {
                    Some(v)
                } else {
                    None
                }
            })
            .collect::<Vec<VariableDeclaration>>();

        let members: Vec<VariableDeclaration> = variable_declarations
            .clone()
            .into_iter()
            .filter(|m| !m.variable_type.is_dictionary_type())
            .collect::<Vec<VariableDeclaration>>();

        let member_names = &members
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

        let struct_type = codegen.context.struct_type(member_types, false);
        let struct_info = (member_names.clone(), struct_type);

        // add struct declaration
        codegen.types.insert(
            self.contract_declaration.identifier.token.clone(),
            struct_info,
        );

        // adds struct initialisation function
        let members: Vec<StructMember> = members
            .into_iter()
            .map(|dec| StructMember::VariableDeclaration(dec, None))
            .collect();

        let _struct_declaration = StructDeclaration {
            identifier: self.contract_declaration.identifier.clone(),
            conformances: self.contract_declaration.conformances.clone(),
            members,
        };
 
        let mut initialiser_declaration = None;
        for declarations in self.contract_behaviour_declarations.clone() {
            for member in declarations.members.clone() {
                if let ContractBehaviourMember::SpecialDeclaration(s) = member {
                    if s.is_init() && s.is_public() {
                        initialiser_declaration = Some(s.clone());
                    }
                }
            }
        }

        if initialiser_declaration.is_none() {
            panic!("Public Initiliaser not found")
        }

        let initialiser_declaration = initialiser_declaration.unwrap();

        generate_initialiser(&initialiser_declaration, codegen);

        
        // add global var declaration of struct
        codegen
            .module
            .add_global(struct_type, None, &self.contract_declaration.identifier.token);

        let _properties: Vec<_> = self
            .contract_declaration
            .get_variable_declarations_without_dict()
            .collect();
        
        // set global variable to struct value
        // do we need to set an initialiser function?
        //global.set_initializer();
        // add struct 'wrapping' to all function declarations

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
        self.contract_behaviour_declarations
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
            .for_each(|func| {
                LLVMFunction {
                    function_declaration: func,
                }
                .generate(codegen)
            });

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
