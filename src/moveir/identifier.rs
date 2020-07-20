use super::function::FunctionContext;
use super::ir::{MoveIRExpression, MoveIROperation, MoveIRTransfer};
use super::property_access::MovePropertyAccess;
use super::MovePosition;
use crate::ast::{mangle, Expression, Identifier};

pub(crate) struct MoveIdentifier {
    pub identifier: Identifier,
    pub position: MovePosition,
}

impl MoveIdentifier {
    pub fn generate(
        &self,
        function_context: &FunctionContext,
        force: bool,
        f_call: bool,
    ) -> MoveIRExpression {
        if self.identifier.enclosing_type.is_some() {
            //REMOVEBEFOREFLIGHT
            return if function_context.is_constructor {
                let name = "__this_".to_owned() + &self.identifier.token.clone();
                MoveIRExpression::Identifier(name)
            } else {
                MovePropertyAccess {
                    left: Expression::SelfExpression,
                    right: Expression::Identifier(self.identifier.clone()),
                    position: self.position.clone(),
                }
                .generate(function_context, f_call)
            };
        };

        if self.identifier.is_self() {
            return MoveSelf {
                token: self.identifier.token.clone(),
                position: self.position.clone(),
            }
            .generate(function_context, force);
        }

        let ir_identifier = MoveIRExpression::Identifier(mangle(&self.identifier.token));

        if force {
            return MoveIRExpression::Transfer(MoveIRTransfer::Move(Box::from(ir_identifier)));
        }

        if let Some(identifier_type) = function_context
            .scope_context
            .type_for(&self.identifier.token)
        {
            if identifier_type.is_currency_type() && f_call {
                return MoveIRExpression::Transfer(MoveIRTransfer::Move(Box::from(ir_identifier)));
            }
            if identifier_type.is_currency_type() {
                return ir_identifier;
            }
            if !identifier_type.is_inout_type() && identifier_type.is_user_defined_type() {
                if f_call {
                    return MoveIRExpression::Transfer(MoveIRTransfer::Move(Box::from(
                        ir_identifier,
                    )));
                } else {
                    return ir_identifier;
                }
            }
        }

        dbg!(ir_identifier.clone());
        dbg!(&self.position);

        if let MovePosition::Left = self.position {
            return ir_identifier;
        }

        if f_call {
            if let MovePosition::Accessed = self.position.clone() {
                let expression =
                    MoveIRExpression::Transfer(MoveIRTransfer::Copy(Box::from(ir_identifier)));
                let expression = MoveIRExpression::Operation(MoveIROperation::MutableReference(
                    Box::from(expression),
                ));
                return expression;
            }
        }

        if let MovePosition::Accessed = self.position {
            let expression =
                MoveIRExpression::Transfer(MoveIRTransfer::Copy(Box::from(ir_identifier)));
            let expression = MoveIRExpression::Operation(MoveIROperation::MutableReference(
                Box::from(expression),
            ));

            MoveIRExpression::Operation(MoveIROperation::Dereference(Box::from(expression)))
        } else {
            MoveIRExpression::Transfer(MoveIRTransfer::Copy(Box::from(ir_identifier)))
        }
    }
}

#[derive(Debug)]
pub(crate) struct MoveSelf {
    pub token: String,
    pub position: MovePosition,
}

impl MoveSelf {
    pub fn generate(&self, function_context: &FunctionContext, force: bool) -> MoveIRExpression {
        if function_context.is_constructor {}
        if let MovePosition::Left = self.position {
            //MoveIRExpression::Operation(MoveIROperation::Dereference(Box::from(
            //  MoveIRExpression::Operation(MoveIROperation::MutableReference(Box::from(
            //    MoveIRExpression::Transfer(MoveIRTransfer::Copy(Box::from(
            //      MoveIRExpression::Identifier(self.name()),
            //))),
            //))),
            //)))
            MoveIRExpression::Identifier(self.name())
        } else if force {
            MoveIRExpression::Transfer(MoveIRTransfer::Move(Box::from(
                MoveIRExpression::Identifier(self.name()),
            )))
        } else if !function_context.self_type().is_inout_type() {
            MoveIRExpression::Identifier(self.name())
        } else if let MovePosition::Accessed = self.position {
            MoveIRExpression::Operation(MoveIROperation::Dereference(Box::from(
                //MoveIRExpression::Operation(MoveIROperation::Reference(Box::from(
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
