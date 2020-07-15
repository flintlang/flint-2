use crate::ast::{Expression, mangle, Type};
use super::function::FunctionContext;
use super::ir::{MoveIRExpression, MoveIRAssignment};
use super::MovePosition;
use super::r#type::MoveType;
use crate::type_checker::ExpressionCheck;
use super::identifier::MoveIdentifier;
use super::expression::{MoveExpression, MoveSubscriptExpression};

#[derive(Debug)]
pub(crate) struct MoveAssignment {
    pub lhs: Expression,
    pub rhs: Expression,
}

impl MoveAssignment {
    pub fn generate(&self, function_context: &FunctionContext) -> MoveIRExpression {
        let lhs = self.lhs.clone();
        if let Expression::Identifier(i) = &lhs {
            if let Some(ref enclosing) = i.enclosing_type {
                let var_type = function_context.environment.get_expression_type(
                    lhs.clone(),
                    enclosing,
                    vec![],
                    vec![],
                    function_context.scope_context.clone(),
                );
                if let Type::ArrayType(a) = var_type {
                    let lhs_ir = MoveExpression {
                        expression: self.lhs.clone(),
                        position: MovePosition::Left,
                    }
                        .generate(function_context);

                    if let Expression::ArrayLiteral(_) = self.rhs {
                        let rhs_ir = MoveExpression {
                            expression: self.rhs.clone(),
                            position: Default::default(),
                        }
                            .generate(function_context);

                        if let MoveIRExpression::Vector(mut vector) = rhs_ir {
                            let vec_type = MoveType::move_type(
                                *a.key_type,
                                Option::from(function_context.environment.clone()),
                            )
                                .generate(function_context);
                            vector.vec_type = Option::from(vec_type);
                            let rhs_ir = MoveIRExpression::Vector(vector);
                            return MoveIRExpression::Assignment(MoveIRAssignment {
                                identifier: format!("{lhs}", lhs = lhs_ir),
                                expression: Box::new(rhs_ir),
                            });
                        }
                    } else {
                        panic!("Wrong type");
                    }
                }
            }
        }

        let rhs_ir = MoveExpression {
            expression: self.rhs.clone(),
            position: Default::default(),
        }
            .generate(function_context);

        if let Expression::VariableDeclaration(_) = lhs {
            unimplemented!()
        }

        if let Expression::Identifier(ref i) = lhs {
            if i.enclosing_type.is_none() {
                return MoveIRExpression::Assignment(MoveIRAssignment {
                    identifier: mangle(i.token.clone()),
                    expression: Box::new(rhs_ir),
                });
            }
        }

        if let Expression::SubscriptExpression(s) = lhs {
            return MoveSubscriptExpression {
                expression: s,
                position: MovePosition::Left,
                rhs: Option::from(rhs_ir),
            }
                .generate(function_context);
        }

        if let Expression::RawAssembly(s, _) = lhs {
            if s == "_" {
                if let Expression::Identifier(i) = &self.rhs {
                    return MoveIRExpression::Assignment(MoveIRAssignment {
                        identifier: "_".to_string(),
                        expression: Box::new(
                            MoveIdentifier {
                                identifier: i.clone(),
                                position: Default::default(),
                            }
                                .generate(function_context, true, false),
                        ),
                    });
                }
            }
        }

        let lhs_ir = MoveExpression {
            expression: self.lhs.clone(),
            position: MovePosition::Left,
        }
            .generate(function_context);

        if function_context.in_struct_function {
            return MoveIRExpression::Assignment(MoveIRAssignment {
                identifier: format!("{lhs}", lhs = lhs_ir),
                expression: Box::new(rhs_ir),
            });
        } else if self.lhs.enclosing_identifier().is_some()
            && function_context
            .scope_context
            .contains_variable_declaration(self.lhs.enclosing_identifier().unwrap().token)
        {
            return MoveIRExpression::Assignment(MoveIRAssignment {
                identifier: self.lhs.enclosing_identifier().unwrap().token,
                expression: Box::new(rhs_ir),
            });
        }

        MoveIRExpression::Assignment(MoveIRAssignment {
            identifier: format!("{lhs}", lhs = lhs_ir),
            expression: Box::new(rhs_ir),
        })
    }
}