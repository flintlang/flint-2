use super::expression::MoveExpression;
use super::function::FunctionContext;
use super::identifier::MoveIdentifier;
use super::ir::{MoveIRExpression, MoveIROperation};
use super::MovePosition;
use crate::ast::Expression;

#[derive(Debug)]
pub(crate) struct MovePropertyAccess {
    pub left: Expression,
    pub right: Expression,
    pub position: MovePosition,
}

impl MovePropertyAccess {
    pub fn generate(&self, function_context: &FunctionContext, f_call: bool) -> MoveIRExpression {
        if let Expression::Identifier(e) = self.left.clone() {
            if let Expression::Identifier(p) = self.right.clone() {
                if function_context.environment.is_enum_declared(&e.token) {
                    if let Some(property) =
                        function_context.environment.property(&p.token, &e.token)
                    {
                        return MoveExpression {
                            expression: property.property.get_value().unwrap(),
                            position: self.position.clone(),
                        }
                            .generate(function_context);
                    }
                }
            }
        }

        if let Some(rhs_enclosing) = self.right.enclosing_identifier() {
            if function_context.is_constructor {
                return MoveIdentifier {
                    identifier: rhs_enclosing,
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
            .generate(function_context);
            if f_call {
                let exp = lhs.clone();
                if let MoveIRExpression::Operation(o) = exp {
                    if let MoveIROperation::Dereference(e) = o {
                        return MoveIRExpression::Operation(MoveIROperation::Access(
                            e,
                            rhs_enclosing.token,
                        ));
                    }
                }
            }
            MoveIRExpression::Operation(MoveIROperation::Access(
                Box::from(lhs),
                rhs_enclosing.token,
            ))
        } else {
            panic!("Fatal Error: {:?}", self)
        }
    }
}
