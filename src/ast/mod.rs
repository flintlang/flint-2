
use crate::type_checker::ExpressionCheck;
use std::cmp::max;
use std::collections::HashMap;
use std::error::Error;
use std::string::String;
use std::vec::Vec;

use hex::encode;
use sha3::{Digest, Keccak256};

use super::context::*;
use super::environment::*;
use super::visitor::*;

pub type VResult = Result<(), Box<dyn Error>>;

pub type PResult = Result<PassResult, Box<dyn Error>>;

pub struct PassResult {
    pub context: Context,
}

pub type TypeIdentifier = String;

#[derive(Clone, Default, Debug, PartialEq)]
pub struct LineInfo {
    pub line: u32,
    pub offset: usize,
}

#[derive(Default, Debug, Clone)]
pub struct TypeInfo {
    pub ordered_properties: Vec<String>,
    pub properties: HashMap<String, PropertyInformation>,
    pub functions: HashMap<String, Vec<FunctionInformation>>,
    pub initialisers: Vec<SpecialInformation>,
    pub fallbacks: Vec<SpecialInformation>,
    pub public_initializer: Option<SpecialDeclaration>,
    pub conformances: Vec<TypeInfo>,
    pub modifiers: Vec<FunctionCall>,
}

impl TypeInfo {
    pub fn all_functions(&self) -> HashMap<String, Vec<FunctionInformation>> {
        self.functions.clone()
    }

    pub fn trait_functions(&self) -> HashMap<String, Vec<FunctionInformation>> {
        let conformances = self.conformances.clone();
        conformances
            .into_iter()
            .map(|c| c.functions)
            .flatten()
            .collect()
    }

    pub fn is_external_module(&self) -> bool {
        let modifiers = self.modifiers.clone();
        let modifiers: Vec<FunctionCall> = modifiers
            .into_iter()
            .filter(|f| f.identifier.token == "module".to_string())
            .collect();

        if modifiers.is_empty() {
            return false;
        }

        true
    }

    pub fn is_external_resource(&self) -> bool {
        let modifiers = self.modifiers.clone();
        let modifiers: Vec<FunctionCall> = modifiers
            .into_iter()
            .filter(|f| f.identifier.token == "resource".to_string())
            .collect();

        if modifiers.is_empty() {
            return false;
        }

        true
    }

    pub fn is_external_struct(&self) -> bool {
        let modifiers = self.modifiers.clone();
        let modifiers: Vec<FunctionCall> = modifiers
            .into_iter()
            .filter(|f| {
                f.identifier.token == "resource".to_string()
                    || f.identifier.token == "struct".to_string()
            })
            .collect();

        if modifiers.is_empty() {
            return false;
        }

        true
    }
}

#[derive(Debug, Clone)]
pub struct PropertyInformation {
    pub property: Property,
}

impl PropertyInformation {
    pub(crate) fn get_type(&self) -> &Type {
        match &self.property {
            Property::VariableDeclaration(v) => &v.variable_type,
            Property::EnumCase(e) => &e.enum_type,
        }
    }

