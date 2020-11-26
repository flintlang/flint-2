use crate::ast::{
    ArrayLiteral, ArrayType, AttemptExpression, BinOp, BinaryExpression, CallerProtection,
    Expression, FunctionCall, Identifier, InoutType, Literal, RangeExpression, RangeType, Type,
    TypeState,
};
use crate::context::ScopeContext;
use crate::environment::*;
use crate::type_checker::ExpressionChecker;

impl ExpressionChecker for Environment {
    fn get_expression_type(
        &self,
        expression: &Expression,
        type_id: &str,
        type_states: &[TypeState],
        caller_protections: &[CallerProtection],
        scope: &ScopeContext,
    ) -> Type {
        match expression {
            Expression::Identifier(i) => {
                if i.enclosing_type.is_none() {
                    if let Some(result_type) = scope.type_for(&i.token) {
                        return if let Type::InoutType(inout) = result_type {
                            *inout.key_type
                        } else {
                            result_type
                        };
                    }
                }

                let enclosing_type = i.enclosing_type.as_deref().unwrap_or(type_id);

                self.get_property_type(&i.token, enclosing_type, scope)
            }
            Expression::BinaryExpression(b) => {
                self.get_binary_expression_type(b, type_id, type_states, caller_protections, scope)
            }
            Expression::InoutExpression(e) => {
                let key_type = self.get_expression_type(
                    &*e.expression,
                    type_id,
                    type_states,
                    caller_protections,
                    scope,
                );

                Type::InoutType(InoutType {
                    key_type: Box::from(key_type),
                })
            }
            Expression::ExternalCall(e) => self.get_expression_type(
                &Expression::BinaryExpression(e.function_call.clone()),
                type_id,
                type_states,
                caller_protections,
                scope,
            ),
            Expression::FunctionCall(f) => {
                let enclosing_type = if f.identifier.enclosing_type.is_some() {
                    let enclosing = f.identifier.enclosing_type.as_ref();
                    enclosing.unwrap()
                } else {
                    type_id
                };

                self.get_function_call_type(f, enclosing_type, caller_protections, scope)
            }
            Expression::VariableDeclaration(v) => v.variable_type.clone(),
            Expression::BracketedExpression(e) => self.get_expression_type(
                &*e.expression,
                type_id,
                type_states,
                caller_protections,
                scope,
            ),
            Expression::AttemptExpression(a) => {
                self.get_attempt_expression_type(a, type_id, type_states, caller_protections, scope)
            }
            Expression::Literal(l) => self.get_literal_type(l),
            Expression::ArrayLiteral(a) => {
                self.get_array_literal_type(a, type_id, type_states, caller_protections, scope)
            }
            Expression::DictionaryLiteral(d) => {
                self.get_dictionary_literal_type(d, type_id, type_states, caller_protections, scope)
            }
            Expression::SelfExpression => Type::UserDefinedType(Identifier {
                token: type_id.to_string(),
                enclosing_type: None,
                line_info: Default::default(),
            }),
            Expression::SubscriptExpression(s) => {
                //    Get Identifier Type
                let identifer_type = self.get_expression_type(
                    &Expression::Identifier(s.base_expression.clone()),
                    type_id,
                    &[],
                    &[],
                    scope,
                );

                match identifer_type {
                    Type::ArrayType(a) => *a.key_type,
                    Type::FixedSizedArrayType(a) => *a.key_type,
                    Type::DictionaryType(d) => *d.key_type,
                    _ => Type::Error,
                }
            }
            Expression::RangeExpression(r) => {
                self.get_range_type(r, type_id, type_states, caller_protections, scope)
            }
            Expression::RawAssembly(_, _) => unimplemented!(),
            Expression::CastExpression(c) => c.cast_type.clone(),
            Expression::Sequence(_) => unimplemented!(),
        }
    }
}

impl Environment {
    pub fn get_property_type(&self, name: &str, type_id: &str, scope: &ScopeContext) -> Type {
        self.types
            .get(type_id)
            .and_then(|enclosing| enclosing.properties.get(name))
            .map(|info| info.property.get_type())
            .unwrap_or_else(|| {
                if scope.type_for(&name).is_some() {
                    unimplemented!()
                }
                Type::Error
            })
    }

    pub fn get_literal_type(&self, literal: &Literal) -> Type {
        match literal {
            Literal::U8Literal(_) => Type::TypeState,
            Literal::BooleanLiteral(_) => Type::Bool,
            Literal::AddressLiteral(_) => Type::Address,
            Literal::StringLiteral(_) => Type::String,
            Literal::IntLiteral(_) => Type::Int,
            Literal::FloatLiteral(_) => Type::Int,
        }
    }

    pub fn get_attempt_expression_type(
        &self,
        expression: &AttemptExpression,
        type_id: &str,
        type_states: &[TypeState],
        caller_protections: &[CallerProtection],
        scope: &ScopeContext,
    ) -> Type {
        if expression.is_soft() {
            return Type::Bool;
        }

        let function_call = &expression.function_call;
        let enclosing_type = expression
            .function_call
            .identifier
            .enclosing_type
            .as_deref()
            .unwrap_or(type_id);

        self.get_expression_type(
            &Expression::FunctionCall(function_call.clone()),
            enclosing_type,
            type_states,
            caller_protections,
            scope,
        )
    }

