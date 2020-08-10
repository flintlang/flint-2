pub mod calls;
#[allow(clippy::large_enum_variant)]
pub mod declarations;
pub mod expressions;
pub mod literals;
pub mod operators;
pub mod statements;
pub mod types;

use std::cmp::max;
use std::collections::HashMap;
use std::error::Error;
use std::string::String;
use std::vec::Vec;

use sha3::{Digest, Keccak256};

use super::context::*;
use super::visitor::*;

pub use crate::ast::calls::*;
pub use crate::ast::declarations::*;
pub use crate::ast::expressions::*;
pub use crate::ast::literals::*;
pub use crate::ast::operators::*;
pub use crate::ast::statements::*;
pub use crate::ast::types::*;

pub type VResult = Result<(), Box<dyn Error>>;

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
    pub type_states: Vec<TypeState>,
    pub current_state: Option<TypeState>,
    pub modifiers: Vec<FunctionCall>,
}

impl TypeInfo {
    pub fn new() -> TypeInfo {
        TypeInfo {
            ordered_properties: vec![],
            properties: Default::default(),
            functions: Default::default(),
            initialisers: vec![],
            fallbacks: vec![],
            public_initializer: None,
            conformances: vec![],
            type_states: vec![],
            current_state: None,
            modifiers: vec![],
        }
    }

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
            .filter(|f| f.identifier.token == "module")
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
            .filter(|f| f.identifier.token == "resource")
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
            .filter(|f| f.identifier.token == "resource" || f.identifier.token == "struct")
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
            Property::VariableDeclaration(v, _) => &v.variable_type,
            Property::EnumCase(e) => &e.enum_type,
        }
    }

    pub fn is_constant(&self) -> bool {
        match &self.property {
            Property::EnumCase(_) => true,
            Property::VariableDeclaration(v, _) => v.is_constant(),
        }
    }

    // Perhaps this should be removed
    pub fn get_modifier(&self) -> &Option<Modifier> {
        self.property.get_modifier()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Property {
    VariableDeclaration(VariableDeclaration, Option<Modifier>),
    EnumCase(EnumMember),
}

impl Property {
    pub fn get_identifier(&self) -> Identifier {
        match self {
            Property::VariableDeclaration(v, _) => v.identifier.clone(),
            Property::EnumCase(e) => e.identifier.clone(),
        }
    }

    pub fn get_type(&self) -> Type {
        match self {
            Property::VariableDeclaration(v, _) => v.variable_type.clone(),
            Property::EnumCase(e) => e.enum_type.clone(),
        }
    }

    pub fn get_value(&self) -> Option<Expression> {
        match self {
            Property::VariableDeclaration(v, _) => {
                let expression = v.expression.clone();
                match expression {
                    None => None,
                    Some(e) => Some(*e),
                }
            }
            Property::EnumCase(e) => e.hidden_value.clone(),
        }
    }

    pub fn get_modifier(&self) -> &Option<Modifier> {
        match self {
            Property::VariableDeclaration(_, modifier) => modifier,
            _ => &None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpecialInformation {
    pub declaration: SpecialDeclaration,
    pub type_states: Vec<TypeState>,
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
    pub caller_protections: Vec<CallerProtection>,
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
            .map(|p| p.identifier)
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
        v.start_module(self, ctx)?;
        self.declarations.visit(v, ctx)?;
        v.finish_module(self, ctx)?;
        Ok(())
    }
}

impl<T: Visitable> Visitable for Vec<T> {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        for t in self {
            t.visit(v, ctx)?;
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

    pub fn is_sub_protection(&self, parent: &CallerProtection) -> bool {
        parent.is_any() || self.name() == parent.name()
    }
}

impl Visitable for CallerProtection {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_caller_protection(self, ctx)?;
        self.identifier.visit(v, ctx)?;
        v.finish_caller_protection(self, ctx)?;
        Ok(())
    }
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

pub fn is_return_or_become_statement(statement: &Statement) -> bool {
    match statement {
        Statement::ReturnStatement(_) | Statement::BecomeStatement(_) => true,
        _ => false,
    }
}

pub fn is_literal(expression: &Expression) -> bool {
    match expression {
        Expression::Literal(_) => true,
        _ => false,
    }
}

pub fn mangle(string: &str) -> String {
    format!("_{}", string)
}

pub fn mangle_dictionary(string: &str) -> String {
    format!("_dictionary_{}", string)
}

#[allow(dead_code)]
pub fn mangle_function(string: &str, type_id: &str, is_contract: bool) -> String {
    let func_type = if is_contract {
        "".to_string()
    } else {
        format!("{}$", type_id)
    };
    format!("{func_type}{name}", name = string, func_type = func_type)
}

pub fn mangle_function_move(string: &str, type_id: &str, is_contract: bool) -> String {
    let func_type = if is_contract {
        "".to_string()
    } else {
        format!("{}_", type_id)
    };
    format!("{func_type}{name}", name = string, func_type = func_type)
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
                (line.matches('{').count() as i32) - (line.matches('}').count() as i32);
            let new_indent_level = max(0, self.indent_level + indent_change);

            let this_line_indent = if line.starts_with('}') || line.ends_with(':') {
                self.indent_level - 1
            } else {
                self.indent_level
            };

            for _ in 0..this_line_indent * self.indent_size {
                self.code.push(' ');
            }
            self.code.push_str(line);
            self.code.push('\n');

            self.indent_level = new_indent_level;
        }
    }
}
