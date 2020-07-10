use crate::moveir::*;

pub struct MoveFieldDeclaration {
    pub declaration: VariableDeclaration,
}

impl MoveFieldDeclaration {
    pub fn generate(&self, function_context: &FunctionContext) -> MoveIRExpression {
        let ir_type = MoveType::move_type(
            self.declaration.variable_type.clone(),
            Option::from(function_context.environment.clone()),
        )
        .generate(function_context);

        MoveIRExpression::FieldDeclaration(MoveIRFieldDeclaration {
            identifier: self.declaration.identifier.token.clone(),
            declaration_type: ir_type,
            expression: None,
        })
    }
}

pub struct MoveVariableDeclaration {
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