    pub fn get_range_type(
        &self,
        expression: &RangeExpression,
        type_id: &str,
        type_states: &[TypeState],
        caller_protections: &[CallerProtection],
        scope: &ScopeContext,
    ) -> Type {
        let element_type = self.get_expression_type(
            &*expression.start_expression,
            type_id,
            type_states,
            caller_protections,
            scope,
        );
        let bound_type = self.get_expression_type(
            &*expression.end_expression,
            type_id,
            type_states,
            caller_protections,
            scope,
        );
        if element_type != bound_type {
            return Type::Error;
        }

        Type::RangeType(RangeType {
            key_type: Box::new(element_type),
        })
    }

    pub fn get_binary_expression_type(
        &self,
        binary: &BinaryExpression,
        type_id: &str,
        type_states: &[TypeState],
        caller_protections: &[CallerProtection],
        scope: &ScopeContext,
    ) -> Type {
        if binary.op.is_boolean() {
            return Type::Bool;
        }

        if let BinOp::Dot = binary.op {
            let lhs_type = self.get_expression_type(
                &*binary.lhs_expression,
                type_id,
                type_states,
                caller_protections,
                scope,
            );
            match lhs_type {
                Type::ArrayType(_) => {
                    if let Expression::Identifier(i) = &*binary.rhs_expression {
                        if i.token == "size" {
                            return Type::Int;
                        }
                    }
                    println!("Arrays only have property 'size'");
                    return Type::Error;
                }
                Type::FixedSizedArrayType(_) => {
                    if let Expression::Identifier(i) = &*binary.rhs_expression {
                        if i.token == "size" {
                            return Type::Int;
                        }
                    }
                    println!("Arrays only have property 'size'");
                    return Type::Error;
                }
                Type::DictionaryType(d) => {
                    if let Expression::Identifier(i) = &*binary.rhs_expression {
                        if i.token == "size" {
                            return Type::Int;
                        } else if i.token == "keys" {
                            return Type::ArrayType(ArrayType {
                                key_type: d.key_type,
                            });
                        }
                    }
                    println!("Dictionaries only have properties size and keys");
                    return Type::Error;
                }
                _ => {}
            };
            self.get_expression_type(
                &*binary.rhs_expression,
                &lhs_type.name(),
                type_states,
                caller_protections,
                scope,
            )
        } else {
            self.get_expression_type(
                &*binary.rhs_expression,
                type_id,
                type_states,
                caller_protections,
                scope,
            )
        }
    }

    pub fn get_array_literal_type(
        &self,
        array: &ArrayLiteral,
        type_id: &str,
        type_states: &[TypeState],
        caller_protections: &[CallerProtection],
        scope: &ScopeContext,
    ) -> Type {
        let mut element_type: Option<Type> = None;

        for elements in &array.elements {
            let elements_type =
                self.get_expression_type(elements, type_id, type_states, caller_protections, scope);

            if let Some(ref comparison_type) = element_type {
                if comparison_type != &elements_type {
                    return Type::Error;
                }
            } else {
                element_type = Some(elements_type)
            }
        }
        let result_type = element_type.unwrap_or(Type::Error);
        Type::ArrayType(ArrayType {
            key_type: Box::new(result_type),
        })
    }

    pub fn get_dictionary_literal_type(
        &self,
        dictionary: &DictionaryLiteral,
        type_id: &str,
        type_states: &[TypeState],
        caller_protections: &[CallerProtection],
        scope: &ScopeContext,
    ) -> Type {
        let mut keys_type: Option<Type> = None;
        let mut values_type: Option<Type> = None;

        for (key, value) in &dictionary.elements {
            let key_type =
                self.get_expression_type(&key, type_id, type_states, caller_protections, scope);
            let value_type =
                self.get_expression_type(&value, type_id, type_states, caller_protections, scope);

            if let Some(ref comparison_type) = keys_type {
                if comparison_type != &key_type {
                    return Type::Error;
                }
            } else {
                keys_type = Some(key_type)
            }

            if let Some(ref comparison_type) = values_type {
                if comparison_type != &value_type {
                    return Type::Error;
                }
            } else {
                values_type = Some(value_type)
            }
        }

        let result_key_type = keys_type.unwrap_or(Type::Error);
        let result_value_type = values_type.unwrap_or(Type::Error);
        Type::DictionaryType(DictionaryType {
            key_type: Box::new(result_key_type),
            value_type: Box::new(result_value_type),
        })
    }

    pub fn get_function_call_type(
        &self,
        call: &FunctionCall,
        type_id: &str,
        caller_protections: &[CallerProtection],
        scope: &ScopeContext,
    ) -> Type {
        let identifier = call.identifier.clone();
        let function_call = self.match_function_call(call, type_id, caller_protections, scope);
        match function_call {
            FunctionCallMatchResult::MatchedFunction(m) => {
                m.get_result_type().cloned().unwrap_or(Type::Error)
            }
            FunctionCallMatchResult::MatchedFunctionWithoutCaller(m) => {
                if m.candidates.len() == 1 {
                    let first = m.candidates.first();
                    let first = first.unwrap();
                    return if let CallableInformation::FunctionInformation(fi) = first {
                        fi.get_result_type().cloned().unwrap_or(Type::Error)
                    } else {
                        Type::Error
                    };
                }
                Type::Error
            }
            FunctionCallMatchResult::MatchedInitializer(_) => Type::UserDefinedType(identifier),
            FunctionCallMatchResult::MatchedGlobalFunction(info) => {
                info.get_result_type().unwrap_or(&Type::Error).clone()
            }
            _ => Type::Error,
        }
    }
}