    pub fn is_constant(&self) -> bool {
        match &self.property {
            Property::EnumCase(_) => true,
            Property::VariableDeclaration(v) => v.is_constant(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Property {
    VariableDeclaration(VariableDeclaration),
    EnumCase(EnumMember),
}

impl Property {
    pub fn get_identifier(&self) -> Identifier {
        match self {
            Property::VariableDeclaration(v) => v.identifier.clone(),
            Property::EnumCase(e) => e.identifier.clone(),
        }
    }

    pub fn get_type(&self) -> Type {
        match self {
            Property::VariableDeclaration(v) => v.variable_type.clone(),
            Property::EnumCase(e) => e.enum_type.clone(),
        }
    }

    pub fn get_value(&self) -> Option<Expression> {
        match self {
            Property::VariableDeclaration(v) => {
                let expression = v.expression.clone();
                match expression {
                    None => None,
                    Some(e) => Some(*e),
                }
            }
            Property::EnumCase(e) => e.hidden_value.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpecialInformation {
    pub declaration: SpecialDeclaration,
    pub caller_protections: Vec<CallerProtection>,
}

impl SpecialInformation {
    pub fn parameter_types(&self) -> Vec<Type> {
        self.declaration
            .head
            .parameters
            .clone()
            .into_iter()
            .map(|p| p.type_assignment)
            .collect()
    }
}

#[derive(Default, Debug, Clone)]
pub struct FunctionInformation {
    pub declaration: FunctionDeclaration,
    pub caller_protection: Vec<CallerProtection>,
    pub type_states: Vec<TypeState>,
    pub mutating: bool,
    pub is_signature: bool,
}

impl FunctionInformation {
    pub fn get_result_type(&self) -> Option<Type> {
        self.declaration.get_result_type()
    }

    pub fn get_parameter_types(&self) -> Vec<Type> {
        self.declaration.head.parameter_types()
    }

    pub fn parameter_identifiers(&self) -> Vec<Identifier> {
        self.declaration.head.parameter_identifiers()
    }

    pub fn required_parameter_identifiers(&self) -> Vec<Identifier> {
        let identifiers = self.declaration.head.parameters.clone();
        identifiers
            .into_iter()
            .filter(|i| i.expression.is_none())
            .map(|p| p.identifier.clone())
            .collect()
    }
}

pub trait Visitable {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub declarations: Vec<TopLevelDeclaration>,
}

impl Visitable for Module {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_module(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = self.declarations.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = v.finish_module(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

impl<T: Visitable> Visitable for Vec<T> {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        for t in self {
            let result = t.visit(v, ctx);
            match result {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TopLevelDeclaration {
    ContractDeclaration(ContractDeclaration),
    ContractBehaviourDeclaration(ContractBehaviourDeclaration),
    StructDeclaration(StructDeclaration),
    AssetDeclaration(AssetDeclaration),
    EnumDeclaration(EnumDeclaration),
    TraitDeclaration(TraitDeclaration),
}

impl TopLevelDeclaration {
    pub fn is_contract_behaviour_declaration(&self) -> bool {
        match self {
            TopLevelDeclaration::ContractBehaviourDeclaration(_) => true,
            _ => false,
        }
    }
}

impl Visitable for TopLevelDeclaration {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_top_level_declaration(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = match self {
            TopLevelDeclaration::ContractDeclaration(c) => c.visit(v, ctx),
            TopLevelDeclaration::ContractBehaviourDeclaration(c) => c.visit(v, ctx),
            TopLevelDeclaration::StructDeclaration(s) => s.visit(v, ctx),
            TopLevelDeclaration::EnumDeclaration(e) => e.visit(v, ctx),
            TopLevelDeclaration::TraitDeclaration(t) => t.visit(v, ctx),
            TopLevelDeclaration::AssetDeclaration(a) => a.visit(v, ctx),
        };
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = v.finish_top_level_declaration(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct ContractDeclaration {
    pub identifier: Identifier,
    pub contract_members: Vec<ContractMember>,
    pub conformances: Vec<Conformance>,
}

impl ContractDeclaration {
    pub fn contract_enum_prefix() -> String {
        "QuartzStateEnum$".to_string()
    }

    pub fn get_variable_declarations(&self) -> Vec<VariableDeclaration> {
        let members = self.contract_members.clone();
        members
            .into_iter()
            .filter_map(|c| match c {
                ContractMember::VariableDeclaration(v) => Some(v),
                ContractMember::EventDeclaration(_) => None,
            })
            .collect()
    }

    pub fn get_variable_declarations_without_dict(&self) -> Vec<VariableDeclaration> {
        let members = self.contract_members.clone();
        members
            .into_iter()
            .filter_map(|c| match c {
                ContractMember::VariableDeclaration(v) => {
                    if v.clone().variable_type.is_dictionary_type() {
                        None
                    } else {
                        Some(v)
                    }
                }
                ContractMember::EventDeclaration(_) => None,
            })
            .collect()
    }
}

impl Visitable for ContractDeclaration {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_contract_declaration(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = self.identifier.visit(v, ctx);

        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        ctx.contract_declaration_context = Some(ContractDeclarationContext {
            identifier: self.identifier.clone(),
        });

        let result = self.conformances.visit(v, ctx);

        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = self.contract_members.visit(v, ctx);

        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        ctx.contract_declaration_context = None;

        let result = v.finish_contract_declaration(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContractBehaviourDeclaration {
    pub identifier: Identifier,
    pub members: Vec<ContractBehaviourMember>,
    pub states: Vec<TypeState>,
    pub caller_binding: Option<Identifier>,
    pub caller_protections: Vec<CallerProtection>,
}

impl Visitable for ContractBehaviourDeclaration {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        ctx.contract_behaviour_declaration_context = Some(ContractBehaviourDeclarationContext {
            identifier: self.identifier.clone(),
            caller: self.caller_binding.clone(),
            caller_protections: self.caller_protections.clone(),
        });

        let mut local_variables: Vec<VariableDeclaration> = vec![];
        if self.caller_binding.is_some() {
            let caller_binding = self.caller_binding.clone();
            let caller_binding = caller_binding.unwrap();
            local_variables.push(VariableDeclaration {
                declaration_token: None,
                identifier: caller_binding,
                variable_type: Type::Address,
                expression: None,
            })
        }
        let scope = ScopeContext {
            parameters: vec![],
            local_variables,
            ..Default::default()
        };
        ctx.scope_context = Some(scope);

        let result = v.start_contract_behaviour_declaration(self, ctx);

        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = self.identifier.visit(v, ctx);

        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        if self.caller_binding.is_some() {
            let caller = self.caller_binding.clone();
            let mut caller = caller.unwrap();

            let result = caller.visit(v, ctx);
            match result {
                Ok(_) => {}
                Err(e) => return Err(e),
            }

            self.caller_binding = Some(caller);
        }

        let result = self.caller_protections.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let scope = ctx.scope_context.clone();

        for member in &mut self.members {
            ctx.scope_context = scope.clone();
            let result = member.visit(v, ctx);
            match result {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }

        ctx.contract_behaviour_declaration_context = None;
        ctx.scope_context = None;

        let result = v.finish_contract_behaviour_declaration(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssetDeclaration {
    pub identifier: Identifier,
    pub members: Vec<AssetMember>,
}

impl AssetDeclaration {
    pub fn get_variable_declarations(&self) -> Vec<VariableDeclaration> {
        let members = self.members.clone();
        members
            .into_iter()
            .filter_map(|m| {
                if let AssetMember::VariableDeclaration(v) = m {
                    Some(v)
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Visitable for AssetDeclaration {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_asset_declaration(self, ctx);

        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let asset_declaration_context = AssetDeclarationContext {
            identifier: self.identifier.clone(),
        };

        let scope_context = ScopeContext {
            parameters: vec![],
            local_variables: vec![],
            counter: 0,
        };
        let scope_context = Some(scope_context);

        ctx.asset_context = Option::from(asset_declaration_context);
        ctx.scope_context = scope_context;

        let result = self.identifier.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        for member in &mut self.members {
            ctx.scope_context = Option::from(ScopeContext {
                parameters: vec![],
                local_variables: vec![],
                counter: 0,
            });
            let result = member.visit(v, ctx);
            match result {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }

        ctx.asset_context = None;
        ctx.scope_context = None;

        let result = v.finish_asset_declaration(self, ctx);

        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssetMember {
    VariableDeclaration(VariableDeclaration),
    FunctionDeclaration(FunctionDeclaration),
    SpecialDeclaration(SpecialDeclaration),
}

impl Visitable for AssetMember {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = match self {
            AssetMember::VariableDeclaration(d) => d.visit(v, ctx),
            AssetMember::SpecialDeclaration(s) => s.visit(v, ctx),
            AssetMember::FunctionDeclaration(f) => f.visit(v, ctx),
        };
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDeclaration {
    pub identifier: Identifier,
    pub conformances: Vec<Conformance>,
    pub members: Vec<StructMember>,
}

impl StructDeclaration {
    pub fn get_variable_declarations(&self) -> Vec<VariableDeclaration> {
        let members = self.members.clone();
        members
            .into_iter()
            .filter_map(|m| {
                if let StructMember::VariableDeclaration(v) = m {
                    Some(v)
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Visitable for StructDeclaration {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_struct_declaration(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let struct_declaration_context = Some(StructDeclarationContext {
            identifier: self.identifier.clone(),
        });
        let scope_context = Some(ScopeContext {
            ..Default::default()
        });

        ctx.struct_declaration_context = struct_declaration_context;
        ctx.scope_context = scope_context;

        let result = self.identifier.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        for member in &mut self.members {
            ctx.scope_context = Option::from(ScopeContext {
                parameters: vec![],
                local_variables: vec![],
                counter: 0,
            });
            let result = member.visit(v, ctx);
            match result {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }
        ctx.struct_declaration_context = None;
        ctx.scope_context = None;

        v.finish_struct_declaration(self, ctx);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumDeclaration {
    pub enum_token: std::string::String,
    pub identifier: Identifier,
    pub type_assigned: Option<Type>,
    pub members: Vec<EnumMember>,
}

impl Visitable for EnumDeclaration {
    fn visit(&mut self, _v: &mut dyn Visitor, _ctx: &mut Context) -> VResult {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitDeclaration {
    pub external: bool,
    pub identifier: Identifier,
    pub members: Vec<TraitMember>,
    pub modifiers: Vec<FunctionCall>,
}

impl TraitDeclaration {
    pub fn get_module_address(&self) -> Option<String> {
        let modifiers = self.modifiers.clone();
        let mut modifiers: Vec<FunctionCall> = modifiers
            .into_iter()
            .filter(|f| f.identifier.token == "module".to_string())
            .collect();

        if modifiers.is_empty() {
            return None;
        }

        let modifier = modifiers.remove(0);
        let mut argument = modifier.arguments.clone();
        if !argument.is_empty() {
            let argument = argument.remove(0);
            if argument.identifier.is_some() {
                let identifier = argument.identifier.clone();
                let identifier = identifier.unwrap();
                let name = identifier.token;
                if name == "address".to_string() {
                    if let Expression::Literal(l) = argument.expression {
                        if let Literal::AddressLiteral(a) = l {
                            return Option::from(a);
                        }
                    }
                }
            }
        }

        None
    }
}

impl Visitable for TraitDeclaration {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_trait_declaration(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = self.identifier.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let trait_declaration_context = TraitDeclarationContext {
            identifier: self.identifier.clone(),
        };
        let trait_scope_ctx = ScopeContext {
            parameters: vec![],
            local_variables: vec![],
            counter: 0,
        };

        ctx.trait_declaration_context = Some(trait_declaration_context);

        ctx.scope_context = Option::from(trait_scope_ctx.clone());

        for member in &mut self.members {
            ctx.scope_context = Some(trait_scope_ctx.clone());
            let result = member.visit(v, ctx);
            match result {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }

        ctx.trait_declaration_context = None;

        ctx.scope_context = None;

        v.finish_trait_declaration(self, ctx);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContractMember {
    VariableDeclaration(VariableDeclaration),
    EventDeclaration(EventDeclaration),
}

impl Visitable for ContractMember {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_contract_member(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = match self {
            ContractMember::VariableDeclaration(d) => d.visit(v, ctx),
            ContractMember::EventDeclaration(d) => d.visit(v, ctx),
        };
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = v.finish_contract_member(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContractBehaviourMember {
    FunctionDeclaration(FunctionDeclaration),
    SpecialDeclaration(SpecialDeclaration),
    FunctionSignatureDeclaration(FunctionSignatureDeclaration),
    SpecialSignatureDeclaration(SpecialSignatureDeclaration),
}

impl Visitable for ContractBehaviourMember {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_contract_behaviour_member(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = match self {
            ContractBehaviourMember::FunctionDeclaration(f) => f.visit(v, ctx),
            ContractBehaviourMember::SpecialDeclaration(s) => s.visit(v, ctx),
            ContractBehaviourMember::FunctionSignatureDeclaration(f) => f.visit(v, ctx),
            ContractBehaviourMember::SpecialSignatureDeclaration(s) => s.visit(v, ctx),
        };
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = v.finish_contract_behaviour_member(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumMember {
    pub case_token: std::string::String,
    pub identifier: Identifier,
    pub hidden_value: Option<Expression>,
    pub enum_type: Type,
}

impl Visitable for EnumMember {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_enum_member(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = self.identifier.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = v.finish_enum_member(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TraitMember {
    FunctionDeclaration(FunctionDeclaration),
    SpecialDeclaration(SpecialDeclaration),
    FunctionSignatureDeclaration(FunctionSignatureDeclaration),
    SpecialSignatureDeclaration(SpecialSignatureDeclaration),
    ContractBehaviourDeclaration(ContractBehaviourDeclaration),
    EventDeclaration(EventDeclaration),
}

impl Visitable for TraitMember {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = match self {
            TraitMember::FunctionDeclaration(f) => f.visit(v, ctx),
            TraitMember::SpecialDeclaration(s) => s.visit(v, ctx),
            TraitMember::FunctionSignatureDeclaration(f) => f.visit(v, ctx),
            TraitMember::SpecialSignatureDeclaration(s) => s.visit(v, ctx),
            TraitMember::ContractBehaviourDeclaration(c) => c.visit(v, ctx),
            TraitMember::EventDeclaration(e) => e.visit(v, ctx),
        };
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StructMember {
    VariableDeclaration(VariableDeclaration),
    FunctionDeclaration(FunctionDeclaration),
    SpecialDeclaration(SpecialDeclaration),
}

impl Visitable for StructMember {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_struct_member(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = match self {
            StructMember::FunctionDeclaration(f) => f.visit(v, ctx),
            StructMember::SpecialDeclaration(s) => s.visit(v, ctx),
            StructMember::VariableDeclaration(d) => d.visit(v, ctx),
        };
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = v.finish_struct_member(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Conformance {
    pub identifier: Identifier,
}

impl Conformance {
    pub fn name(&self) -> String {
        self.identifier.token.clone()
    }
}

impl Visitable for Conformance {
    fn visit(&mut self, _v: &mut dyn Visitor, _ctx: &mut Context) -> VResult {
        Ok(())
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct TypeState {
    pub identifier: Identifier,
}

impl TypeState {
    pub fn is_any(&self) -> bool {
        self.identifier.token == "any"
    }
}

impl Visitable for TypeState {
    fn visit(&mut self, _v: &mut dyn Visitor, _ctx: &mut Context) -> VResult {
        Ok(())
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct CallerProtection {
    pub identifier: Identifier,
}

impl CallerProtection {
    pub fn is_any(&self) -> bool {
        self.identifier.token.eq("any")
    }

    pub fn name(&self) -> String {
        self.identifier.token.clone()
    }

    pub fn is_sub_protection(&self, parent: CallerProtection) -> bool {
        parent.is_any() || self.name() == parent.name()
    }
}

impl Visitable for CallerProtection {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_caller_protection(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = self.identifier.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = v.finish_caller_protection(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct FunctionDeclaration {
    pub head: FunctionSignatureDeclaration,
    pub body: Vec<Statement>,
    pub scope_context: Option<ScopeContext>,
    pub tags: Vec<String>,
    pub mangled_identifier: Option<String>,
    pub is_external: bool,
}

impl FunctionDeclaration {
    pub fn is_mutating(&self) -> bool {
        !self.head.mutates.is_empty()
    }
    pub fn is_payable(&self) -> bool {
        self.head.is_payable()
    }

    pub fn first_payable_param(&self) -> Option<Parameter> {
        if !self.is_payable() {
            return None;
        }

        let parameters = self.head.parameters.clone();
        let mut parameters: Vec<Parameter> = parameters
            .into_iter()
            .filter(|p| p.type_assignment.is_currency_type())
            .collect();

        if !parameters.is_empty() {
            return Option::from(parameters.remove(0));
        }
        None
    }
    pub fn is_public(&self) -> bool {
        self.head.is_public()
    }

    pub fn get_result_type(&self) -> Option<Type> {
        self.head.result_type.clone()
    }

    pub fn is_void(&self) -> bool {
        self.head.result_type.is_none()
    }

    pub fn mutates(&self) -> Vec<Identifier> {
        self.head.mutates.clone()
    }

    pub fn parameters_and_types(&self) -> Vec<(String, Type)> {
        self.head
            .parameters
            .clone()
            .into_iter()
            .map(|p| (p.identifier.token, p.type_assignment))
            .collect()
    }

    pub fn external_signature_hash(&self) -> String {
        if self.is_external {
            let args = self.head.parameters.clone();
            let args: Vec<String> = args.into_iter().map(|a| a.type_assignment.name()).collect();
            let args = args.join(",");
            let name = self.head.identifier.token.clone();
            let args = format!("{name}({args})", name = name, args = args);
            let hash = Keccak256::digest(args.as_bytes());
            let mut hex = encode(hash);
            hex.truncate(8);
            return hex;
        }
        unimplemented!()
    }
}

impl Visitable for FunctionDeclaration {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_function_declaration(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = self.head.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let local_variables = {
            if ctx.has_scope_context() {
                let scope = ctx.scope_context.clone();
                let scope = scope.unwrap();
                scope.local_variables
            } else {
                vec![]
            }
        };
        ctx.function_declaration_context = Some(FunctionDeclarationContext {
            declaration: self.clone(),
            local_variables,
        });

        if ctx.scope_context.is_some() {
            for parameter in &self.head.parameters {
                ctx.scope_context
                    .as_mut()
                    .unwrap()
                    .parameters
                    .push(parameter.clone());
            }
        }

        let mut statements: Vec<Vec<Statement>> = vec![];

        for statement in &mut self.body {
            ctx.pre_statements = vec![];
            ctx.post_statements = vec![];
            let result = statement.visit(v, ctx);
            match result {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
            statements.push(ctx.pre_statements.clone());
            statements.push(ctx.post_statements.clone());
        }

        let body = self.body.clone();
        let mut counter = 1;
        for statement in body {
            statements.insert(counter, vec![statement]);
            counter = counter + 3;
        }

        let statements: Vec<Statement> = statements.into_iter().flatten().collect();

        self.body = statements;

        let declarations = ctx.function_declaration_context.clone();
        let declarations = declarations.unwrap().local_variables;
        if ctx.has_scope_context() {
            let scope = ctx.scope_context.clone();
            let mut scope = scope.unwrap();
            scope.local_variables = declarations;
            ctx.scope_context = Some(scope);
        }
        ctx.function_declaration_context = None;

        let result = v.finish_function_declaration(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        ctx.pre_statements = vec![];
        ctx.post_statements = vec![];

        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct FunctionSignatureDeclaration {
    pub func_token: std::string::String,
    pub attributes: Vec<Attribute>,
    pub modifiers: Vec<std::string::String>,
    pub mutates: Vec<Identifier>,
    pub identifier: Identifier,
    pub parameters: Vec<Parameter>,
    pub result_type: Option<Type>,
    pub payable: bool,
}

impl FunctionSignatureDeclaration {
    pub fn is_payable(&self) -> bool {
        self.payable
    }

    pub fn is_public(&self) -> bool {
        self.modifiers.contains(&"public".to_string())
    }

    pub fn parameter_identifiers(&self) -> Vec<Identifier> {
        self.parameters
            .clone()
            .into_iter()
            .map(|p| p.identifier)
            .collect()
    }

    pub fn parameter_types(&self) -> Vec<Type> {
        self.parameters
            .clone()
            .into_iter()
            .map(|p| p.type_assignment)
            .collect()
    }

    pub fn is_equal(&self, against: FunctionSignatureDeclaration) -> bool {
        let modifiers_match = do_vecs_match(&self.modifiers.clone(), &against.modifiers.clone());
        let attibutes_match = do_vecs_match(&self.attributes.clone(), &against.attributes.clone());
        let parameter_names_match = do_vecs_match(
            &self.parameter_identifiers().clone(),
            &against.parameter_identifiers().clone(),
        );
        let parameter_types = do_vecs_match(
            &self.parameter_types().clone(),
            &against.parameter_types().clone(),
        );
        if self.identifier.token.clone() == against.identifier.token.clone()
            && modifiers_match
            && attibutes_match
            && parameter_names_match
            && parameter_types
        {
            return true;
        }

        false
    }
}

impl Visitable for FunctionSignatureDeclaration {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_function_signature_declaration(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = self.identifier.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = self.parameters.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        if self.result_type.is_some() {
            let result_type = self.result_type.clone();
            let mut result_type = result_type.unwrap();
            let result = result_type.visit(v, ctx);
            match result {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
            self.result_type = Some(result_type);
        }

        let result = v.finish_function_signature_declaration(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpecialDeclaration {
    pub head: SpecialSignatureDeclaration,
    pub body: Vec<Statement>,
    pub scope_context: ScopeContext,
    pub generated: bool,
}

impl SpecialDeclaration {
    pub(crate) fn is_init(&self) -> bool {
        &self.head.special_token == "init"
    }

    pub fn is_fallback(&self) -> bool {
        &self.head.special_token == "fallback"
    }

    pub(crate) fn is_public(&self) -> bool {
        let modifiers = &self.head.modifiers;
        for modifier in modifiers {
            if modifier == "public" {
                return true;
            }
        }
        false
    }

    pub fn as_function_declaration(&self) -> FunctionDeclaration {
        let identifier = Identifier {
            token: self.head.special_token.clone(),
            enclosing_type: None,
            line_info: Default::default(),
        };

        let function_sig = FunctionSignatureDeclaration {
            func_token: self.head.special_token.clone(),
            attributes: self.head.attributes.clone(),
            modifiers: self.head.modifiers.clone(),
            mutates: self.head.mutates.clone(),
            identifier,
            parameters: self.head.parameters.clone(),
            result_type: None,
            payable: false,
        };

        FunctionDeclaration {
            head: function_sig,
            body: self.body.clone(),
            scope_context: Option::from(self.scope_context.clone()),
            tags: vec![],
            mangled_identifier: None,
            is_external: false,
        }
    }
}

impl Visitable for SpecialDeclaration {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_special_declaration(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = self.head.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let local_variables = {
            if ctx.has_scope_context() {
                let scope = ctx.scope_context.clone();
                let scope = scope.unwrap();
                scope.local_variables
            } else {
                vec![]
            }
        };
        ctx.special_declaration_context = Some(SpecialDeclarationContext {
            declaration: self.clone(),
            local_variables,
        });

        if ctx.scope_context.is_some() {
            for parameter in &self.head.parameters {
                ctx.scope_context
                    .as_mut()
                    .unwrap()
                    .parameters
                    .push(parameter.clone());
            }
        }

        let mut statements: Vec<Vec<Statement>> = vec![];
        for statement in &mut self.body {
            ctx.pre_statements = vec![];
            ctx.post_statements = vec![];
            let result = statement.visit(v, ctx);
            match result {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
            statements.push(ctx.pre_statements.clone());
            statements.push(ctx.post_statements.clone());
        }

        let body = self.body.clone();
        let mut counter = 1;
        for statement in body {
            statements.insert(counter, vec![statement]);
            counter = counter + 3;
        }

        let statements: Vec<Statement> = statements.into_iter().flatten().collect();

        self.body = statements;

        let declarations = ctx.special_declaration_context.clone();
        let declarations = declarations.unwrap().local_variables;
        if ctx.has_scope_context() {
            let scope = ctx.scope_context.clone();
            let mut scope = scope.unwrap();
            scope.local_variables = declarations;
            ctx.scope_context = Some(scope);
        }
        ctx.special_declaration_context = None;
        let result = v.finish_special_declaration(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpecialSignatureDeclaration {
    pub special_token: std::string::String,
    pub attributes: Vec<Attribute>,
    pub modifiers: Vec<std::string::String>,
    pub mutates: Vec<Identifier>,
    pub parameters: Vec<Parameter>,
}

impl SpecialSignatureDeclaration {
    pub fn has_parameters(&self) -> bool {
        !self.parameters.is_empty()
    }
}

impl Visitable for SpecialSignatureDeclaration {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_special_signature_declaration(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = self.parameters.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = v.finish_special_signature_declaration(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Statement {
    ReturnStatement(ReturnStatement),
    Expression(Expression),
    BecomeStatement(BecomeStatement),
    EmitStatement(EmitStatement),
    ForStatement(ForStatement),
    IfStatement(IfStatement),
    DoCatchStatement(DoCatchStatement),
}

impl Statement {
    pub fn is_expression(&self) -> bool {
        match self {
            Statement::Expression(_) => true,
            _ => false,
        }
    }
}

impl Visitable for Statement {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_statement(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = match self {
            Statement::ReturnStatement(r) => r.visit(v, ctx),
            Statement::Expression(e) => e.visit(v, ctx),
            Statement::BecomeStatement(b) => b.visit(v, ctx),
            Statement::EmitStatement(e) => e.visit(v, ctx),
            Statement::ForStatement(f) => f.visit(v, ctx),
            Statement::IfStatement(i) => i.visit(v, ctx),
            Statement::DoCatchStatement(d) => d.visit(v, ctx),
        };
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = v.finish_statement(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DoCatchStatement {
    pub error: Expression,
    pub do_body: Vec<Statement>,
    pub catch_body: Vec<Statement>,
}

impl Visitable for DoCatchStatement {
    fn visit(&mut self, _v: &mut dyn Visitor, _ctx: &mut Context) -> VResult {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IfStatement {
    pub condition: Expression,
    pub body: Vec<Statement>,
    pub else_body: Vec<Statement>,
    pub if_body_scope_context: Option<ScopeContext>,
    pub else_body_scope_context: Option<ScopeContext>,
}

impl IfStatement {
    pub fn ends_with_return(&self) -> bool {
        let body = self.body.clone();
        for b in body {
            if let Statement::ReturnStatement(_) = b {
                return true;
            }
        }
        false
    }
}

impl Visitable for IfStatement {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_if_statement(self, ctx);

        ctx.in_if_condition = true;

        self.condition.visit(v, ctx);

        ctx.in_if_condition = false;

        let pre_statements = ctx.pre_statements.clone();
        let post_statements = ctx.post_statements.clone();
        let scope = ctx.scope_context.clone();
        let block = ctx.block_context.clone();

        let blocks_scope = if self.if_body_scope_context.is_some() {
            let temp = self.if_body_scope_context.clone();
            temp.unwrap()
        } else {
            let temp = ctx.scope_context.clone();
            temp.unwrap()
        };
        let block_context = BlockContext {
            scope_context: blocks_scope,
        };

        ctx.block_context = Some(block_context);
        let mut statements: Vec<Vec<Statement>> = vec![];
        for statement in &mut self.body {
            ctx.pre_statements = vec![];
            ctx.post_statements = vec![];
            statement.visit(v, ctx);
            statements.push(ctx.pre_statements.clone());
            statements.push(ctx.post_statements.clone());
        }

        let body = self.body.clone();
        let mut counter = 1;
        for statement in body {
            statements.insert(counter, vec![statement]);
            counter = counter + 3;
        }

        let statements: Vec<Statement> = statements.into_iter().flatten().collect();

        self.body = statements;

        if self.if_body_scope_context.is_none() {
            self.if_body_scope_context = ctx.scope_context.clone();
        } else if ctx.block_context.is_some() {
            let block = ctx.block_context.clone();
            let block = block.unwrap();
            self.if_body_scope_context = Option::from(block.scope_context.clone());
        }

        if scope.is_some() {
            let temp_scope = scope.clone();
            let mut temp_scope = temp_scope.unwrap();

            temp_scope.counter = if ctx.scope_context().is_some() {
                let ctx_scope = ctx.scope_context.clone();
                let ctx_scope = ctx_scope.unwrap();

                temp_scope.counter + ctx_scope.local_variables.len() as u64
            } else {
                temp_scope.counter + 1
            };

            temp_scope.counter = if ctx.block_context.is_some() {
                let ctx_block = ctx.block_context.clone();
                let ctx_scope = ctx_block.unwrap();
                let ctx_scope = ctx_scope.scope_context;
                temp_scope.counter + ctx_scope.local_variables.len() as u64
            } else {
                temp_scope.counter + 1
            };

            ctx.scope_context = Option::from(temp_scope);
        }

        let blocks_scope = if self.else_body_scope_context.is_some() {
            let temp = self.else_body_scope_context.clone();
            temp.unwrap()
        } else {
            let temp = ctx.scope_context.clone();
            temp.unwrap()
        };
        let block_context = BlockContext {
            scope_context: blocks_scope,
        };

        ctx.block_context = Some(block_context);

        let mut statements: Vec<Vec<Statement>> = vec![];
        for statement in &mut self.else_body {
            ctx.pre_statements = vec![];
            ctx.post_statements = vec![];
            statement.visit(v, ctx);
            statements.push(ctx.pre_statements.clone());
            statements.push(ctx.post_statements.clone());
        }

        let body = self.else_body.clone();
        let mut counter = 1;
        for statement in body {
            statements.insert(counter, vec![statement]);
            counter = counter + 3;
        }

        let statements: Vec<Statement> = statements.into_iter().flatten().collect();

        self.else_body = statements;

        if self.else_body_scope_context.is_none() {
            self.else_body_scope_context = ctx.scope_context.clone();
        } else if ctx.block_context.is_some() {
            let block = ctx.block_context.clone();
            let block = block.unwrap();
            self.else_body_scope_context = Option::from(block.scope_context.clone());
        }

        ctx.scope_context = scope;
        ctx.block_context = block;
        ctx.pre_statements = pre_statements;
        ctx.post_statements = post_statements;

        let result = v.finish_if_statement(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ForStatement {
    pub variable: VariableDeclaration,
    pub iterable: Expression,
    pub body: Vec<Statement>,
    pub for_body_scope_context: Option<ScopeContext>,
}

impl Visitable for ForStatement {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_for_statement(self, ctx);

        self.variable.visit(v, ctx);

        self.iterable.visit(v, ctx);

        let original_scope_context = ctx.scope_context.clone();
        let original_block_context = ctx.block_context.clone();
        let original_pre_statements = ctx.pre_statements.clone();
        let original_post_statements = ctx.post_statements.clone();

        let blocks_scope = if self.for_body_scope_context.is_some() {
            let temp = self.for_body_scope_context.clone();
            temp.unwrap()
        } else {
            let temp = ctx.scope_context.clone();
            temp.unwrap()
        };
        let block_context = BlockContext {
            scope_context: blocks_scope,
        };
        ctx.block_context = Some(block_context);

        let mut statements: Vec<Vec<Statement>> = vec![];
        for statement in &mut self.body {
            ctx.pre_statements = vec![];
            ctx.post_statements = vec![];
            statement.visit(v, ctx);
            statements.push(ctx.pre_statements.clone());
            statements.push(ctx.post_statements.clone());
        }

        let body = self.body.clone();
        let mut counter = 1;
        for statement in body {
            statements.insert(counter, vec![statement]);
            counter = counter + 3;
        }

        let statements: Vec<Statement> = statements.into_iter().flatten().collect();

        self.body = statements;

        ctx.scope_context = original_scope_context;
        ctx.block_context = original_block_context;
        ctx.pre_statements = original_pre_statements;
        ctx.post_statements = original_post_statements;

        let result = v.finish_for_statement(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmitStatement {
    pub function_call: FunctionCall,
}

impl Visitable for EmitStatement {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_emit_statement(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        ctx.in_emit = true;
        let result = self.function_call.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        ctx.in_emit = false;

        let result = v.finish_emit_statement(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BecomeStatement {
    pub expression: Expression,
    pub line_info: LineInfo,
}

impl Visitable for BecomeStatement {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        ctx.in_become = true;
        self.expression.visit(v, ctx);
        ctx.in_become = false;
        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ReturnStatement {
    pub expression: Option<Expression>,
    pub cleanup: Vec<Statement>,
    pub line_info: LineInfo,
}

impl Visitable for ReturnStatement {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_return_statement(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        if self.expression.is_some() {
            let expression = self.expression.clone();
            let mut expression = expression.unwrap();
            let result = expression.visit(v, ctx);
            match result {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
            self.expression = Option::from(expression);
        }

        let result = v.finish_return_statement(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    Identifier(Identifier),
    BinaryExpression(BinaryExpression),
    InoutExpression(InoutExpression),
    ExternalCall(ExternalCall),
    FunctionCall(FunctionCall),
    VariableDeclaration(VariableDeclaration),
    BracketedExpression(BracketedExpression),
    AttemptExpression(AttemptExpression),
    Literal(Literal),
    ArrayLiteral(ArrayLiteral),
    DictionaryLiteral(DictionaryLiteral),
    SelfExpression,
    SubscriptExpression(SubscriptExpression),
    RangeExpression(RangeExpression),
    RawAssembly(String, Option<Type>),
    CastExpression(CastExpression),
    Sequence(Vec<Expression>),
}

impl Expression {
    pub fn assign_enclosing_type(&mut self, t: &TypeIdentifier) {
        match self {
            Expression::Identifier(i) => {
                i.enclosing_type = Some(String::from(t));
            }
            Expression::BinaryExpression(b) => {
                b.lhs_expression.assign_enclosing_type(t);
            }
            Expression::ExternalCall(e) => {
                e.function_call.lhs_expression.assign_enclosing_type(t);
            }
            Expression::FunctionCall(f) => {
                f.identifier.enclosing_type = Some(String::from(t));
            }
            Expression::BracketedExpression(b) => {
                b.expression.assign_enclosing_type(t);
            }
            Expression::SubscriptExpression(s) => {
                s.base_expression.enclosing_type = Some(String::from(t));
            }
            _ => {}
        }
    }

    pub fn enclosing_type(&self) -> Option<String> {
        match self.clone() {
            Expression::Identifier(i) => i.enclosing_type,
            Expression::InoutExpression(i) => i.expression.enclosing_type(),
            Expression::BinaryExpression(b) => b.lhs_expression.enclosing_type(),
            Expression::VariableDeclaration(v) => Option::from(v.identifier.token),
            Expression::BracketedExpression(b) => b.expression.enclosing_type(),
            Expression::FunctionCall(f) => f.identifier.enclosing_type,
            Expression::ExternalCall(e) => e.function_call.lhs_expression.enclosing_type(),
            Expression::SubscriptExpression(_) => unimplemented!(),
            _ => None,
        }
    }

    pub fn enclosing_identifier(&self) -> Option<Identifier> {
        match self.clone() {
            Expression::Identifier(i) => Some(i),
            Expression::BinaryExpression(b) => b.lhs_expression.enclosing_identifier(),
            Expression::InoutExpression(i) => i.expression.enclosing_identifier(),
            Expression::ExternalCall(e) => e.function_call.lhs_expression.enclosing_identifier(),
            Expression::FunctionCall(f) => Some(f.identifier),
            Expression::VariableDeclaration(v) => Some(v.identifier),
            Expression::BracketedExpression(b) => b.expression.enclosing_identifier(),
            Expression::SubscriptExpression(s) => Some(s.base_expression),
            _ => None,
        }
    }

    pub fn get_line_info(&self) -> LineInfo {
        match self {
            Expression::Identifier(i) => i.line_info.clone(),
            Expression::BinaryExpression(b) => b.line_info.clone(),
            Expression::InoutExpression(i) => i.expression.get_line_info(),
            Expression::ExternalCall(_) => unimplemented!(),
            Expression::FunctionCall(_) => unimplemented!(),
            Expression::VariableDeclaration(_) => unimplemented!(),
            Expression::BracketedExpression(_) => unimplemented!(),
            Expression::AttemptExpression(_) => unimplemented!(),
            Expression::Literal(_) => unimplemented!(),
            Expression::ArrayLiteral(_) => unimplemented!(),
            Expression::DictionaryLiteral(_) => unimplemented!(),
            Expression::SelfExpression => unimplemented!(),
            Expression::SubscriptExpression(_) => unimplemented!(),
            Expression::RangeExpression(_) => unimplemented!(),
            Expression::RawAssembly(_, _) => unimplemented!(),
            Expression::CastExpression(_) => unimplemented!(),
            Expression::Sequence(_) => unimplemented!(),
        }
    }
}

impl Visitable for Expression {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_expression(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = match self {
            Expression::Identifier(i) => i.visit(v, ctx),
            Expression::BinaryExpression(b) => b.visit(v, ctx),
            Expression::InoutExpression(i) => i.visit(v, ctx),
            Expression::ExternalCall(e) => e.visit(v, ctx),
            Expression::FunctionCall(f) => f.visit(v, ctx),
            Expression::VariableDeclaration(d) => d.visit(v, ctx),
            Expression::BracketedExpression(b) => b.visit(v, ctx),
            Expression::AttemptExpression(a) => a.visit(v, ctx),
            Expression::Literal(l) => l.visit(v, ctx),
            Expression::ArrayLiteral(a) => a.visit(v, ctx),
            Expression::DictionaryLiteral(d) => d.visit(v, ctx),
            Expression::SelfExpression => return Ok(()),
            Expression::SubscriptExpression(s) => s.visit(v, ctx),
            Expression::RangeExpression(r) => r.visit(v, ctx),
            Expression::RawAssembly(_, _) => return Ok(()),
            Expression::CastExpression(c) => c.visit(v, ctx),
            Expression::Sequence(l) => {
                for i in l {
                    i.visit(v, ctx);
                }
                Ok(())
            }
        };

        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = v.finish_expression(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CastExpression {
    pub expression: Box<Expression>,
    pub cast_type: Type,
}

impl Visitable for CastExpression {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_cast_expression(self, ctx);

        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = self.cast_type.visit(v, ctx);

        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = self.expression.visit(v, ctx);

        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = v.finish_cast_expression(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RangeExpression {
    pub start_expression: Box<Expression>,
    pub end_expression: Box<Expression>,
    pub op: std::string::String,
}

impl Visitable for RangeExpression {
    fn visit(&mut self, _v: &mut dyn Visitor, _ctx: &mut Context) -> VResult {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubscriptExpression {
    pub base_expression: Identifier,
    pub index_expression: Box<Expression>,
}

impl Visitable for SubscriptExpression {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_subscript_expression(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let in_subscript = ctx.in_subscript;

        let result = self.base_expression.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        ctx.in_subscript = true;

        let result = self.index_expression.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        ctx.in_subscript = in_subscript;

        let result = v.finish_subscript_expression(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DictionaryLiteral {
    pub elements: Vec<(Expression, Expression)>,
}

impl Visitable for DictionaryLiteral {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_dictionary_literal(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        for (e, l) in &mut self.elements {
            let result = e.visit(v, ctx);
            match result {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
            let result = l.visit(v, ctx);
            match result {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }
        let result = v.finish_dictionary_literal(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayLiteral {
    pub elements: Vec<Expression>,
}

impl Visitable for ArrayLiteral {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_array_literal(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = self.elements.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = v.finish_array_literal(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Literal {
    BooleanLiteral(bool),
    AddressLiteral(String),
    StringLiteral(String),
    IntLiteral(u64),
    FloatLiteral(f64),
}

impl Visitable for Literal {
    fn visit(&mut self, _v: &mut dyn Visitor, _ctx: &mut Context) -> VResult {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttemptExpression {
    pub kind: String,
    pub function_call: FunctionCall,
}

impl AttemptExpression {
    pub fn is_soft(&self) -> bool {
        self.kind.eq("?")
    }
}

impl Visitable for AttemptExpression {
    fn visit(&mut self, _v: &mut dyn Visitor, _ctx: &mut Context) -> VResult {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExternalCall {
    pub arguments: Vec<FunctionArgument>,
    pub function_call: BinaryExpression,
    pub external_trait_name: Option<String>,
}

impl Visitable for ExternalCall {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_external_call(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let old_is_external_call = ctx.is_external_function_call;
        let old_external_call_context = ctx.external_call_context.clone();

        ctx.is_external_function_call = true;
        ctx.external_call_context = Option::from(self.clone());

        let result = self.function_call.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        ctx.is_external_function_call = old_is_external_call;
        ctx.external_call_context = old_external_call_context;

        let result = v.finish_external_call(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Default, Debug)]
pub struct Identifier {
    pub token: std::string::String,
    pub enclosing_type: Option<std::string::String>,
    pub line_info: LineInfo,
}

impl Identifier {
    pub fn is_self(&self) -> bool {
        self.token == "self"
    }
}

impl PartialEq for Identifier {
    fn eq(&self, other: &Self) -> bool {
        self.token == other.token && self.enclosing_type == other.enclosing_type
    }
}

impl Visitable for Identifier {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_identifier(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = v.finish_identifier(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BinOp {
    Plus,
    OverflowingPlus,
    Minus,
    OverflowingMinus,
    Times,
    OverflowingTimes,
    Power,
    Divide,
    Percent,
    Dot,
    Equal,
    PlusEqual,
    MinusEqual,
    TimesEqual,
    DivideEqual,
    DoubleEqual,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Or,
    And,
    Implies,
}

impl BinOp {
    pub fn is_left(&self) -> bool {
        match self {
            BinOp::Plus => true,
            BinOp::Times => true,
            BinOp::Dot => true,
            _ => false,
        }
    }
    pub fn is_boolean(&self) -> bool {
        match self {
            BinOp::DoubleEqual => true,
            BinOp::NotEqual => true,
            BinOp::LessThan => true,
            BinOp::LessThanOrEqual => true,
            BinOp::GreaterThan => true,
            BinOp::GreaterThanOrEqual => true,
            BinOp::Or => true,
            BinOp::And => true,
            BinOp::Implies => true,
            _ => false,
        }
    }

    pub fn is_assignment(&self) -> bool {
        match self {
            BinOp::Equal => true,
            BinOp::PlusEqual => true,
            BinOp::MinusEqual => true,
            BinOp::TimesEqual => true,
            BinOp::DivideEqual => true,
            _ => false,
        }
    }

    pub fn is_assignment_shorthand(&self) -> bool {
        match self {
            BinOp::PlusEqual => true,
            BinOp::MinusEqual => true,
            BinOp::TimesEqual => true,
            BinOp::DivideEqual => true,
            _ => false,
        }
    }

    pub fn get_assignment_shorthand(&self) -> BinOp {
        match self {
            BinOp::PlusEqual => BinOp::Plus,
            BinOp::MinusEqual => BinOp::Minus,
            BinOp::TimesEqual => BinOp::Times,
            BinOp::DivideEqual => BinOp::Divide,
            _ => unimplemented!(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BinaryExpression {
    pub lhs_expression: Box<Expression>,
    pub rhs_expression: Box<Expression>,
    pub op: BinOp,
    pub line_info: LineInfo,
}

impl Visitable for BinaryExpression {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_binary_expression(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        if self.op.is_assignment() {
            if let Expression::VariableDeclaration(_) = *self.lhs_expression {
            } else {
                ctx.is_lvalue = true;
            }
        }

        if let BinOp::Dot = self.op {
            ctx.is_enclosing = true;
        }

        let old_context = ctx.external_call_context.clone();
        ctx.external_call_context = None;

        let result = self.lhs_expression.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        if let BinOp::Dot = self.op.clone() {
            ctx.is_lvalue = false;
        }

        ctx.external_call_context = old_context;
        ctx.is_enclosing = false;

        let scope = ctx.scope_context.clone();
        let scope = scope.unwrap_or_default();

        let enclosing = ctx.enclosing_type_identifier();
        let enclosing = enclosing.unwrap_or_default();
        let enclosing = enclosing.token;
        let lhs_type = ctx.environment.get_expression_type(
            *self.lhs_expression.clone(),
            &enclosing,
            vec![],
            vec![],
            scope,
        );

        match lhs_type {
            Type::DictionaryType(_) => {}
            Type::ArrayType(_) => {}
            Type::FixedSizedArrayType(_) => {}
            _ => {
                if self.op.is_assignment() {
                    ctx.in_assignment = true;
                }
                let result = self.rhs_expression.visit(v, ctx);
                match result {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
                ctx.in_assignment = false;
            }
        };

        let result = v.finish_binary_expression(self, ctx);

        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        ctx.is_lvalue = false;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InoutExpression {
    pub ampersand_token: std::string::String,
    pub expression: Box<Expression>,
}

impl Visitable for InoutExpression {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_inout_expression(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = self.expression.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = v.finish_inout_expression(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionCall {
    pub identifier: Identifier,
    pub arguments: Vec<FunctionArgument>,
    pub mangled_identifier: Option<Identifier>,
}

impl Visitable for FunctionCall {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_function_call(self, ctx);

        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        ctx.is_function_call_context = true;
        let result = self.identifier.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        ctx.is_function_call_context = false;

        let old_context = ctx.external_call_context.clone();
        ctx.external_call_context = None;

        let result = self.arguments.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        ctx.external_call_context = old_context;

        let result = v.finish_function_call(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        ctx.external_call_context = None;

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BracketedExpression {
    pub expression: Box<Expression>,
}

impl Visitable for BracketedExpression {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = self.expression.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionArgument {
    pub identifier: Option<Identifier>,
    pub expression: Expression,
}

impl Visitable for FunctionArgument {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_function_argument(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        ctx.is_function_call_argument_label = true;
        if self.identifier.is_some() {
            let ident = self.identifier.clone();
            let mut ident = ident.unwrap();

            ident.visit(v, ctx);
            self.identifier = Option::from(ident);
        }
        ctx.is_function_call_argument_label = false;

        ctx.is_function_call_argument = true;
        let result = self.expression.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        ctx.is_function_call_argument = false;

        let result = v.finish_function_argument(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        Ok(())
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct EventDeclaration {
    pub identifier: Identifier,
    pub parameter_list: Vec<Parameter>,
}

impl Visitable for EventDeclaration {
    fn visit(&mut self, _v: &mut dyn Visitor, _ctx: &mut Context) -> VResult {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Parameter {
    pub identifier: Identifier,
    pub type_assignment: Type,
    pub expression: Option<Expression>,
    pub line_info: LineInfo,
}

impl Parameter {
    pub fn is_payable(&self) -> bool {
        self.type_assignment.is_currency_type()
    }

    pub fn is_dynamic(&self) -> bool {
        self.type_assignment.is_dynamic_type()
    }
    pub fn as_variable_declaration(&self) -> VariableDeclaration {
        VariableDeclaration {
            declaration_token: None,
            identifier: self.identifier.clone(),
            variable_type: self.type_assignment.clone(),
            expression: None,
        }
    }

    pub fn is_inout(&self) -> bool {
        if self.type_assignment.is_inout_type() {
            return true;
        }
        false
    }
}

impl Visitable for Parameter {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_parameter(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = self.type_assignment.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = v.finish_parameter(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VariableDeclaration {
    pub declaration_token: Option<String>,
    pub identifier: Identifier,
    pub variable_type: Type,
    pub expression: Option<Box<Expression>>,
}

impl VariableDeclaration {
    pub fn is_constant(&self) -> bool {
        if self.declaration_token.is_some() {
            return self.declaration_token.as_ref().unwrap() == "let";
        }
        false
    }

    pub fn is_variable(&self) -> bool {
        if self.declaration_token.is_some() {
            return self.declaration_token.as_ref().unwrap() == "var";
        }
        false
    }
}

impl Visitable for VariableDeclaration {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_variable_declaration(self, ctx);

        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = self.identifier.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        let result = self.variable_type.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        if self.expression.is_some() {
            let previous_scope = ctx.scope_context.clone();
            ctx.scope_context = Option::from(ScopeContext {
                parameters: vec![],
                local_variables: vec![],
                counter: 0,
            });

            ctx.is_property_default_assignment = true;
            let expression = self.expression.clone();
            let mut expression = expression.unwrap();

            expression.visit(v, ctx);

            self.expression = Option::from(expression);
            ctx.is_property_default_assignment = false;

            ctx.scope_context = previous_scope;
        }

        let result = v.finish_variable_declaration(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    QuartzType(QuartzType),
    InoutType(InoutType),
    ArrayType(ArrayType),
    RangeType(RangeType),
    FixedSizedArrayType(FixedSizedArrayType),
    DictionaryType(DictionaryType),
    UserDefinedType(Identifier),
    Solidity(SolidityType),
    SelfType,
    Bool,
    Int,
    String,
    Address,
    Error,
}

impl Type {
    pub fn type_from_identifier(identifier: Identifier) -> Type {
        Type::UserDefinedType(identifier)
    }

    pub fn name_is_basic_type(name: &str) -> bool {
        match name {
            "Bool" => true,
            "Address" => true,
            "Int" => true,
            "String" => true,
            _ => false,
        }
    }

    pub fn is_dictionary_type(&self) -> bool {
        match self {
            Type::DictionaryType(_) => true,
            _ => false,
        }
    }

    pub fn is_currency_type(&self) -> bool {
        let currency_type = self.clone();

        let identifier = match currency_type {
            Type::UserDefinedType(i) => i,
            _ => return false,
        };

        let identifier = identifier.token.clone();
        println!("Is resource type");
        println!("{:?}", identifier.clone());
        identifier.eq("Wei") || identifier.eq("Libra") || identifier.eq("LibraCoin.T")
    }

    pub fn is_currency_original_type(&self) -> bool {
        let identifier = match self.clone() {
            Type::UserDefinedType(i) => i,
            _ => return false,
        };

        let identifier = identifier.token.clone();
        identifier.eq("Wei") || identifier.eq("Libra")
    }

    pub fn is_dynamic_type(&self) -> bool {
        match self {
            Type::Int => false,
            Type::Address => false,
            Type::Bool => false,
            Type::String => false,
            _ => true,
        }
    }

    pub fn is_address_type(&self) -> bool {
        match self {
            Type::Address => true,
            _ => false,
        }
    }

    pub fn is_bool_type(&self) -> bool {
        match self {
            Type::Bool => true,
            _ => false,
        }
    }

    pub fn is_inout_type(&self) -> bool {
        match self {
            Type::InoutType(_) => true,
            _ => false,
        }
    }

    pub fn is_user_defined_type(&self) -> bool {
        match self {
            Type::UserDefinedType(_) => true,
            _ => false,
        }
    }

    pub fn is_built_in_type(&self) -> bool {
        match self {
            Type::QuartzType(_) => unimplemented!(),
            Type::InoutType(i) => i.key_type.is_built_in_type(),
            Type::ArrayType(a) => a.key_type.is_built_in_type(),
            Type::RangeType(r) => r.key_type.is_built_in_type(),
            Type::FixedSizedArrayType(a) => a.key_type.is_built_in_type(),
            Type::DictionaryType(_) => unimplemented!(),
            Type::UserDefinedType(_) => false,
            Type::Bool => true,
            Type::Int => true,
            Type::String => true,
            Type::Address => true,
            Type::Error => true,
            Type::SelfType => unimplemented!(),
            Type::Solidity(_) => unimplemented!(),
        }
    }

    pub fn name(&self) -> String {
        match self {
            Type::QuartzType(_) => unimplemented!(),
            Type::InoutType(i) => {
                let name = i.key_type.name();
                format!("$inout{name}", name = name)
            }
            Type::ArrayType(_) => unimplemented!(),
            Type::RangeType(_) => unimplemented!(),
            Type::FixedSizedArrayType(_) => unimplemented!(),
            Type::DictionaryType(_) => unimplemented!(),
            Type::UserDefinedType(i) => i.token.clone(),
            Type::Bool => "Bool".to_string(),
            Type::Int => "Int".to_string(),
            Type::String => "String".to_string(),
            Type::Address => "Address".to_string(),
            Type::Error => "Quartz$ErrorType".to_string(),
            Type::SelfType => "Self".to_string(),
            Type::Solidity(s) => format!("{:?}", s),
        }
    }

    pub fn replacing_self(&self, t: &TypeIdentifier) -> Type {
        let input_type = self.clone();

        if Type::SelfType == input_type {
            return Type::UserDefinedType(Identifier {
                token: t.to_string(),
                enclosing_type: None,
                line_info: Default::default(),
            });
        }

        if let Type::InoutType(i) = input_type.clone() {
            if let Type::SelfType = *i.key_type {
                return Type::InoutType(InoutType {
                    key_type: Box::new(Type::UserDefinedType(Identifier {
                        token: t.to_string(),
                        enclosing_type: None,
                        line_info: Default::default(),
                    })),
                });
            }
        }

        println!("FLOPPED IT");

        input_type
    }

    pub fn is_external_contract(&self, environment: Environment) -> bool {
        let mut internal_type = self.clone();

        if let Type::InoutType(i) = internal_type {
            internal_type = *i.key_type;
        }

        if let Type::UserDefinedType(u) = internal_type {
            let type_identifer = u.token.clone();
            if environment.is_trait_declared(&type_identifer) {
                let type_infos = environment.types.get(&type_identifer);
                if type_infos.is_some() {
                    let type_infos = type_infos.unwrap();
                    let type_infos = type_infos.clone();
                    if !type_infos.is_external_struct() {
                        return true;
                    }
                }
            } else {
                return false;
            }
        }

        false
    }

    pub fn is_external_resource(&self, environment: Environment) -> bool {
        let mut internal_type = self.clone();

        if let Type::InoutType(i) = internal_type {
            internal_type = *i.key_type;
        }

        if let Type::UserDefinedType(u) = internal_type {
            let type_identifer = u.token.clone();
            if environment.is_trait_declared(&type_identifer) {
                let type_infos = environment.types.get(&type_identifer);
                if type_infos.is_some() {
                    let type_infos = type_infos.unwrap();
                    let type_infos = type_infos.clone();
                    if type_infos.is_external_resource() {
                        return true;
                    }
                }
            } else {
                return false;
            }
        }
        false
    }

    pub fn is_external_module(&self, environment: Environment) -> bool {
        let mut internal_type = self.clone();

        if let Type::InoutType(i) = internal_type {
            internal_type = *i.key_type;
        }

        if let Type::UserDefinedType(u) = internal_type {
            let type_identifer = u.token.clone();
            if environment.is_trait_declared(&type_identifer) {
                let type_infos = environment.types.get(&type_identifer);
                if type_infos.is_some() {
                    let type_infos = type_infos.unwrap();
                    let type_infos = type_infos.clone();
                    if type_infos.is_external_module() {
                        return true;
                    }
                }
            } else {
                return false;
            }
        }
        false
    }

    pub fn from_identifier(identifier: Identifier) -> Type {
        let name = identifier.token.clone();
        if name == "Address" {
            return Type::Address;
        }
        if name == "Bool" {
            return Type::Bool;
        }
        if name == "Int" {
            return Type::Int;
        }
        if name == "String" {
            return Type::String;
        }

        Type::UserDefinedType(identifier)
    }
}

impl Visitable for Type {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_type(self, ctx);

        v.finish_type(self, ctx);

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SolidityType {
    ADDRESS,
    STRING,
    BOOL,
    INT8,
    INT16,
    INT24,
    INT32,
    INT40,
    INT48,
    INT56,
    INT64,
    INT72,
    INT80,
    INT88,
    INT96,
    INT104,
    INT112,
    INT120,
    INT128,
    INT136,
    INT144,
    INT152,
    INT160,
    INT168,
    INT176,
    INT184,
    INT192,
    INT200,
    INT208,
    INT216,
    INT224,
    INT232,
    INT240,
    INT248,
    INT256,
    UINT8,
    UINT16,
    UINT24,
    UINT32,
    UINT40,
    UINT48,
    UINT56,
    UINT64,
    UINT72,
    UINT80,
    UINT88,
    UINT96,
    UINT104,
    UINT112,
    UINT120,
    UINT128,
    UINT136,
    UINT144,
    UINT152,
    UINT160,
    UINT168,
    UINT176,
    UINT184,
    UINT192,
    UINT200,
    UINT208,
    UINT216,
    UINT224,
    UINT232,
    UINT240,
    UINT248,
    UINT256,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DictionaryType {
    pub key_type: Box<Type>,
    pub value_type: Box<Type>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RangeType {
    pub key_type: Box<Type>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayType {
    pub key_type: Box<Type>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FixedSizedArrayType {
    pub key_type: Box<Type>,
    pub size: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InoutType {
    pub key_type: Box<Type>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuartzType {
    pub base_type: Box<Type>,
    pub arguments: Vec<Type>,
}

#[derive(Debug)]
pub struct TypeAnnotation {
    pub colon: std::string::String,
    pub type_assigned: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub at_token: std::string::String,
    pub identifier_token: std::string::String,
}

pub fn is_redeclaration(identifier1: &Identifier, identifier2: &Identifier) -> bool {
    if identifier1.token == identifier2.token && identifier1.line_info != identifier2.line_info {
        return true;
    }
    false
}

pub fn is_return_or_become_statement(statement: Statement) -> bool {
    match statement {
        Statement::ReturnStatement(_) => true,
        Statement::BecomeStatement(_) => false,
        _ => false,
    }
}

pub fn is_literal(expression: &Expression) -> bool {
    match expression {
        Expression::Literal(_) => true,
        _ => false,
    }
}

pub fn do_vecs_match<T: PartialEq>(a: &Vec<T>, b: &Vec<T>) -> bool {
    let matching = a.iter().zip(b.iter()).filter(|&(a, b)| a == b).count();
    matching == a.len() && matching == b.len()
}

pub fn mangle(string: String) -> String {
    format!("_{}", string)
}

pub fn mangle_dictionary(string: String) -> String {
    format!("_dictionary_{}", string)
}

pub fn mangle_function(string: String, t: &TypeIdentifier, is_contract: bool) -> String {
    let func_type = if is_contract {
        "".to_string()
    } else {
        format!("{}$", t)
    };
    format!("{func_type}{name}", name = string, func_type = func_type)
}

pub fn mangle_function_move(string: String, t: &TypeIdentifier, is_contract: bool) -> String {
    let func_type = if is_contract {
        "".to_string()
    } else {
        format!("{}_", t)
    };
    format!("{func_type}{name}", name = string, func_type = func_type)
}

pub fn mangle_mem(string: String) -> String {
    format!("{}$isMem", string)
}

pub struct CodeGen {
    pub code: String,
    pub indent_level: i32,
    pub indent_size: i32,
}

impl CodeGen {
    pub fn add<S>(&mut self, code: S)
    where
        S: AsRef<str>,
    {
        for line in code.as_ref().lines() {
            let line = line.trim();
            let indent_change =
                (line.matches("{").count() as i32) - (line.matches("}").count() as i32);
            let new_indent_level = max(0, self.indent_level + indent_change);

            let this_line_indent = if line.starts_with("}") || line.ends_with(":") {
                self.indent_level - 1
            } else {
                self.indent_level
            };

            for _ in 0..this_line_indent * self.indent_size {
                self.code.push(' ');
            }
            self.code.push_str(line);
            self.code.push_str("\n");

            self.indent_level = new_indent_level;
        }
    }
}