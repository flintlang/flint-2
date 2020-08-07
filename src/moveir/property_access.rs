use super::expression::MoveExpression;
use super::function::FunctionContext;
use super::identifier::MoveIdentifier;
use super::ir::{MoveIRExpression, MoveIROperation};
use super::MovePosition;
use crate::ast::expressions::Expression::SelfExpression;
use crate::ast::{mangle, Expression, Type};
use crate::moveir::ir::MoveIRExpression::{Identifier, Transfer};
use crate::moveir::ir::MoveIRTransfer;

#[derive(Debug)]
pub(crate) struct MovePropertyAccess {
    pub left: Expression,
    pub right: Expression,
    pub position: MovePosition,
}

impl MovePropertyAccess {
    pub fn generate(&self, function_context: &FunctionContext, f_call: bool) -> MoveIRExpression {
        if let Expression::Identifier(ref identifier) = self.left {
            if let Expression::Identifier(ref property) = self.right {
                // TODO REMOVE if jess' fixes this
                let defined_type = function_context.scope_context.type_for(&identifier.token);

                if let Some(Type::InoutType(inout)) = defined_type {
                    if let Type::UserDefinedType(user_defined) = *inout.key_type {
                        if function_context.is_constructor
                            && function_context
                                .environment
                                .is_struct_declared(user_defined.token.as_str())
                        {
                            // TODO this still will not work because you need the reference to it
                            // I will not do that here as I do not know how it will interact with Jess' code
                            // It may also be that none of this will be needed
                            let copied_struct = Transfer(MoveIRTransfer::Copy(Box::from(
                                Identifier(mangle(identifier.token.as_str())),
                            )));
                            return MoveIRExpression::Operation(MoveIROperation::Dereference(
                                Box::from(MoveIRExpression::Operation(
                                    MoveIROperation::MutableReference(Box::from(
                                        MoveIRExpression::Operation(MoveIROperation::Access(
                                            Box::from(copied_struct),
                                            property.token.clone(),
                                        )),
                                    )),
                                )),
                            ));
                        }
                    }
                }

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
                        .generate(&function_context);
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
            .generate(&function_context);
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
