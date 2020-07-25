use crate::ast::*;
use crate::environment::*;

#[derive(Debug, Default)]
pub struct Context {
    pub environment: Environment,
    pub contract_declaration_context: Option<ContractDeclarationContext>,
    pub contract_behaviour_declaration_context: Option<ContractBehaviourDeclarationContext>,
    pub struct_declaration_context: Option<StructDeclarationContext>,
    pub function_declaration_context: Option<FunctionDeclarationContext>,
    pub special_declaration_context: Option<SpecialDeclarationContext>,
    pub trait_declaration_context: Option<TraitDeclarationContext>,
    pub scope_context: Option<ScopeContext>,
    pub asset_context: Option<AssetDeclarationContext>,
    pub block_context: Option<BlockContext>,
    pub function_call_receiver_trail: Vec<Expression>,
    pub is_property_default_assignment: bool,
    pub is_function_call_context: bool,
    pub is_function_call_argument: bool,
    pub is_function_call_argument_label: bool,
    pub external_call_context: Option<ExternalCall>,
    pub is_external_function_call: bool,
    pub in_assignment: bool,
    pub in_if_condition: bool,
    pub in_become: bool,
    pub is_lvalue: bool,
    pub in_subscript: bool,
    pub is_enclosing: bool,
    pub in_emit: bool,
    pub pre_statements: Vec<Statement>,
    pub post_statements: Vec<Statement>,
}

impl Context {
    pub fn enclosing_type_identifier(&self) -> Option<&Identifier> {
        self.contract_behaviour_declaration_context
            .as_ref()
            .map(|c| &c.identifier)
            .or_else(|| {
                self.struct_declaration_context
                    .as_ref()
                    .map(|c| &c.identifier)
                    .or_else(|| {
                        self.contract_declaration_context
                            .as_ref()
                            .map(|c| &c.identifier)
                            .or_else(|| self.asset_context.as_ref().map(|c| &c.identifier))
                    })
            })
    }

    pub fn is_trait_declaration_context(&self) -> bool {
        self.trait_declaration_context.is_some()
    }

    pub(crate) fn in_function_or_special(&self) -> bool {
        self.function_declaration_context.is_some() || self.special_declaration_context.is_some()
    }

    pub fn scope_context(&self) -> Option<&ScopeContext> {
        self.scope_context.as_ref()
    }
}

#[derive(Debug)]
pub struct ContractDeclarationContext {
    pub identifier: Identifier,
}

#[derive(Debug, Clone)]
pub struct ContractBehaviourDeclarationContext {
    pub identifier: Identifier,
    pub caller: Option<Identifier>,
    pub type_states: Vec<TypeState>,
    pub caller_protections: Vec<CallerProtection>,
}

#[derive(Debug, Clone)]
pub struct StructDeclarationContext {
    pub identifier: Identifier,
}

#[derive(Debug, Default, Clone)]
pub struct FunctionDeclarationContext {
    pub declaration: FunctionDeclaration,
    pub local_variables: Vec<VariableDeclaration>,
}

impl FunctionDeclarationContext {
    pub fn mutates(&self) -> Vec<Identifier> {
        self.declaration.mutates()
    }
}

#[derive(Debug, Clone)]
pub struct SpecialDeclarationContext {
    pub declaration: SpecialDeclaration,
    pub local_variables: Vec<VariableDeclaration>,
}

#[derive(Debug, Clone)]
pub struct TraitDeclarationContext {
    pub identifier: Identifier,
}

#[derive(Debug, Clone)]
pub struct BlockContext {
    pub scope_context: ScopeContext,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ScopeContext {
    pub parameters: Vec<Parameter>,
    pub local_variables: Vec<VariableDeclaration>,
    pub counter: u64,
}

impl ScopeContext {
    fn first_local_or_parameter<P: Fn(&VariableDeclaration) -> bool>(
        &self,
        predicate: P,
    ) -> Option<VariableDeclaration> {
        self.local_variables
            .iter()
            .find(|&p| predicate(p))
            .cloned()
            .or_else(|| {
                self.parameters
                    .iter()
                    .map(Parameter::as_variable_declaration)
                    .find(predicate)
            })
    }

    pub fn declaration(&self, name: &str) -> Option<VariableDeclaration> {
        self.first_local_or_parameter(|v: &VariableDeclaration| v.identifier.token.as_str() == name)
    }

    pub fn type_for(&self, variable: &str) -> Option<Type> {
        self.first_local_or_parameter(|v| {
            v.identifier.token == variable || mangle(variable) == v.identifier.token
        })
        .map(|i| i.variable_type)
    }

    pub fn contains_variable_declaration(&self, name: &str) -> bool {
        self.local_variables
            .iter()
            .map(|v| &*v.identifier.token)
            .any(|id| id == name)
    }

    pub fn contains_parameter_declaration(&self, name: &str) -> bool {
        self.parameters
            .iter()
            .map(|p| &*p.identifier.token)
            .any(|id| id == name)
    }

    pub fn fresh_identifier(&mut self, line_info: LineInfo) -> Identifier {
        self.counter += 1;
        let count = self.local_variables.len() + self.parameters.len() + self.counter as usize;
        let name = format!("temp__{}", count);
        Identifier {
            token: name,
            enclosing_type: None,
            line_info,
        }
    }

    pub fn enclosing_parameter(&self, expression: &Expression, type_id: &str) -> Option<String> {
        let expression_enclosing = expression.enclosing_type().unwrap_or_default();
        if expression_enclosing == type_id {
            if let Some(enclosing_identifier) = expression.enclosing_identifier() {
                if self.contains_parameter_declaration(&enclosing_identifier.token) {
                    return Some(enclosing_identifier.token.clone());
                }
            }
        }
        None
    }
}

const DEFAULT_SCOPE_CONTEXT_REF: &ScopeContext = &ScopeContext {
    parameters: vec![],
    local_variables: vec![],
    counter: 0,
};

impl Default for &ScopeContext {
    fn default() -> Self {
        DEFAULT_SCOPE_CONTEXT_REF
    }
}

#[derive(Debug, Clone)]
pub struct AssetDeclarationContext {
    pub identifier: Identifier,
}
