use super::assignment::MoveAssignment;
use super::call::{MoveExternalCall, MoveFunctionCall};
use super::declaration::MoveVariableDeclaration;
use super::function::FunctionContext;
use super::identifier::MoveIdentifier;
use super::ir::{
    MoveIRExpression, MoveIRFunctionCall, MoveIRLiteral, MoveIROperation, MoveIRVector,
};
use super::literal::MoveLiteralToken;
use super::property_access::MovePropertyAccess;
use super::r#type::MoveType;
use super::runtime_function::MoveRuntimeFunction;
use super::*;
use crate::ast::{
    mangle_dictionary, AttemptExpression, BinOp, BinaryExpression, CastExpression, Expression,
    Identifier, InoutExpression, SubscriptExpression, Type,
};
use crate::moveir::identifier::MoveSelf;

pub(crate) struct MoveExpression {
    pub expression: Expression,
    pub position: MovePosition,
}

impl MoveExpression {
    pub fn generate(&self, function_context: &FunctionContext) -> MoveIRExpression {
        match self.expression.clone() {
            Expression::Identifier(i) => MoveIdentifier {
                identifier: i,
                position: self.position.clone(),
            }
            .generate(function_context, false, false),
            Expression::BinaryExpression(b) => MoveBinaryExpression {
                expression: b,
                position: self.position.clone(),
            }
            .generate(function_context),
            Expression::InoutExpression(i) => MoveInoutExpression {
                expression: i,
                position: self.position.clone(),
            }
            .generate(function_context),
            Expression::ExternalCall(f) => {
                MoveExternalCall { external_call: f }.generate(function_context)
            }
            Expression::FunctionCall(f) => {
                MoveFunctionCall {
                    function_call: f,
                    module_name: "Self".to_string(),
                }
            }
            .generate(function_context),
            Expression::VariableDeclaration(v) => {
                MoveVariableDeclaration { declaration: v }.generate(function_context)
            }
            Expression::BracketedExpression(b) => MoveExpression {
                expression: *b.expression,
                position: Default::default(),
            }
            .generate(function_context),
            Expression::AttemptExpression(a) => {
                MoveAttemptExpression { expression: a }.generate(function_context)
            }
            Expression::Literal(l) => {
                MoveIRExpression::Literal(MoveLiteralToken { token: l }.generate())
            }
            //TODO: fix vector type
            Expression::ArrayLiteral(a) => {
                let elements = a
                    .elements
                    .into_iter()
                    .map(|e| {
                        MoveExpression {
                            expression: e,
                            position: Default::default(),
                        }
                        .generate(function_context)
                    })
                    .collect();

                MoveIRExpression::Vector(MoveIRVector {
                    elements,
                    vec_type: None,
                })
            }
            Expression::DictionaryLiteral(_) => unimplemented!(),
            Expression::SelfExpression => MoveSelf {
                token: Identifier::SELF.to_string(),
                position: self.position.clone(),
            }
            .generate(function_context, false),
            Expression::SubscriptExpression(s) => MoveSubscriptExpression {
                expression: s,
                position: self.position.clone(),
                rhs: None,
            }
            .generate(function_context),
            Expression::RangeExpression(_) => unimplemented!(),
            Expression::RawAssembly(s, _) => MoveIRExpression::Inline(s),
            Expression::CastExpression(c) => {
                MoveCastExpression { expression: c }.generate(function_context)
            }
            Expression::Sequence(_) => unimplemented!(),
        }
    }
}

struct MoveCastExpression {
    pub expression: CastExpression,
}

