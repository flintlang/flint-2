mod conflicts;
mod declarations;
mod functions;
mod properties;

use crate::ast::*;
use std::collections::HashMap;

mod expr_type_check;

#[derive(Debug, Default, Clone)]
pub struct Environment {
    pub contract_declarations: Vec<Identifier>,
    pub struct_declarations: Vec<Identifier>,
    pub enum_declarations: Vec<Identifier>,
    pub event_declarations: Vec<Identifier>,
    pub trait_declarations: Vec<Identifier>,
    pub asset_declarations: Vec<Identifier>,
    pub types: HashMap<TypeIdentifier, TypeInfo>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum FunctionCallMatchResult {
    MatchedFunction(FunctionInformation),
    MatchedFunctionWithoutCaller(Candidates),
    MatchedInitializer(SpecialInformation),
    MatchedFallback(SpecialInformation),
    MatchedGlobalFunction(FunctionInformation),
    Failure(Candidates),
}

impl FunctionCallMatchResult {
    fn merge(self, f: FunctionCallMatchResult) -> FunctionCallMatchResult {
        if let FunctionCallMatchResult::Failure(c1) = &self {
            if let FunctionCallMatchResult::Failure(c2) = f.clone() {
                let mut c1_candidates = c1.candidates.clone();
                let mut c2_candidates = c2.candidates;
                c1_candidates.append(&mut c2_candidates);
                FunctionCallMatchResult::Failure(Candidates {
                    candidates: c1_candidates,
                })
            } else {
                f
            }
        } else {
            self
        }
    }
}

#[derive(Debug, Clone)]
pub enum CallableInformation {
    FunctionInformation(FunctionInformation),
    SpecialInformation(SpecialInformation),
}

impl CallableInformation {
    pub(crate) fn name(&self) -> &str {
        match self {
            CallableInformation::FunctionInformation(info) => &info.identifier().token,
            CallableInformation::SpecialInformation(info) => info.name()
        }
    }

    pub(crate) fn line_info(&self) -> Option<&LineInfo> {
        match self {
            CallableInformation::FunctionInformation(info) => Some(info.line_info()),
            _ => None
        }
    }

