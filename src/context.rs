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
    pub fn enclosing_type_identifier(&self) -> Option<Identifier> {
        if self.is_contract_behaviour_declaration_context() {
            let i = self
                .contract_behaviour_declaration_context
                .as_ref()
                .unwrap()
                .identifier
                .clone();
            Some(i)
        } else if self.is_struct_declaration_context() {
            let i = self
                .struct_declaration_context
                .as_ref()
                .unwrap()
                .identifier
                .clone();
            Some(i)
        } else if self.is_contract_declaration_context() {
            let i = self
                .contract_declaration_context
                .as_ref()
                .unwrap()
                .identifier
                .clone();
            Some(i)
        } else if self.is_asset_declaration_context() {
            let i = self.asset_context.as_ref().unwrap().identifier.clone();
            Some(i)
        } else {
            None
        }
    }
    pub fn is_contract_declaration_context(&self) -> bool {
        self.contract_declaration_context.is_some()
    }

    pub fn is_contract_behaviour_declaration_context(&self) -> bool {
        self.contract_behaviour_declaration_context.is_some()
    }

    fn is_struct_declaration_context(&self) -> bool {
        self.struct_declaration_context.is_some()
    }

    fn is_asset_declaration_context(&self) -> bool {
        self.asset_context.is_some()
    }

    pub fn is_function_declaration_context(&self) -> bool {
        self.function_declaration_context.is_some()
    }

    pub fn is_special_declaration_context(&self) -> bool {
        self.special_declaration_context.is_some()
    }

    pub fn is_trait_declaration_context(&self) -> bool {
        self.trait_declaration_context.is_some()
    }

    pub(crate) fn in_function_or_special(&self) -> bool {
        self.is_function_declaration_context() || self.is_special_declaration_context()
    }

    pub(crate) fn has_scope_context(&self) -> bool {
        self.scope_context.is_some()
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
    pub fn declaration(&self, name: String) -> Option<VariableDeclaration> {
        let mut identifiers: Vec<VariableDeclaration> = self
            .local_variables
            .clone()
            .into_iter()
            .chain(
                self.parameters
                    .clone()
                    .into_iter()
                    .map(|p| p.as_variable_declaration()),
            )
            .collect();
        identifiers = identifiers
            .into_iter()
            .filter(|v| v.identifier.token == name)
            .collect();
        identifiers.first().cloned()
    }

    pub fn type_for(&self, variable: &str) -> Option<Type> {
        self.local_variables
            .clone()
            .into_iter()
            .chain(
                self.parameters
                    .clone()
                    .into_iter()
                    .map(|p| p.as_variable_declaration()),
            )
            .find(|v| v.identifier.token == variable || mangle(variable) == v.identifier.token)
            .map(|i| i.variable_type)
    }

    pub fn contains_variable_declaration(&self, name: String) -> bool {
        let variables: Vec<String> = self
            .local_variables
            .clone()
            .into_iter()
            .map(|v| v.identifier.token)
            .collect();
        variables.contains(&name)
    }

    pub fn contains_parameter_declaration(&self, name: String) -> bool {
        let parameters: Vec<String> = self
            .parameters
            .clone()
            .into_iter()
            .map(|p| p.identifier.token)
            .collect();
        parameters.contains(&name)
    }

    //TODO: here is where new local variables are created
    pub fn fresh_identifier(&mut self, line_info: LineInfo) -> Identifier {
        self.counter = self.counter + 1;
        let count = self.local_variables.len() + self.parameters.len() + self.counter as usize;
        let name = format!("temp__{}", count);
        Identifier {
            token: name,
            enclosing_type: None,
            line_info,
        }
    }

    pub fn enclosing_parameter(
        &self,
        expression: Expression,
        t: &TypeIdentifier,
    ) -> Option<String> {
        let expression_enclosing = expression.enclosing_type();
        let expression_enclosing = expression_enclosing.unwrap_or_default();
        if expression_enclosing == t.to_string() && expression.enclosing_identifier().is_some() {
            //REMOVEBEFOREFLIGHT
            let enclosing_identifier = expression.enclosing_identifier().clone();
            let enclosing_identifier = enclosing_identifier.unwrap();
            if self.contains_parameter_declaration(enclosing_identifier.token.clone()) {
                return Option::from(enclosing_identifier.token);
            }
        }

        None
    }
}

#[derive(Debug, Clone)]
pub struct AssetDeclarationContext {
    pub identifier: Identifier,
}