impl MoveCastExpression {
    pub fn generate(&self, function_context: &FunctionContext) -> MoveIRExpression {
        let enclosing = self.expression.expression.enclosing_type();
        let enclosing = enclosing
            .as_ref()
            .unwrap_or_else(|| &function_context.enclosing_type);
        let original_type = function_context.environment.get_expression_type(
            &*self.expression.expression,
            enclosing,
            &[],
            &[],
            &function_context.scope_context,
        );

        let original_type_information = MoveCastExpression::get_type_info(&original_type);
        let target_type_information = MoveCastExpression::get_type_info(&self.expression.cast_type);

        let expression_code = MoveExpression {
            expression: (*self.expression.expression).clone(),
            position: Default::default(),
        }
        .generate(&function_context);

        if original_type_information.0 <= target_type_information.0 {
            return expression_code;
        }

        let target = MoveCastExpression::maximum_value(target_type_information.0);

        MoveRuntimeFunction::revert_if_greater(expression_code, MoveIRExpression::Inline(target))
    }

    pub fn get_type_info(input: &Type) -> (u64, bool) {
        match input {
            Type::Bool => (256, false),
            Type::Int => (256, true),
            Type::String => (256, false),
            Type::Address => (256, false),
            _ => (256, false),
        }
    }

    pub fn maximum_value(input: u64) -> String {
        match input {
            8 => "255".to_string(),
            16 => "65535".to_string(),
            24 => "16777215".to_string(),
            32 => "4294967295".to_string(),
            40 => "1099511627775".to_string(),
            48 => "281474976710655".to_string(),
            56 => "72057594037927935".to_string(),
            64 => "18446744073709551615".to_string(),
            _ => unimplemented!(),
        }
    }
}

pub(crate) struct MoveSubscriptExpression {
    pub expression: SubscriptExpression,
    pub position: MovePosition,
    pub rhs: Option<MoveIRExpression>,
}

impl MoveSubscriptExpression {
    pub fn generate(&self, function_context: &FunctionContext) -> MoveIRExpression {
        let rhs = self.rhs.clone();
        let rhs =
            rhs.unwrap_or_else(|| MoveIRExpression::Literal(MoveIRLiteral::Hex("0x0".to_string())));

        let index = &self.expression.index_expression;
        let index = MoveExpression {
            expression: *index.clone(),
            position: Default::default(),
        }
        .generate(function_context);

        let identifier = self.expression.base_expression.clone();

        let identifier_code = MoveIdentifier {
            identifier,
            position: self.position.clone(),
        }
        .generate(function_context, false, false);
        let base_type = function_context.environment.get_expression_type(
            &Expression::Identifier(self.expression.base_expression.clone()),
            &function_context.enclosing_type.clone(),
            &[],
            &[],
            &function_context.scope_context,
        );

        let inner_type = match base_type.clone() {
            Type::ArrayType(a) => *a.key_type,
            Type::DictionaryType(d) => *d.key_type,
            _ => unimplemented!(),
        };

        let move_type = MoveType::move_type(
            inner_type,
            Option::from(function_context.environment.clone()),
        );
        let _move_type = move_type.generate(function_context);

        if let MovePosition::Left = self.position.clone() {
            return match base_type {
                Type::ArrayType(_) => {
                    MoveRuntimeFunction::append_to_array_int(identifier_code, rhs)
                }
                Type::FixedSizedArrayType(_) => panic!("Fixed Size Arrays not currently supported"),
                Type::DictionaryType(_) => {
                    let f_name = format!(
                        "Self._insert_{}",
                        mangle_dictionary(&self.expression.base_expression.token)
                    );
                    MoveIRExpression::FunctionCall(MoveIRFunctionCall {
                        identifier: f_name,
                        arguments: vec![index, rhs],
                    })
                }
                _ => panic!("Invalid Type for Subscript Expression"),
            };
        }

        match base_type {
            Type::ArrayType(_) => {
                let identifier = self.expression.base_expression.clone();

                let identifier_code = MoveIdentifier {
                    identifier,
                    position: self.position.clone(),
                }
                .generate(function_context, false, true);
                MoveRuntimeFunction::get_from_array_int(identifier_code, index)
            }
            Type::FixedSizedArrayType(_) => panic!("Fixed Size Arrays not currently supported"),
            Type::DictionaryType(_) => {
                let f_name = format!(
                    "Self._get_{}",
                    mangle_dictionary(&self.expression.base_expression.token)
                );
                MoveIRExpression::FunctionCall(MoveIRFunctionCall {
                    identifier: f_name,
                    arguments: vec![index],
                })
            }
            _ => panic!("Invalid Type for Subscript Expression"),
        }
    }
}

