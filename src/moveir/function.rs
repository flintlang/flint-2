use super::identifier::MoveIdentifier;
use super::ir::{MoveIRBlock, MoveIRExpression, MoveIRStatement};
use super::r#type::MoveType;
use super::statement::MoveStatement;
use super::*;
use crate::ast::{
    Expression, FunctionDeclaration, Identifier, Statement, Type, VariableDeclaration,
};
use crate::context::ScopeContext;
use crate::environment::Environment;
use crate::moveir::preprocessor::utils::release;
use crate::moveir::utils::*;
use crate::type_checker::ExpressionChecker;

#[derive(Debug)]
pub(crate) struct MoveFunction {
    pub function_declaration: FunctionDeclaration,
    pub environment: Environment,
    pub is_contract_function: bool,
    pub enclosing_type: Identifier,
}

impl MoveFunction {
    pub(crate) fn generate(&self, _return: bool) -> String {
        let scope = self
            .function_declaration
            .scope_context
            .clone()
            .unwrap_or_default();

        let function_context = FunctionContext {
            environment: self.environment.clone(),
            scope_context: scope,
            enclosing_type: self.enclosing_type.token.clone(),
            block_stack: vec![MoveIRBlock { statements: vec![] }],
            in_struct_function: !self.is_contract_function,
            is_constructor: false,
        };

        let modifiers = self
            .function_declaration
            .head
            .modifiers
            .clone()
            .into_iter()
            .filter(|s| s == &Modifier::Public)
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
            .join(",");

        let name = self.function_declaration.head.identifier.token.clone();
        let name = self
            .function_declaration
            .mangled_identifier
            .as_ref()
            .unwrap_or(&name);
        let parameter_move_types: Vec<MoveType> = self
            .function_declaration
            .head
            .parameters
            .clone()
            .into_iter()
            .map(|p| MoveType::move_type(p.type_assignment, Option::from(self.environment.clone())))
            .collect();
        let parameters: Vec<MoveIRExpression> = self
            .function_declaration
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
        let parameters: Vec<String> = parameters
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

        let result_type = match self.function_declaration.get_result_type() {
            Some(ref result) if _return => {
                let result =
                    MoveType::move_type(result.clone(), Option::from(self.environment.clone()));
                format!("{}", result.generate(&function_context))
            }
            _ => "".to_string(),
        };
        let tags = self.function_declaration.tags.join("");

        let mut scope = self
            .function_declaration
            .scope_context
            .clone()
            .unwrap_or_default();

        let variables: Vec<Expression> = self
            .function_declaration
            .body
            .clone()
            .into_iter()
            .filter_map(|v| {
                if let Statement::Expression(e) = v {
                    Some(e)
                } else {
                    None
                }
            })
            .collect();

        let mut variables: Vec<VariableDeclaration> = variables
            .into_iter()
            .filter_map(|v| {
                if let Expression::VariableDeclaration(e) = v {
                    Some(e)
                } else {
                    None
                }
            })
            .collect();

        let mut all_variables = scope.local_variables.clone();
        all_variables.append(&mut variables);

        scope.local_variables = all_variables;
        let mut function_context = FunctionContext {
            environment: self.environment.clone(),
            enclosing_type: self.enclosing_type.token.clone(),
            block_stack: vec![MoveIRBlock { statements: vec![] }],
            scope_context: scope,
            in_struct_function: !self.is_contract_function,
            is_constructor: false,
        };
        let statements = self.function_declaration.body.clone();
        let mut statements: Vec<MoveStatement> = statements
            .into_iter()
            .map(|s| MoveStatement { statement: s })
            .collect();
        while !statements.is_empty() {
            let statement = statements.remove(0);
            let statement = statement.generate(&mut function_context);
            function_context.emit(statement);
        }

        let mut release_references = function_context.get_release_references();

        let mut new_statements = Vec::new();

        if let Some(block) = function_context.block_stack.pop() {
            for statement in block.statements.iter().rev() {
                if let MoveIRStatement::Expression(e) = statement {
                    let (remaining, new_expr) = remove_moves(release_references, e.clone());
                    new_statements.push(MoveIRStatement::Expression(new_expr));
                    release_references = remaining;
                } else {
                    new_statements.push(statement.clone());
                }
            }
        }

        new_statements.reverse();

        let new_block = MoveIRBlock {
            statements: new_statements,
        };
        function_context.block_stack.push(new_block);

        for reference in release_references {
            if let Statement::Expression(Expression::Identifier(id)) = reference {
                let expression = MoveIdentifier {
                    identifier: id,
                    position: Default::default(),
                }
                .generate(&function_context, true, false);
                function_context.emit(MoveIRStatement::Inline(format!("_ = {}", expression)));
            }
        }

        let body = function_context.generate();
        if result_type.is_empty() {
            let result = format!(
                " {modifiers} {name} ({parameters}) {tags} {{ \n {body} \n }}",
                modifiers = modifiers,
                name = name,
                parameters = parameters,
                tags = tags,
                body = body
            );
            return result;
        }

        let result = format!(
            " {modifiers} {name} ({parameters}): {result} {tags} {{ \n {body} \n }}",
            modifiers = modifiers,
            name = name,
            parameters = parameters,
            result = result_type,
            tags = tags,
            body = body
        );
        result
    }
}

