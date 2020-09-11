use super::function::FunctionContext;
use super::ir::{MoveIRExpression, MoveIRFieldDeclaration, MoveIRVariableDeclaration};
use super::r#type::MoveType;
use crate::ast::VariableDeclaration;

pub(crate) struct MoveFieldDeclaration {
    pub declaration: VariableDeclaration,
}

impl MoveFieldDeclaration {
    pub fn generate(&self, function_context: &FunctionContext) -> MoveIRExpression {
        let ir_type = MoveType::move_type(
            self.declaration.variable_type.clone(),
            Option::from(function_context.environment.clone()),
        )
        .generate(function_context);

        if let Some(expr) = &self.declaration.expression {
            MoveIRExpression::FieldDeclaration(MoveIRFieldDeclaration {
                identifier: self.declaration.identifier.token.clone(),
                declaration_type: ir_type,
                expression: Some(*expr.clone()),
            })
        } else {
            MoveIRExpression::FieldDeclaration(MoveIRFieldDeclaration {
                identifier: self.declaration.identifier.token.clone(),
                declaration_type: ir_type,
                expression: None,
            })
        }
    }
}

pub(crate) struct MoveVariableDeclaration {
    pub declaration: VariableDeclaration,
}

impl MoveVariableDeclaration {
    pub fn generate(&self, function_context: &FunctionContext) -> MoveIRExpression {
        let ir_type = MoveType::move_type(
            self.declaration.variable_type.clone(),
            Option::from(function_context.environment.clone()),
        )
        .generate(function_context);

        if self.declaration.identifier.is_self() {
            return MoveIRExpression::VariableDeclaration(MoveIRVariableDeclaration {
                identifier: "this".to_string(),
                declaration_type: ir_type,
            });
        }
        MoveIRExpression::VariableDeclaration(MoveIRVariableDeclaration {
            identifier: self.declaration.identifier.token.clone(),
            declaration_type: ir_type,
        })
    }
}
