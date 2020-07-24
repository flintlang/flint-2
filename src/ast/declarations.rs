use crate::ast::*;
use crate::context::*;
use crate::visitor::Visitor;
use hex::encode;
use nom::lib::std::fmt::Formatter;

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
    #[allow(dead_code)]
    pub fn is_contract_behaviour_declaration(&self) -> bool {
        match self {
            TopLevelDeclaration::ContractBehaviourDeclaration(_) => true,
            _ => false,
        }
    }
}

impl Visitable for TopLevelDeclaration {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_top_level_declaration(self, ctx)?;

        match self {
            TopLevelDeclaration::ContractDeclaration(c) => c.visit(v, ctx),
            TopLevelDeclaration::ContractBehaviourDeclaration(c) => c.visit(v, ctx),
            TopLevelDeclaration::StructDeclaration(s) => s.visit(v, ctx),
            TopLevelDeclaration::EnumDeclaration(e) => e.visit(v, ctx),
            TopLevelDeclaration::TraitDeclaration(t) => t.visit(v, ctx),
            TopLevelDeclaration::AssetDeclaration(a) => a.visit(v, ctx),
        }?;

        v.finish_top_level_declaration(self, ctx)?;
        Ok(())
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct ContractDeclaration {
    pub identifier: Identifier,
    pub contract_members: Vec<ContractMember>,
    pub type_states: Vec<TypeState>,
    pub conformances: Vec<Conformance>,
}

impl ContractDeclaration {
    #[allow(dead_code)]
    pub fn contract_enum_prefix() -> String {
        "QuartzStateEnum$".to_string()
    }

    #[allow(dead_code)]
    pub fn get_variable_declarations(&self) -> Vec<VariableDeclaration> {
        let members = self.contract_members.clone();
        members
            .into_iter()
            .filter_map(|c| match c {
                ContractMember::VariableDeclaration(v, _) => Some(v),
                ContractMember::EventDeclaration(_) => None,
            })
            .collect()
    }

