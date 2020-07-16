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
                let mut c1_canididates = c1.candidates.clone();
                let mut c2_canididates = c2.candidates.clone();
                c1_canididates.append(&mut c2_canididates);
                FunctionCallMatchResult::Failure(Candidates {
                    candidates: c1_canididates,
                })
            } else {
                f
            }
        } else {
            self.clone()
        }
    }
}

#[derive(Debug, Clone)]
pub enum CallableInformation {
    FunctionInformation(FunctionInformation),
    SpecialInformation(SpecialInformation),
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

    fn add_conformance(&mut self, t: &TypeIdentifier, conformance_identifier: &TypeIdentifier) {
        let trait_info = &self.types.get(conformance_identifier);
        let type_info = &self.types.get(t);
        if trait_info.is_some() && type_info.is_some() {
            let conformance = self.types.get(conformance_identifier).unwrap().clone();
            self.types
                .get_mut(t)
                .unwrap()
                .conformances
                .push(conformance);
        }
    }

    fn add_type_state(&mut self, t: &TypeIdentifier, type_state: TypeState) {
        if let Some(type_info) = self.types.get_mut(t) {
            type_info.type_states.push(type_state);
        }
    }

    pub fn add_special(
        &mut self,
        s: &SpecialDeclaration,
        t: &TypeIdentifier,
        caller_protections: Vec<CallerProtection>,
        type_states: Vec<TypeState>,
    ) {
        if s.is_init() {
            if s.is_public() {
                let type_info = &self.types.get_mut(t);
                if type_info.is_some() {
                    self.types.get_mut(t).unwrap().public_initializer = Some(s.clone());
                }
            }
            let type_info = &self.types.get_mut(t);
            if type_info.is_some() {
                self.types
                    .get_mut(t)
                    .unwrap()
                    .initialisers
                    .push(SpecialInformation {
                        declaration: s.clone(),
                        type_states,
                        caller_protections,
                    });
            }
        } else {
            let type_info = &self.types.get_mut(t);
            if type_info.is_some() {
                self.types
                    .get_mut(t)
                    .unwrap()
                    .fallbacks
                    .push(SpecialInformation {
                        declaration: s.clone(),
                        type_states,
                        caller_protections,
                    });
            }
        }
    }

    pub fn contains_caller_protection(&self, c: &CallerProtection, t: &TypeIdentifier) -> bool {
        self.declared_caller_protections(t).contains(&c.name())
    }

