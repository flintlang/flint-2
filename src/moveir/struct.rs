use super::declaration::MoveFieldDeclaration;
use super::function::{FunctionContext, MoveFunction};
use super::identifier::MoveIdentifier;
use super::ir::{
    MoveIRBlock, MoveIRExpression, MoveIRStatement, MoveIRStructConstructor, MoveIRTransfer,
    MoveIRType, MoveIRVariableDeclaration,
};
use super::r#type::MoveType;
use super::statement::MoveStatement;
use super::MovePosition;
use crate::ast::{
    BinOp, Expression, FunctionDeclaration, Identifier, Modifier, SpecialDeclaration, Statement,
    StructDeclaration, StructMember, Type, VariableDeclaration,
};
use crate::context::ScopeContext;
use crate::environment::Environment;

pub(crate) struct MoveStruct {
    pub struct_declaration: StructDeclaration,
    pub environment: Environment,
}

impl MoveStruct {
    pub(crate) fn generate(&self) -> String {
        let function_context = FunctionContext {
            environment: self.environment.clone(),
            scope_context: Default::default(),
            enclosing_type: self.struct_declaration.identifier.token.clone(),
            block_stack: vec![],
            in_struct_function: true,
            is_constructor: false,
        };

        let members: Vec<MoveIRExpression> = self
            .struct_declaration
            .members
            .clone()
            .into_iter()
            .filter_map(|s| match s {
                StructMember::VariableDeclaration(v, _) => {
                    Some(MoveFieldDeclaration { declaration: v }.generate(&function_context))
                }
                _ => None,
            })
            .collect();
        let members: Vec<String> = members.into_iter().map(|e| format!("{}", e)).collect();
        let members = members.join(",\n");
        let kind = MoveType::move_type(
            Type::UserDefinedType(self.struct_declaration.identifier.clone()),
            Option::from(self.environment.clone()),
        );
        let kind = if kind.is_resource() {
            "resource".to_string()
        } else {
            "struct".to_string()
        };
        let result = format!(
            "{kind} {name} {{ \n {members} \n }}",
            kind = kind,
            name = self.struct_declaration.identifier.token,
            members = members
        );
        result
    }

    pub fn generate_all_functions(&self) -> String {
        format!(
            "{initialisers} \n\n {functions}",
            initialisers = self.generate_initialisers(),
            functions = self.generate_functions()
        )
    }
    pub fn generate_initialisers(&self) -> String {
        let initialisers: Vec<SpecialDeclaration> = self
            .struct_declaration
            .members
            .clone()
            .into_iter()
            .filter_map(|m| {
                if let StructMember::SpecialDeclaration(s) = m {
                    if s.is_init() {
                        return Some(s);
                    }
                }
                None
            })
            .collect();
        let initialisers: Vec<String> = initialisers
            .into_iter()
            .map(|i| {
                MoveStructInitialiser {
                    declaration: i,
                    identifier: self.struct_declaration.identifier.clone(),
                    environment: self.environment.clone(),
                    properties: self.struct_declaration.get_variable_declarations(),
                }
                .generate()
            })
            .collect();
        initialisers.join("\n\n")
    }

    pub fn generate_functions(&self) -> String {
        let functions: Vec<FunctionDeclaration> = self
            .struct_declaration
            .members
            .clone()
            .into_iter()
            .filter_map(|m| {
                if let StructMember::FunctionDeclaration(f) = m {
                    return Some(f);
                }
                None
            })
            .collect();
        let functions: Vec<String> = functions
            .into_iter()
            .map(|f| {
                MoveFunction {
                    function_declaration: f,
                    environment: self.environment.clone(),
                    is_contract_function: false,
                    enclosing_type: self.struct_declaration.identifier.clone(),
                }
                .generate(true)
            })
            .collect();
        functions.join("\n\n")
    }
}

pub(crate) struct MoveStructInitialiser {
    pub declaration: SpecialDeclaration,
    pub identifier: Identifier,
    pub environment: Environment,
    pub properties: Vec<VariableDeclaration>,
}