    pub fn get_variable_declarations_without_dict(&self) -> Vec<VariableDeclaration> {
        let members = self.contract_members.clone();
        members
            .into_iter()
            .filter_map(|c| match c {
                ContractMember::VariableDeclaration(v, _) => {
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
        v.start_contract_declaration(self, ctx)?;

        self.identifier.visit(v, ctx)?;

        ctx.contract_declaration_context = Some(ContractDeclarationContext {
            identifier: self.identifier.clone(),
        });

        self.conformances.visit(v, ctx)?;

        self.type_states.visit(v, ctx)?;

        self.contract_members.visit(v, ctx)?;

        ctx.contract_declaration_context = None;

        v.finish_contract_declaration(self, ctx)?;

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContractMember {
    VariableDeclaration(VariableDeclaration, Option<Modifier>),
    EventDeclaration(EventDeclaration),
}

impl Visitable for ContractMember {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_contract_member(self, ctx)?;
        match self {
            ContractMember::VariableDeclaration(d, _) => d.visit(v, ctx),
            ContractMember::EventDeclaration(d) => d.visit(v, ctx),
        }?;
        v.finish_contract_member(self, ctx)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContractBehaviourDeclaration {
    pub identifier: Identifier,
    pub members: Vec<ContractBehaviourMember>,
    pub type_states: Vec<TypeState>,
    pub caller_binding: Option<Identifier>,
    pub caller_protections: Vec<CallerProtection>,
}

impl Visitable for ContractBehaviourDeclaration {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        ctx.contract_behaviour_declaration_context = Some(ContractBehaviourDeclarationContext {
            identifier: self.identifier.clone(),
            caller: self.caller_binding.clone(),
            type_states: self.type_states.clone(),
            caller_protections: self.caller_protections.clone(),
        });

        let local_variables: Vec<VariableDeclaration> = vec![];
        let mut parameters: Vec<Parameter> = vec![];

        if let Some(caller) = &self.caller_binding {
            parameters.push(Parameter {
                identifier: caller.clone(),
                type_assignment: Type::UserDefinedType(Identifier {
                    token: "&signer".to_string(),
                    enclosing_type: None,
                    line_info: Default::default(),
                }),
                expression: None,
                line_info: Default::default(),
            })
        }

        let scope = ScopeContext {
            parameters,
            local_variables,
            ..Default::default()
        };
        ctx.scope_context = Some(scope);

        v.start_contract_behaviour_declaration(self, ctx)?;

        self.identifier.visit(v, ctx)?;

        if let Some(ref mut caller) = self.caller_binding {
            caller.visit(v, ctx)?;
        }

        self.type_states.visit(v, ctx)?;

        self.caller_protections.visit(v, ctx)?;

        let scope = ctx.scope_context.clone();

        for member in &mut self.members {
            ctx.scope_context = scope.clone();
            member.visit(v, ctx)?;
        }

        ctx.contract_behaviour_declaration_context = None;
        ctx.scope_context = None;

        v.finish_contract_behaviour_declaration(self, ctx)?;

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
        v.start_contract_behaviour_member(self, ctx)?;
        match self {
            ContractBehaviourMember::FunctionDeclaration(f) => f.visit(v, ctx),
            ContractBehaviourMember::SpecialDeclaration(s) => s.visit(v, ctx),
            ContractBehaviourMember::FunctionSignatureDeclaration(f) => f.visit(v, ctx),
            ContractBehaviourMember::SpecialSignatureDeclaration(s) => s.visit(v, ctx),
        }?;
        v.finish_contract_behaviour_member(self, ctx)?;
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
        v.start_asset_declaration(self, ctx)?;

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

        self.identifier.visit(v, ctx)?;

        for member in &mut self.members {
            ctx.scope_context = Option::from(ScopeContext {
                parameters: vec![],
                local_variables: vec![],
                counter: 0,
            });
            member.visit(v, ctx)?;
        }

        ctx.asset_context = None;
        ctx.scope_context = None;

        v.finish_asset_declaration(self, ctx)?;

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
        match self {
            AssetMember::VariableDeclaration(d) => d.visit(v, ctx),
            AssetMember::SpecialDeclaration(s) => s.visit(v, ctx),
            AssetMember::FunctionDeclaration(f) => f.visit(v, ctx),
        }?;
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
                if let StructMember::VariableDeclaration(v, _) = m {
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
        v.start_struct_declaration(self, ctx)?;

        let struct_declaration_context = Some(StructDeclarationContext {
            identifier: self.identifier.clone(),
        });
        let scope_context = Some(ScopeContext {
            ..Default::default()
        });

        ctx.struct_declaration_context = struct_declaration_context;
        ctx.scope_context = scope_context;

        self.identifier.visit(v, ctx)?;

        for member in &mut self.members {
            ctx.scope_context = Option::from(ScopeContext {
                parameters: vec![],
                local_variables: vec![],
                counter: 0,
            });
            member.visit(v, ctx)?;
        }
        ctx.struct_declaration_context = None;
        ctx.scope_context = None;

        v.finish_struct_declaration(self, ctx)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StructMember {
    VariableDeclaration(VariableDeclaration, Option<Modifier>),
    FunctionDeclaration(FunctionDeclaration),
    SpecialDeclaration(SpecialDeclaration),
}

impl Visitable for StructMember {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_struct_member(self, ctx)?;
        match self {
            StructMember::FunctionDeclaration(f) => f.visit(v, ctx),
            StructMember::SpecialDeclaration(s) => s.visit(v, ctx),
            StructMember::VariableDeclaration(d, _) => d.visit(v, ctx),
        }?;
        v.finish_struct_member(self, ctx)?;
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

#[derive(Clone, Debug, PartialEq)]
pub struct EnumMember {
    pub case_token: std::string::String,
    pub identifier: Identifier,
    pub hidden_value: Option<Expression>,
    pub enum_type: Type,
}

impl Visitable for EnumMember {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_enum_member(self, ctx)?;
        self.identifier.visit(v, ctx)?;
        v.finish_enum_member(self, ctx)?;
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
        let modifiers: Vec<FunctionCall> = self
            .modifiers
            .clone()
            .into_iter()
            .filter(|f| f.identifier.token == "module".to_string())
            .collect();

        if let Some(argument) = modifiers.first().and_then(|m| m.arguments.first()) {
            if let Some(ref identifier) = argument.identifier {
                let name = &identifier.token;
                if name == "address" {
                    if let Expression::Literal(ref l) = argument.expression {
                        if let Literal::AddressLiteral(a) = l {
                            return Option::from(a.clone());
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
        v.start_trait_declaration(self, ctx)?;

        self.identifier.visit(v, ctx)?;

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
            member.visit(v, ctx)?;
        }

        ctx.trait_declaration_context = None;

        ctx.scope_context = None;

        v.finish_trait_declaration(self, ctx)?;
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
        match self {
            TraitMember::FunctionDeclaration(f) => f.visit(v, ctx),
            TraitMember::SpecialDeclaration(s) => s.visit(v, ctx),
            TraitMember::FunctionSignatureDeclaration(f) => f.visit(v, ctx),
            TraitMember::SpecialSignatureDeclaration(s) => s.visit(v, ctx),
            TraitMember::ContractBehaviourDeclaration(c) => c.visit(v, ctx),
            TraitMember::EventDeclaration(e) => e.visit(v, ctx),
        }?;
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
        v.start_function_declaration(self, ctx)?;
        self.head.visit(v, ctx)?;

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

        if let Some(ref mut scope_context) = ctx.scope_context {
            for parameter in &self.head.parameters {
                scope_context.parameters.push(parameter.clone());
            }
        }

        let mut statements: Vec<Vec<Statement>> = vec![];

        for statement in &mut self.body {
            ctx.pre_statements = vec![];
            ctx.post_statements = vec![];
            statement.visit(v, ctx)?;
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
        v.finish_function_declaration(self, ctx)?;

        ctx.pre_statements = vec![];
        ctx.post_statements = vec![];

        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct FunctionSignatureDeclaration {
    pub func_token: std::string::String,
    pub attributes: Vec<Attribute>,
    pub modifiers: Vec<Modifier>,
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
        self.modifiers.contains(&Modifier::Public)
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
        v.start_function_signature_declaration(self, ctx)?;

        self.identifier.visit(v, ctx)?;

        self.parameters.visit(v, ctx)?;

        if let Some(ref mut result_type) = self.result_type {
            result_type.visit(v, ctx)?;
        }

        v.finish_function_signature_declaration(self, ctx)?;

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
        self.head.modifiers.contains(&Modifier::Public)
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
        v.start_special_declaration(self, ctx)?;

        self.head.visit(v, ctx)?;

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

        if let Some(ref mut scope_context) = ctx.scope_context {
            for parameter in &self.head.parameters {
                scope_context.parameters.push(parameter.clone());
            }
            scope_context.parameters.push(Parameter {
                identifier: Identifier {
                    token: "caller".to_string(),
                    enclosing_type: None,
                    line_info: Default::default(),
                },
                type_assignment: Type::UserDefinedType(Identifier {
                    token: "&signer".to_string(),
                    enclosing_type: None,
                    line_info: Default::default(),
                }),
                expression: None,
                line_info: Default::default(),
            });
        }

        let mut statements: Vec<Vec<Statement>> = vec![];
        for statement in &mut self.body {
            ctx.pre_statements = vec![];
            ctx.post_statements = vec![];
            statement.visit(v, ctx)?;
            if let Statement::Expression(Expression::BinaryExpression(be)) = statement {
                if let Expression::Identifier(id) = &*be.rhs_expression {
                    if id.token == "caller" {
                        be.rhs_expression = Box::new(Expression::RawAssembly(
                            "Signer.address_of(copy(caller))".to_string(),
                            None,
                        ));
                    }
                }
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
        v.finish_special_declaration(self, ctx)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpecialSignatureDeclaration {
    pub special_token: std::string::String,
    pub enclosing_type: Option<String>,
    pub attributes: Vec<Attribute>,
    pub modifiers: Vec<Modifier>,
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
        v.start_special_signature_declaration(self, ctx)?;

        self.parameters.visit(v, ctx)?;

        v.finish_special_signature_declaration(self, ctx)?;
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
        v.start_parameter(self, ctx)?;
        self.type_assignment.visit(v, ctx)?;
        v.finish_parameter(self, ctx)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Modifier {
    Public,
    Visible,
}

impl std::fmt::Display for Modifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Modifier::Public => write!(f, "public"),
            Modifier::Visible => write!(f, "visible"),
        }
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
        match &self.declaration_token {
            Some(t) => t == "let",
            _ => false,
        }
    }

    #[allow(dead_code)]
    pub fn is_variable(&self) -> bool {
        match &self.declaration_token {
            Some(t) => t == "var",
            _ => false,
        }
    }
}

impl Visitable for VariableDeclaration {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_variable_declaration(self, ctx)?;

        self.identifier.visit(v, ctx)?;

        self.variable_type.visit(v, ctx)?;

        if let Some(ref mut expression) = self.expression {
            let previous_scope = ctx.scope_context.clone();
            ctx.scope_context = Option::from(ScopeContext {
                parameters: vec![],
                local_variables: vec![],
                counter: 0,
            });

            ctx.is_property_default_assignment = true;

            expression.visit(v, ctx)?;

            ctx.is_property_default_assignment = false;

            ctx.scope_context = previous_scope;
        }

        v.finish_variable_declaration(self, ctx)?;

        Ok(())
    }
}