    fn declared_caller_protections(&self, t: &TypeIdentifier) -> Vec<String> {
        let type_info = self.types.get(t);
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
            if f.declaration.get_result_type().is_some() {
                if f.get_result_type().unwrap().is_address_type()
                    && f.get_parameter_types().is_empty()
                {
                    return true;
                }
                if f.get_result_type().unwrap().is_bool_type() && f.get_parameter_types().len() == 1
                {
                    let element = f.get_parameter_types().remove(0);
                    if element.is_address_type() {
                        return true;
                    }
                }
                return false;
            }
            false
        };
        if type_info.is_some() {
            let mut properties: Vec<String> = type_info
                .unwrap()
                .properties
                .clone()
                .into_iter()
                .filter(|(_, v)| caller_protection_property(v))
                .map(|(k, _)| k)
                .collect();

            let functions: HashMap<String, Vec<FunctionInformation>> = self
                .types
                .get(t)
                .unwrap()
                .functions
                .clone()
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        v.clone()
                            .into_iter()
                            .filter(|f| caller_protection_function(f))
                            .collect(),
                    )
                })
                .collect();
            let mut functions: Vec<String> = functions
                .into_iter()
                .filter(|(_, v)| !v.is_empty())
                .map(|(k, _)| k)
                .collect();

            properties.append(&mut functions);

            return properties;
        }

        Vec::new()
    }

    fn external_trait_init() -> SpecialSignatureDeclaration {
        SpecialSignatureDeclaration {
            special_token: "init".to_string(),
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

    pub fn has_public_initialiser(&mut self, t: &TypeIdentifier) -> bool {
        self.types.get_mut(t).unwrap().public_initializer.is_some()
    }

    pub fn is_contract_declared(&self, t: &TypeIdentifier) -> bool {
        let contract = &self.contract_declarations.iter().find(|&x| x.token.eq(t));
        if contract.is_none() {
            return false;
        }
        true
    }

    pub fn is_contract_stateful(&self, t: &TypeIdentifier) -> bool {
        if let Some(contract_info) = self.types.get(t) {
            !contract_info.type_states.is_empty()
        } else {
            panic!("Contract {} does not exist!", t)
        }
    }

    pub fn is_state_declared(&self, state: &TypeState, t: &TypeIdentifier) -> bool {
        if let Some(contract_info) = self.types.get(t) {
            contract_info.type_states.contains(state)
        } else {
            panic!("Contract {} does not exist", t)
        }
    }

    pub fn is_struct_declared(&self, t: &TypeIdentifier) -> bool {
        let struct_decl = &self.struct_declarations.iter().find(|&x| x.token.eq(t));
        if struct_decl.is_none() {
            return false;
        }
        true
    }

    pub fn is_trait_declared(&self, t: &TypeIdentifier) -> bool {
        let identifier = &self.trait_declarations.iter().find(|&x| x.token.eq(t));
        if identifier.is_none() {
            return false;
        }
        true
    }

    pub fn is_asset_declared(&self, t: &TypeIdentifier) -> bool {
        let identifier = &self.asset_declarations.iter().find(|&x| x.token.eq(t));
        if identifier.is_none() {
            return false;
        }
        true
    }

    pub fn is_enum_declared(&self, t: &TypeIdentifier) -> bool {
        let enum_declaration = &self.enum_declarations.iter().find(|&x| x.token.eq(t));
        if enum_declaration.is_none() {
            return false;
        }
        true
    }

    pub fn is_recursive_struct(&self, t: &TypeIdentifier) -> bool {
        let properties = &self.types.get(t).unwrap().ordered_properties;

        for property in properties {
            let type_property = self.types.get(t).unwrap().properties.get(property);
            if type_property.is_some() {
                match type_property.unwrap().get_type() {
                    Type::UserDefinedType(i) => return i.token == t.to_string(),
                    _ => {
                        return false;
                    }
                }
            }
        }
        false
    }

    pub fn is_type_declared(&self, t: &TypeIdentifier) -> bool {
        self.types.get(t).is_some()
    }

    pub fn is_initiliase_call(&self, function_call: FunctionCall) -> bool {
        self.is_struct_declared(&function_call.identifier.token)
            || self.is_asset_declared(&function_call.identifier.token)
    }

    pub fn type_size(&self, input_type: Type) -> u64 {
        match input_type {
            Type::Bool => 1,
            Type::Int => 1,
            Type::String => 1,
            Type::Address => 1,
            Type::InoutType(_) => unimplemented!(),
            Type::ArrayType(_) => 1,
            Type::RangeType(_) => unimplemented!(),
            Type::FixedSizedArrayType(a) => {
                let key_size = self.type_size(*a.key_type.clone());
                let size = a.size;
                key_size * size
            }
            Type::DictionaryType(_) => unimplemented!(),
            Type::UserDefinedType(i) => {
                if self.is_enum_declared(&i.token) {
                    unimplemented!()
                }

                let mut acc = 0;
                let enclosing = self.types.get(&i.token);
                let enclosing = enclosing.unwrap();
                let enclosing_properties = enclosing.properties.clone();

                for (_, v) in enclosing_properties {
                    acc = acc + self.type_size(v.property.get_type())
                }

                acc
            }
            Type::Error => unimplemented!(),
            Type::SelfType => unimplemented!(),
            Type::Solidity(_) => unimplemented!(),
        }
    }
}
