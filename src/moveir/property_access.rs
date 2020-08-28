use super::expression::MoveExpression;
use super::function::FunctionContext;
use super::identifier::MoveIdentifier;
use super::ir::{MoveIRExpression, MoveIROperation};
use super::MovePosition;
use crate::ast::expressions::Expression::SelfExpression;
use crate::ast::Expression;

#[derive(Debug)]
pub(crate) struct MovePropertyAccess {
    pub left: Expression,
    pub right: Expression,
    pub position: MovePosition,
}

impl MovePropertyAccess {
    pub fn generate(&self, mut function_context: &mut FunctionContext, f_call: bool) -> MoveIRExpression {
        if let Expression::Identifier(ref identifier) = self.left {
            if let Expression::Identifier(ref property) = self.right {
                if function_context
                    .environment
                    .is_enum_declared(&identifier.token)
                {
                    if let Some(property) = function_context
                        .environment
                        .property(&property.token, &identifier.token)
                    {
                        return MoveExpression {
                            expression: property.property.get_value().unwrap(),
                            position: self.position.clone(),
                        }
                        .generate(&mut function_context);
                    }
                }
            }
        }

        if let Some(rhs_enclosing) = self.right.enclosing_identifier() {
            if function_context.is_constructor && self.left == SelfExpression {
                return MoveIdentifier {
                    identifier: rhs_enclosing.clone(),
                    position: self.position.clone(),
                }
                .generate(function_context, false, false);
            }
            let position = if let MovePosition::Inout = self.position {
                MovePosition::Inout
            } else {
                MovePosition::Accessed
            };
            let lhs = MoveExpression {
                expression: self.left.clone(),
                position,
            }
            .generate(&mut function_context);
            if f_call {
                if let MoveIRExpression::Operation(ref operation) = lhs {
                    if let MoveIROperation::Dereference(ref deref) = operation {
                        return MoveIRExpression::Operation(MoveIROperation::Access(
                            deref.clone(),
                            rhs_enclosing.token.clone(),
                        ));
                    }
                }
            }
            MoveIRExpression::Operation(MoveIROperation::Access(
                Box::from(lhs),
                rhs_enclosing.token.clone(),
            ))
        } else {
            panic!("Fatal Error: {:?}", self)
        }
    }
}