struct MoveAttemptExpression {
    pub expression: AttemptExpression,
}

impl MoveAttemptExpression {
    pub fn generate(&self, function_context: &FunctionContext) -> MoveIRExpression {
        let _function_call = self.expression.function_call.clone();
        let identifier =
            "QuartzWrapper".to_owned() + &self.expression.function_call.identifier.token.clone();
        let arguments: Vec<MoveIRExpression> = self
            .expression
            .function_call
            .arguments
            .clone()
            .into_iter()
            .map(|a| {
                MoveExpression {
                    expression: a.expression,
                    position: Default::default(),
                }
                .generate(function_context)
            })
            .collect();
        MoveIRExpression::FunctionCall(MoveIRFunctionCall {
            identifier,
            arguments,
        })
    }
}

struct MoveInoutExpression {
    pub expression: InoutExpression,
    pub position: MovePosition,
}

impl MoveInoutExpression {
    pub fn generate(&self, function_context: &FunctionContext) -> MoveIRExpression {
        let expression_type = function_context.environment.get_expression_type(
            &*self.expression.expression.clone(),
            &function_context.enclosing_type,
            &[],
            &[],
            &function_context.scope_context,
        );

        if let Type::InoutType(_) = expression_type {
            return MoveExpression {
                expression: *self.expression.expression.clone(),
                position: self.position.clone(),
            }
            .generate(&function_context);
        }

        if let MovePosition::Accessed = self.position {
        } else if let Expression::Identifier(i) = *self.expression.expression.clone() {
            if i.enclosing_type.is_none() {
                return MoveIRExpression::Operation(MoveIROperation::MutableReference(Box::from(
                    MoveExpression {
                        expression: *self.expression.expression.clone(),
                        position: MovePosition::Left,
                    }
                    .generate(&function_context),
                )));
            }
        }

        if let Expression::SelfExpression = *self.expression.expression.clone() {
            return MoveExpression {
                expression: *self.expression.expression.clone(),
                position: self.position.clone(),
            }
            .generate(&function_context);
        }

        let expression = self.expression.clone();
        MoveIRExpression::Operation(MoveIROperation::MutableReference(Box::from(
            MoveExpression {
                expression: *expression.expression,
                position: MovePosition::Inout,
            }
            .generate(&function_context),
        )))
    }
}

struct MoveBinaryExpression {
    pub expression: BinaryExpression,
    pub position: MovePosition,
}