pub fn reorder_statements(
    statements: &[MoveIRStatement],
) -> impl Iterator<Item=&MoveIRStatement> + '_ {
    let declarations = statements.into_iter().filter(|statement| {
        matches!(
            statement,
            MoveIRStatement::Expression(MoveIRExpression::VariableDeclaration(_))
        )
    });

    let non_declarations = statements.into_iter().filter(|statement| {
        !matches!(
            statement,
            MoveIRStatement::Expression(MoveIRExpression::VariableDeclaration(_))
        )
    });

    declarations.chain(non_declarations)
}

#[derive(Debug, Default, Clone)]
pub struct FunctionContext {
    pub environment: Environment,
    pub scope_context: ScopeContext,
    pub enclosing_type: String,
    pub block_stack: Vec<MoveIRBlock>,
    pub in_struct_function: bool,
    pub is_constructor: bool,
}

impl FunctionContext {
    pub fn generate(&mut self) -> String {
        let block = self.block_stack.last();
        if !self.block_stack.is_empty() {
            let statements = block.unwrap().statements.clone();
            let statements = reorder_statements(&statements);
            let statements: Vec<String> = statements
                .into_iter()
                .map(|s| format!("{s}", s = s))
                .collect();
            return statements.join("\n");
        }

        String::from("")
    }

    pub fn emit(&mut self, statement: MoveIRStatement) {
        let count = self.block_stack.len();
        let block = self.block_stack.get_mut(count - 1);
        block.unwrap().statements.push(statement);
    }

    pub fn with_new_block(&mut self, count: usize) -> MoveIRBlock {
        while self.block_stack.len() != count {
            let block = MoveIRStatement::Block(self.pop_block());
            self.emit(block);
        }
        self.pop_block()
    }

    pub fn push_block(&mut self) -> usize {
        self.block_stack.push(MoveIRBlock { statements: vec![] });
        self.block_stack.len()
    }

    pub fn pop_block(&mut self) -> MoveIRBlock {
        self.block_stack.pop().unwrap()
    }
    pub fn emit_release_references(&mut self) {
        let references: Vec<Identifier> = self
            .scope_context
            .parameters
            .clone()
            .into_iter()
            .filter(|i| i.is_inout())
            .map(|p| p.identifier)
            .collect();
        for reference in references {
            let expression = MoveIdentifier {
                identifier: reference,
                position: Default::default(),
            }
            .generate(self, true, false);
            self.emit(MoveIRStatement::Inline(format!("_ = {}", expression)))
        }
    }

    pub fn get_release_references(&self) -> Vec<Statement> {
        let mut release_references = Vec::new();
        let references: Vec<Parameter> = self
            .scope_context
            .parameters
            .clone()
            .into_iter()
            .filter(|i| i.is_inout())
            .collect();
        for reference in references {
            let statement = release(
                Expression::Identifier(reference.identifier.clone()),
                Type::InoutType(InoutType {
                    key_type: Box::new(reference.type_assignment),
                }),
            );
            release_references.push(statement);
        }
        release_references
    }

    pub fn self_type(&self) -> Type {
        if let Some(self_type) = self.scope_context.type_for(Identifier::SELF) {
            self_type
        } else {
            self.environment.get_expression_type(
                &Expression::SelfExpression,
                &self.enclosing_type,
                &[],
                &[],
                &self.scope_context,
            )
        }
    }
}