    pub(crate) fn get_parameter_types(&self) -> Vec<&Type> {
        match self {
            CallableInformation::FunctionInformation(info) => info.get_parameter_types().collect(),
            CallableInformation::SpecialInformation(info) => info.get_parameter_types().collect()
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Candidates {
    pub(crate) candidates: Vec<CallableInformation>,
}

impl Environment {
    pub fn build(&mut self, module: Module) {
        for declaration in module.declarations {
            match declaration {
                TopLevelDeclaration::ContractDeclaration(c) => self.add_contract_declaration(&c),
                TopLevelDeclaration::StructDeclaration(s) => self.add_struct_declaration(&s),
                TopLevelDeclaration::EnumDeclaration(e) => self.add_enum_declaration(&e),
                TopLevelDeclaration::TraitDeclaration(t) => self.add_trait_declaration(&t),
                TopLevelDeclaration::ContractBehaviourDeclaration(c) => {
                    self.add_contract_behaviour_declaration(&c)
                }
                TopLevelDeclaration::AssetDeclaration(a) => self.add_asset_declaration(&a),
            }
        }
    }

    fn add_conformance(&mut self, type_id: &str, conformance_identifier: &str) {
        let trait_info = &self.types.get(conformance_identifier);
        let type_info = &self.types.get(type_id);
        if trait_info.is_some() && type_info.is_some() {
            let conformance = self.types.get(conformance_identifier).unwrap().clone();
            self.types
                .get_mut(type_id)
                .unwrap()
                .conformances
                .push(conformance);
        }
    }

    pub fn add_special(
        &mut self,
        special: SpecialDeclaration,
        type_id: &str,
        caller_protections: Vec<CallerProtection>,
        type_states: Vec<TypeState>,
    ) {
        if special.is_init() {
            if special.is_public() {
                let type_info = &self.types.get_mut(type_id);
                if type_info.is_some() {
                    self.types.get_mut(type_id).unwrap().public_initializer = Some(special.clone());
                }
            }
            if let Some(type_info) = self.types.get_mut(type_id) {
                type_info.initialisers.push(SpecialInformation {
                    declaration: special,
                    type_states,
                    caller_protections,
                });
            }
        } else if let Some(type_info) = self.types.get_mut(type_id) {
            type_info.fallbacks.push(SpecialInformation {
                declaration: special,
                type_states,
                caller_protections,
            });
        }
    }
    fn add_type_state(&mut self, contract_name: &str, type_state: TypeState) {
        if let Some(type_info) = self.types.get_mut(contract_name) {
            type_info.type_states.push(type_state);
        }
    }

    pub fn get_contract_type_states(&self, contract_name: &str) -> Vec<TypeState> {
        if let Some(type_info) = self.types.get(contract_name) {
            type_info.type_states.clone()
        } else {
            panic!("Contract {} does not exist", contract_name)
        }
    }

    pub fn contains_type_state(&mut self, contract_name: &str, type_state: &TypeState) -> bool {
        self.get_contract_type_states(contract_name)
            .contains(type_state)
    }

    pub fn get_contract_state(&self, contract_name: &str) -> Option<TypeState> {
        if let Some(type_info) = self.types.get(contract_name) {
            type_info.current_state.clone()
        } else {
            panic!("Contract {} does not exist", contract_name)
        }
    }

    pub fn set_contract_state(&mut self, contract_name: &str, type_state: TypeState) {
        if let Some(type_info) = self.types.get_mut(contract_name) {
            type_info.current_state = Some(type_state)
        } else {
            panic!("Contract {} does not exist", contract_name)
        }
    }

    pub fn get_caller_protection(
        &self,
        protection: &CallerProtection,
    ) -> Option<PropertyInformation> {
        if let Some(contract_declaration) = self.contract_declarations.get(0) {
            if let Some(type_info) = self.types.get(&contract_declaration.token) {
                for (property, property_info) in &type_info.properties {
                    if *property == protection.identifier.token {
                        return Some(property_info.clone());
                    }
                }
            }
        }
        None
    }

    pub fn contains_caller_protection(&self, protection: &CallerProtection, type_id: &str) -> bool {
        self.declared_caller_protections(type_id)
            .contains(&protection.name())
    }

    pub fn declared_caller_protections(&self, type_id: &str) -> Vec<String> {
        let caller_protection_property = |p: &PropertyInformation| match p.property.get_type() {
            Type::Address => true,
            Type::FixedSizedArrayType(f) => {
                if f.key_type.is_address_type() {
                    return true;
                }
                false
            }
            Type::ArrayType(a) => {
                if a.key_type.is_address_type() {
                    return true;
                }
                false
            }
            Type::DictionaryType(d) => {
                if d.value_type.is_address_type() {
                    return true;
                }
                false
            }
            _ => false,
        };
        let caller_protection_function = |f: &FunctionInformation| {
            if let Some(result_type) = f.declaration.get_result_type() {
                let mut parameter_types = f.get_parameter_types();
                if let Some(parameter_type) = parameter_types.next() {
                    // There are no further parameter types
                    parameter_types.all(|_| false)
                        && parameter_type.is_address_type()
                        && result_type.is_bool_type()
                } else {
                    result_type.is_address_type()
                }
            } else { false }
        };
        self.types
            .get(type_id)
            .into_iter()
            .flat_map(|type_info| {
                type_info
                    .properties
                    .iter()
                    .filter(|(_, v)| caller_protection_property(v))
                    .map(|(k, _)| k.clone())
                    .chain(
                        type_info
                            .functions
                            .iter()
                            .filter(|(_, v)| v.iter().any(caller_protection_function))
                            .map(|(k, _)| k.clone()),
                    )
            })
            .collect()
    }

    fn external_trait_init() -> SpecialSignatureDeclaration {
        SpecialSignatureDeclaration {
            special_token: "init".to_string(),
            enclosing_type: None,
            attributes: vec![],
            modifiers: vec![],
            mutates: vec![],
            parameters: vec![Parameter {
                identifier: Identifier::generated("address"),
                type_assignment: Type::Address,
                expression: None,
                line_info: Default::default(),
            }],
        }
    }

    pub fn get_public_initialiser(&mut self, type_id: &str) -> Option<&mut SpecialDeclaration> {
        self.types
            .get_mut(type_id)
            .unwrap()
            .public_initializer
            .as_mut()
    }

    pub fn has_public_initialiser(&self, type_id: &str) -> bool {
        self.types
            .get(type_id)
            .unwrap()
            .public_initializer
            .is_some()
    }

    pub fn is_contract_declared(&self, type_id: &str) -> bool {
        let contract = &self
            .contract_declarations
            .iter()
            .find(|&x| x.token.eq(type_id));
        contract.is_some()
    }

    pub fn is_contract_stateful(&self, type_id: &str) -> bool {
        if let Some(contract_info) = self.types.get(type_id) {
            !contract_info.type_states.is_empty()
        } else {
            panic!("Contract {} does not exist!", type_id)
        }
    }

    pub fn is_state_declared(&self, state: &TypeState, type_id: &str) -> bool {
        if let Some(contract_info) = self.types.get(type_id) {
            contract_info.type_states.contains(state)
        } else {
            panic!("Contract {} does not exist", type_id)
        }
    }

    pub fn is_struct_declared(&self, type_id: &str) -> bool {
        let struct_decl = &self
            .struct_declarations
            .iter()
            .find(|&x| x.token.eq(type_id));
        struct_decl.is_some()
    }

    pub fn is_trait_declared(&self, type_id: &str) -> bool {
        let identifier = &self
            .trait_declarations
            .iter()
            .find(|&x| x.token.eq(type_id));
        identifier.is_some()
    }

    pub fn is_asset_declared(&self, type_id: &str) -> bool {
        let identifier = &self
            .asset_declarations
            .iter()
            .find(|&x| x.token.eq(type_id));
        identifier.is_some()
    }

    pub fn is_enum_declared(&self, type_id: &str) -> bool {
        let enum_declaration = &self.enum_declarations.iter().find(|&x| x.token.eq(type_id));
        enum_declaration.is_some()
    }

    pub fn is_recursive_struct(&self, type_id: &str) -> bool {
        let properties = &self.types.get(type_id).unwrap().ordered_properties;

        for property in properties {
            if let Some(type_property) = self.types.get(type_id).unwrap().properties.get(property) {
                match type_property.get_type() {
                    Type::UserDefinedType(i) => return i.token == type_id,
                    _ => {
                        return false;
                    }
                }
            }
        }
        false
    }

    pub fn is_type_declared(&self, type_id: &str) -> bool {
        self.types.get(type_id).is_some()
    }

    pub fn is_initialise_call(&self, call: &FunctionCall) -> bool {
        self.is_struct_declared(&call.identifier.token)
            || self.is_asset_declared(&call.identifier.token)
    }

    pub fn type_size(&self, input_type: &Type) -> u64 {
        match input_type {
            Type::Bool => 1,
            Type::Int => 1,
            Type::String => 1,
            Type::Address => 1,
            Type::InoutType(_) => unimplemented!(),
            Type::ArrayType(_) => 1,
            Type::RangeType(_) => unimplemented!(),
            Type::FixedSizedArrayType(a) => {
                let key_size = self.type_size(&a.key_type);
                key_size * a.size
            }
            Type::DictionaryType(_) => unimplemented!(),
            Type::UserDefinedType(i) => {
                if self.is_enum_declared(&i.token) {
                    unimplemented!()
                }

                self.types
                    .get(&i.token)
                    .into_iter()
                    .flat_map(|enclosing| &enclosing.properties)
                    .map(|(_, v)| self.type_size(&v.property.get_type()))
                    .sum()
            }
            Type::Error => unimplemented!(),
            Type::SelfType => unimplemented!(),
            Type::Solidity(_) => unimplemented!(),
            Type::TypeState => 1,
        }
    }
}
