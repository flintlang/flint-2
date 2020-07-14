use crate::moveir::*;

#[derive(Debug)]
pub struct MoveSelf {
    pub token: String,
    pub position: MovePosition,
}

impl MoveSelf {
    pub fn generate(&self, function_context: &FunctionContext, force: bool) -> MoveIRExpression {
        if function_context.is_constructor {}
        if let MovePosition::Left = self.position {
            MoveIRExpression::Identifier(self.name())
        } else if force {
            MoveIRExpression::Transfer(MoveIRTransfer::Move(Box::from(
                MoveIRExpression::Identifier(self.name()),
            )))
        } else if !function_context.self_type().is_inout_type() {
            MoveIRExpression::Identifier(self.name())
        } else if let MovePosition::Accessed = self.position {
            MoveIRExpression::Operation(MoveIROperation::Dereference(Box::from(
                MoveIRExpression::Operation(MoveIROperation::MutableReference(Box::from(
                    MoveIRExpression::Transfer(MoveIRTransfer::Copy(Box::from(
                        MoveIRExpression::Identifier(self.name()),
                    ))),
                ))),
            )))
        } else {
            MoveIRExpression::Transfer(MoveIRTransfer::Copy(Box::from(
                MoveIRExpression::Identifier(self.name()),
            )))
        }
    }

    pub fn name(&self) -> String {
        "this".to_string()
    }
}