impl MoveBinaryExpression {
    pub fn generate(&self, function_context: &FunctionContext) -> MoveIRExpression {
        if let BinOp::Dot = self.expression.op {
            if let Expression::FunctionCall(f) = *self.expression.rhs_expression.clone() {
                return MoveFunctionCall {
                    function_call: f,
                    module_name: "Self".to_string(),
                }
                .generate(function_context);
            }
            return MovePropertyAccess {
                left: *self.expression.lhs_expression.clone(),
                right: *self.expression.rhs_expression.clone(),
                position: self.position.clone(),
            }
            .generate(function_context, false);
        }

        if let BinOp::Equal = self.expression.op {
            return MoveAssignment {
                lhs: *self.expression.lhs_expression.clone(),
                rhs: *self.expression.rhs_expression.clone(),
            }
            .generate(function_context);
        }

        let lhs = MoveExpression {
            expression: *self.expression.lhs_expression.clone(),
            position: self.position.clone(),
        }
        .generate(function_context);

        let rhs: MoveIRExpression;

        let (is_signer, id) = is_signer_type(&*self.expression.rhs_expression, function_context);

        if is_signer {
            rhs = MoveIRExpression::Inline(format!("Signer.address_of(copy({}))", id));
        } else {
            rhs = MoveExpression {
                expression: *self.expression.rhs_expression.clone(),
                position: self.position.clone(),
            }
            .generate(function_context);
        }

        match self.expression.op.clone() {
            BinOp::Plus => {
                MoveIRExpression::Operation(MoveIROperation::Add(Box::from(lhs), Box::from(rhs)))
            }
            BinOp::GreaterThan => MoveIRExpression::Operation(MoveIROperation::GreaterThan(
                Box::from(lhs),
                Box::from(rhs),
            )),

            BinOp::OverflowingPlus => {
                MoveIRExpression::Operation(MoveIROperation::Add(Box::from(lhs), Box::from(rhs)))
            }
            BinOp::Minus => {
                MoveIRExpression::Operation(MoveIROperation::Minus(Box::from(lhs), Box::from(rhs)))
            }

            BinOp::OverflowingMinus => {
                MoveIRExpression::Operation(MoveIROperation::Minus(Box::from(lhs), Box::from(rhs)))
            }
            BinOp::Times => {
                MoveIRExpression::Operation(MoveIROperation::Times(Box::from(lhs), Box::from(rhs)))
            }
            BinOp::OverflowingTimes => {
                MoveIRExpression::Operation(MoveIROperation::Times(Box::from(lhs), Box::from(rhs)))
            }

            BinOp::Power => {
                MoveRuntimeFunction::power(lhs, rhs)
            }
            BinOp::Divide => {
                MoveIRExpression::Operation(MoveIROperation::Divide(Box::from(lhs), Box::from(rhs)))
            }
            BinOp::Percent => {
                MoveIRExpression::Operation(MoveIROperation::Modulo(Box::from(lhs), Box::from(rhs)))
            }
            BinOp::Dot => panic!("operator not supported"),
            BinOp::Equal => {
                MoveIRExpression::Operation(MoveIROperation::Equal(Box::from(lhs), Box::from(rhs)))
            }
            BinOp::PlusEqual => panic!("operator not supported"),
            BinOp::MinusEqual => panic!("operator not supported"),
            BinOp::TimesEqual => panic!("operator not supported"),
            BinOp::DivideEqual => panic!("operator not supported"),
            BinOp::DoubleEqual => {
                MoveIRExpression::Operation(MoveIROperation::Equal(Box::from(lhs), Box::from(rhs)))
            }
            BinOp::NotEqual => MoveIRExpression::Operation(MoveIROperation::NotEqual(
                Box::from(lhs),
                Box::from(rhs),
            )),
            BinOp::LessThan => MoveIRExpression::Operation(MoveIROperation::LessThan(
                Box::from(lhs),
                Box::from(rhs),
            )),
            BinOp::LessThanOrEqual => MoveIRExpression::Operation(MoveIROperation::LessThanEqual(
                Box::from(lhs),
                Box::from(rhs),
            )),
            BinOp::GreaterThanOrEqual => MoveIRExpression::Operation(
                MoveIROperation::GreaterThanEqual(Box::from(lhs), Box::from(rhs)),
            ),
            BinOp::Or => {
                MoveIRExpression::Operation(MoveIROperation::Or(Box::from(lhs), Box::from(rhs)))
            }
            BinOp::And => {
                MoveIRExpression::Operation(MoveIROperation::And(Box::from(lhs), Box::from(rhs)))
            }
        }
    }
}

pub fn is_signer_type(
    expression: &Expression,
    function_context: &FunctionContext,
) -> (bool, String) {
    if let Expression::Identifier(id) = expression {
        if let Some(identifier_type) = function_context.scope_context.type_for(&id.token) {
            return (
                identifier_type
                    == Type::UserDefinedType(Identifier {
                        token: "&signer".to_string(),
                        enclosing_type: None,
                        line_info: Default::default(),
                    }),
                id.token.clone(),
            );
        }
    }
    (false, String::from(""))
}
