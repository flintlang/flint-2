use crate::TypeChecker::ExpressionCheck;

impl ExpressionCheck for Environment {
    fn get_expression_type(
        &self,
        expression: Expression,
        t: &TypeIdentifier,
        type_states: Vec<TypeState>,
        caller_protections: Vec<CallerProtection>,
        scope: ScopeContext,
        ) -> Type {
        match expression {
            Expression::Identifier(i) => {
                if i.enclosing_type.is_none() {
                    let result_type = scope.type_for(i.token.clone());
                    if result_type.is_some() {
                        let result_type = result_type.unwrap();
                        return if let Type::InoutType(inout) = result_type {
                            *inout.key_type
                        } else {
                            result_type
                        };
                    }
                }

                let enclosing_type = if i.enclosing_type.is_some() {
                    let enclosing = i.enclosing_type.as_ref();
                    enclosing.unwrap()
                } else {
                    t
                };

                self.get_property_type(i.token.clone(), enclosing_type, scope)
            }
            Expression::BinaryExpression(b) => {
                self.get_binary_expression_type(
                    b,
                    t,
                    type_states,
                    caller_protections,
                    scope,
                    )
            }
            Expression::InoutExpression(e) => {
                let key_type = self.get_expression_type(
                    *e.expression,
                    t,
                    type_states,
                    caller_protections,
                    scope,
                    );

                Type::InoutType(InoutType {
                    key_type: Box::from(key_type),
                })
            }
            Expression::ExternalCall(e) => {
                self.get_expression_type(
                    Expression::BinaryExpression(e.function_call),
                    t,
                    type_states,
                    caller_protections,
                    scope,
                    )
            }
            Expression::FunctionCall(f) => {
                let enclosing_type = if f.identifier.enclosing_type.is_some() {
                    let enclosing = f.identifier.enclosing_type.as_ref();
                    enclosing.unwrap()
                } else {
                    t
                };

                self.get_function_call_type(
                    f.clone(),
                    enclosing_type,
                    caller_protections,
                    scope,
                    )
            }
            Expression::VariableDeclaration(v) => v.variable_type,
            Expression::BracketedExpression(e) => {
                self.get_expression_type(
                    *e.expression,
                    t,
                    type_states,
                    caller_protections,
                    scope,
                    )
            }
            Expression::AttemptExpression(a) => {
                self.get_attempt_expression_type(
                    a,
                    t,
                    type_states,
                    caller_protections,
                    scope,
                    )
            }
            Expression::Literal(l) => {
                self.get_literal_type(l)
            }
            Expression::ArrayLiteral(a) => {
                self.get_array_literal_type(a, t, type_states, caller_protections, scope)
            }
            Expression::DictionaryLiteral(_) => unimplemented!(),
            Expression::SelfExpression => Type::UserDefinedType(Identifier {
                token: t.clone(),
                enclosing_type: None,
                line_info: Default::default(),
            }),
            Expression::SubscriptExpression(s) => {
                //    Get Identifier Type
                let identifer_type = self.get_expression_type(
                    Expression::Identifier(s.base_expression.clone()),
                    t,
                    vec![],
                    vec![],
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
                self.get_range_type(r, t, type_states, caller_protections, scope)
            }
            Expression::RawAssembly(_, _) => unimplemented!(),
            Expression::CastExpression(c) => c.cast_type,
            Expression::Sequence(_) => unimplemented!(),
        }
    }
}

impl Environment {
    pub fn get_property_type(&self, name: String, t: &TypeIdentifier, scope: ScopeContext) -> Type {
        let enclosing = self.types.get(t);
        // println!("{:?}", t);
        if enclosing.is_some() {
            let enclosing = enclosing.unwrap();
            // println!("{:?}", enclosing.clone());
            // println!("{:?}", name.clone());
            if enclosing.properties.get(name.as_str()).is_some() {
                return self
                    .types
                    .get(t)
                    .unwrap()
                    .properties
                    .get(name.as_str())
                    .unwrap()
                    .property
                    .get_type();
            }

            if enclosing.functions.get(name.as_str()).is_some() {
                unimplemented!()
            }
        }

        if scope.type_for(name.clone()).is_some() {
            unimplemented!()
        }
        Type::Error
    }

    fn get_literal_type(&self, literal: Literal) -> Type {
        match literal {
            Literal::BooleanLiteral(_) => Type::Bool,
            Literal::AddressLiteral(_) => Type::Address,
            Literal::StringLiteral(_) => Type::String,
            Literal::IntLiteral(_) => Type::Int,
            Literal::FloatLiteral(_) => Type::Int,
        }
    }

