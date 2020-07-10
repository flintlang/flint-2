use crate::moveir::*;

pub struct MoveIdentifier {
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
            if function_context.is_constructor {
                let name = "__this_".to_owned() + &self.identifier.token.clone();
                return MoveIRExpression::Identifier(name);
            } else {
                return MovePropertyAccess {
                    left: Expression::SelfExpression,
                    right: Expression::Identifier(self.identifier.clone()),
                    position: self.position.clone(),
                }
                .generate(function_context, f_call);
            }
        };

        if self.identifier.is_self() {
            return MoveSelf {
                token: self.identifier.token.clone(),
                position: self.position.clone(),
            }
            .generate(function_context, force);
        }

        let ir_identifier = MoveIRExpression::Identifier(mangle(self.identifier.token.clone()));

        if force {
            return MoveIRExpression::Transfer(MoveIRTransfer::Move(Box::from(ir_identifier)));
        }

        let identifier_type = function_context
            .scope_context
            .type_for(self.identifier.token.clone());
        if identifier_type.is_some() {
            let unwrapped_type = identifier_type.unwrap();
            if unwrapped_type.is_currency_type() && f_call {
                return MoveIRExpression::Transfer(MoveIRTransfer::Move(Box::from(ir_identifier)));
            }
            if unwrapped_type.is_currency_type() {
                return ir_identifier;
            }
            if !unwrapped_type.is_inout_type() && unwrapped_type.is_user_defined_type() {
                return MoveIRExpression::Operation(MoveIROperation::MutableReference(Box::from(
                    ir_identifier,
                )));
            }
        }

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