impl MoveStructInitialiser {
    pub fn generate(&self) -> String {
        let modifiers = self
            .declaration
            .head
            .modifiers
            .clone()
            .into_iter()
            .filter_map(|s| {
                if s == Modifier::Public {
                    Some(s.to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<String>>()
            .join(",");

        let scope = ScopeContext {
            parameters: self.declaration.head.parameters.clone(),
            local_variables: vec![],
            counter: 0,
        };

        let function_context = FunctionContext {
            environment: self.environment.clone(),
            scope_context: scope,
            enclosing_type: self.identifier.token.clone(),
            block_stack: vec![MoveIRBlock { statements: vec![] }],
            in_struct_function: true,
            is_constructor: false,
        };

        let parameter_move_types: Vec<MoveType> = self
            .declaration
            .head
            .parameters
            .clone()
            .into_iter()
            .map(|p| MoveType::move_type(p.type_assignment, Option::from(self.environment.clone())))
            .collect();

        let parameter_name: Vec<MoveIRExpression> = self
            .declaration
            .head
            .parameters
            .clone()
            .into_iter()
            .map(|p| {
                MoveIdentifier {
                    identifier: p.identifier,
                    position: MovePosition::Left,
                }
                .generate(&function_context, false, false)
            })
            .collect();

        let parameter_name: Vec<String> = parameter_name
            .into_iter()
            .map(|p| format!("{}", p))
            .collect();

        let parameters: Vec<String> = parameter_name
            .into_iter()
            .zip(parameter_move_types.into_iter())
            .map(|(p, t)| {
                format!(
                    "{parameter}: {ir_type}",
                    parameter = p,
                    ir_type = t.generate(&function_context)
                )
            })
            .collect();
        let parameters = parameters.join(", ");

        let result_type = Type::from_identifier(self.identifier.clone());
        let result_type = MoveType::move_type(result_type, Option::from(self.environment.clone()));
        let result_type = result_type.generate(&function_context);

        let mut function_context = FunctionContext {
            environment: self.environment.clone(),
            scope_context: self.declaration.scope_context.clone(),
            enclosing_type: self.identifier.token.clone(),
            block_stack: vec![MoveIRBlock { statements: vec![] }],
            in_struct_function: true,
            is_constructor: true,
        };

        let body = if self.declaration.body.is_empty() {
            "".to_string()
        } else {
            let mut properties = self.properties.clone();
            for _ in &self.properties {
                let property = properties.remove(0);
                let property_type = MoveType::move_type(
                    property.variable_type,
                    Option::from(self.environment.clone()),
                )
                .generate(&function_context);
                let name = format!("__this_{}", property.identifier.token);
                function_context.emit(MoveIRStatement::Expression(
                    MoveIRExpression::VariableDeclaration(MoveIRVariableDeclaration {
                        identifier: name,
                        declaration_type: property_type,
                    }),
                ));
            }

            let mut unassigned: Vec<Identifier> = self
                .properties
                .clone()
                .into_iter()
                .map(|v| v.identifier)
                .collect();
            let mut statements = self.declaration.body.clone();
            while !(statements.is_empty() || unassigned.is_empty()) {
                let statement = statements.remove(0);
                if let Statement::Expression(e) = statement.clone() {
                    if let Expression::BinaryExpression(b) = e {
                        if let BinOp::Equal = b.op {
                            match *b.lhs_expression {
                                Expression::Identifier(i) => {
                                    if let Some(ref enclosing) = i.enclosing_type {
                                        if self.identifier.token == *enclosing {
                                            unassigned = unassigned
                                                .into_iter()
                                                .filter(|u| u.token != i.token)
                                                .collect();
                                        }
                                    }
                                }
                                Expression::BinaryExpression(be) => {
                                    let op = be.op.clone();
                                    let lhs = *be.lhs_expression;
                                    let rhs = *be.rhs_expression;
                                    if let BinOp::Dot = op {
                                        if let Expression::SelfExpression = lhs {
                                            if let Expression::Identifier(i) = rhs {
                                                unassigned = unassigned
                                                    .into_iter()
                                                    .filter(|u| u.token != i.token)
                                                    .collect();
                                            }
                                        }
                                    }
                                }
                                _ => break,
                            }
                        }
                    }
                }

                let statement = MoveStatement { statement }.generate(&mut function_context);
                function_context.emit(statement);
            }

            let fields = self.properties.clone();
            let fields = fields
                .into_iter()
                .map(|f| {
                    let name = format!("__this_{}", f.identifier.token);
                    (
                        f.identifier.token,
                        MoveIRExpression::Transfer(MoveIRTransfer::Move(Box::from(
                            MoveIRExpression::Identifier(name),
                        ))),
                    )
                })
                .collect();

            let constructor = MoveIRExpression::StructConstructor(MoveIRStructConstructor {
                identifier: self.identifier.clone(),
                fields,
            });

            if statements.is_empty() {
                function_context.emit_release_references();
                function_context.emit(MoveIRStatement::Return(constructor));
                function_context.generate()
            } else {
                function_context.is_constructor = false;

                function_context.emit_release_references();

                let self_type = MoveType::move_type(
                    Type::type_from_identifier(self.identifier.clone()),
                    Option::from(self.environment.clone()),
                )
                .generate(&function_context);

                let emit = MoveIRExpression::VariableDeclaration(MoveIRVariableDeclaration {
                    identifier: "this".to_string(),
                    declaration_type: MoveIRType::MutableReference(Box::from(self_type.clone())),
                });
                function_context.emit(MoveIRStatement::Expression(emit));

                let emit = MoveIRExpression::VariableDeclaration(MoveIRVariableDeclaration {
                    identifier: "".to_string(),
                    declaration_type: self_type,
                });
                function_context.emit(MoveIRStatement::Expression(emit));

                function_context.generate()
            }
        };

        format!(
            "{modifiers} {name}_init({parameters}): {result_type} {{ \n\n {body} \n\n }}",
            modifiers = modifiers,
            result_type = result_type,
            name = self.identifier.token,
            parameters = parameters,
            body = body
        )
    }
}