    fn get_attempt_expression_type(
        &self,
        expression: AttemptExpression,
        t: &TypeIdentifier,
        type_states: Vec<TypeState>,
        caller_protections: Vec<CallerProtection>,
        scope: ScopeContext,
    ) -> Type {
        if expression.is_soft() {
            return Type::Bool;
        }

        let function_call = expression.function_call.clone();

        let enclosing_type = if function_call.identifier.enclosing_type.is_some() {
            let enclosing = function_call.identifier.enclosing_type.clone();
            enclosing.unwrap()
        } else {
            t.clone()
        };

        self.get_expression_type(
            Expression::FunctionCall(function_call),
            &enclosing_type,
            type_states,
            caller_protections,
            scope,
        )
    }

    fn get_range_type(
        &self,
        expression: RangeExpression,
        t: &TypeIdentifier,
        type_states: Vec<TypeState>,
        caller_protections: Vec<CallerProtection>,
        scope: ScopeContext,
    ) -> Type {
        let element_type = self.get_expression_type(
            *expression.start_expression,
            t,
            type_states.clone(),
            caller_protections.clone(),
            scope.clone(),
        );
        let bound_type = self.get_expression_type(
            *expression.end_expression,
            t,
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

    fn get_binary_expression_type(
        &self,
        b: BinaryExpression,
        t: &TypeIdentifier,
        type_states: Vec<TypeState>,
        caller_protections: Vec<CallerProtection>,
        scope: ScopeContext,
    ) -> Type {
        if b.op.is_boolean() {
            return Type::Bool;
        }

        if let BinOp::Dot = b.op {
            let lhs_type = self.get_expression_type(
                *b.lhs_expression,
                t,
                type_states.clone(),
                caller_protections.clone(),
                scope.clone(),
            );
            match lhs_type {
                Type::ArrayType(_) => {
                    if let Expression::Identifier(i) = *b.rhs_expression {
                        if i.token == "size" {
                            return Type::Int;
                        }
                    }
                    println!("Arrays only have property 'size'");
                    return Type::Error;
                }
                Type::FixedSizedArrayType(_) => {
                    if let Expression::Identifier(i) = *b.rhs_expression {
                        if i.token == "size" {
                            return Type::Int;
                        }
                    }
                    println!("Arrays only have property 'size'");
                    return Type::Error;
                }
                Type::DictionaryType(d) => {
                    if let Expression::Identifier(i) = *b.rhs_expression {
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
            let rhs_type = self.get_expression_type(
                *b.rhs_expression,
                &lhs_type.name(),
                type_states.clone(),
                caller_protections.clone(),
                scope.clone(),
            );
            return rhs_type;
        }

        self.get_expression_type(*b.rhs_expression, t, type_states, caller_protections, scope)
    }

    fn get_array_literal_type(
        &self,
        a: ArrayLiteral,
        t: &TypeIdentifier,
        type_states: Vec<TypeState>,
        caller_protections: Vec<CallerProtection>,
        scope: ScopeContext,
    ) -> Type {
        let mut element_type: Option<Type> = None;

        for elements in a.elements {
            let elements_type = self.get_expression_type(
                elements.clone(),
                t,
                type_states.clone(),
                caller_protections.clone(),
                scope.clone(),
            );

            if element_type.is_some() {
                let comparison_type = element_type.clone();
                let comparison_type = comparison_type.unwrap();
                if comparison_type != elements_type {
                    return Type::Error;
                }
            }
            if element_type.is_none() {
                element_type = Some(elements_type)
            }
        }
        let result_type = if element_type.is_some() {
            element_type.unwrap()
        } else {
            //TODO change to Type::Any
            Type::Error
        };
        Type::ArrayType(ArrayType {
            key_type: Box::new(result_type),
        })
    }

    fn get_function_call_type(
        &self,
        f: FunctionCall,
        t: &TypeIdentifier,
        caller_protections: Vec<CallerProtection>,
        scope: ScopeContext,
    ) -> Type {
        let identifier = f.identifier.clone();
        let function_call = self.match_function_call(f, t, caller_protections, scope);
        match function_call {
            FunctionCallMatchResult::MatchedFunction(m) => {
                m.get_result_type().unwrap_or(Type::Error)
            }
            FunctionCallMatchResult::MatchedFunctionWithoutCaller(m) => {
                if m.candidates.len() == 1 {
                    let first = m.candidates.first();
                    let first = first.unwrap();
                    return if let CallableInformation::FunctionInformation(fi) = first {
                        fi.get_result_type().unwrap_or(Type::Error)
                    } else {
                        Type::Error
                    };
                }
                Type::Error
            }
            FunctionCallMatchResult::MatchedInitializer(_) => Type::UserDefinedType(identifier),
            _ => Type::Error,
        }
    }
}


