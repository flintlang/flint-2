use crate::moveir::*;

#[derive(Debug)]
pub struct MovePropertyAccess {
    pub left: Expression,
    pub right: Expression,
    pub position: MovePosition,
}

impl MovePropertyAccess {
    pub fn generate(&self, function_context: &FunctionContext, f_call: bool) -> MoveIRExpression {
        if let Expression::Identifier(e) = self.left.clone() {
            if let Expression::Identifier(p) = self.right.clone() {
                if function_context.environment.is_enum_declared(&e.token) {
                    let property = function_context.environment.property(p.token, &e.token);
                    if property.is_some() {
                        return MoveExpression {
                            expression: property.unwrap().property.get_value().unwrap(),
                            position: self.position.clone(),
                        }
                        .generate(function_context);
                    }
                }
            }
        }
        let rhs_enclosing = self.right.enclosing_identifier();
        if rhs_enclosing.is_some() {
            if function_context.is_constructor {
                return MoveIdentifier {
                    identifier: rhs_enclosing.unwrap(),
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
                            rhs_enclosing.unwrap().token,
                        ));
                    }
                }
            }
            return MoveIRExpression::Operation(MoveIROperation::Access(
                Box::from(lhs),
                rhs_enclosing.unwrap().token,
            ));
        }
        panic!("Fatal Error: {:?}", self.right.clone())
    }
}