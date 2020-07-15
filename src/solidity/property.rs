use super::*;

pub struct SolidityPropertyAccess {
    pub lhs: Expression,
    pub rhs: Expression,
    pub is_left: bool,
}

impl SolidityPropertyAccess {
    pub fn generate(&self, function_context: &mut FunctionContext) -> YulExpression {
        let type_identifier = function_context.enclosing_type.clone();
        let scope = function_context.scope_context.clone();
        let is_mem_access = false;
        let lhs_type = function_context.environment.get_expression_type(
            self.lhs.clone(),
            &type_identifier,
            vec![],
            vec![],
            scope.clone(),
        );
        if let Expression::Identifier(li) = self.lhs.clone() {
            if let Expression::Identifier(_) = self.rhs.clone() {
                if function_context.environment.is_enum_declared(&li.token) {
                    unimplemented!()
                }
            }
        }

        let rhs_offset = match lhs_type {
            Type::ArrayType(_) => {
                if let Expression::Identifier(i) = self.rhs.clone() {
                    if i.token == "size".to_string() {
                        YulExpression::Literal(YulLiteral::Num(0))
                    } else {
                        panic!("Unsupported identifier on array")
                    }
                } else {
                    panic!("Unsupported identifier on array")
                }
            }
            Type::FixedSizedArrayType(_) => {
                if let Expression::Identifier(i) = self.rhs.clone() {
                    if i.token == "size".to_string() {
                        YulExpression::Literal(YulLiteral::Num(0))
                    } else {
                        panic!("Unsupported identifier on array")
                    }
                } else {
                    panic!("Unsupported identifier on array")
                }
            }
            Type::DictionaryType(_) => {
                if let Expression::Identifier(i) = self.rhs.clone() {
                    if i.token == "size".to_string() {
                        YulExpression::Literal(YulLiteral::Num(0))
                    } else {
                        panic!("Unsupported identifier on dictionary")
                    }
                } else {
                    panic!("Unsupported identifier on dictionary")
                }
            }
            _ => SolidityPropertyOffset {
                expression: self.rhs.clone(),
                enclosing_type: lhs_type,
            }
            .generate(function_context),
        };

        let offset = if function_context.in_struct_function {
            let enclosing_parameter = function_context
                .scope_context
                .enclosing_parameter(self.lhs.clone(), &type_identifier);
            let enclosing_name = if enclosing_parameter.is_some() {
                enclosing_parameter.unwrap()
            } else {
                "QuartzSelf".to_string()
            };

            let lhs_offset = YulExpression::Identifier(mangle(enclosing_name.clone()));
            SolidityRuntimeFunction::add_offset(
                lhs_offset,
                rhs_offset,
                mangle(mangle_mem(enclosing_name)),
            )
        } else {
            let lhs_offset = if let Expression::Identifier(i) = self.lhs.clone() {
                if i.enclosing_type.is_some() {
                    let enclosing_type = i.enclosing_type.clone();
                    let enclosing_type = enclosing_type.unwrap();
                    let offset = function_context
                        .environment
                        .property_offset(i.token.clone(), &enclosing_type);
                    YulExpression::Literal(YulLiteral::Num(offset))
                } else {
                    unimplemented!()
                }
            } else {
                SolidityExpression {
                    expression: self.lhs.clone(),
                    is_lvalue: true,
                }
                .generate(function_context)
            };

            SolidityRuntimeFunction::add_offset_bool(lhs_offset, rhs_offset, is_mem_access)
        };

        if self.is_left {
            return offset;
        }

        if function_context.in_struct_function && !is_mem_access {
            let lhs_enclosing = if self.lhs.enclosing_identifier().is_some() {
                let ident = self.lhs.enclosing_identifier().clone();
                let ident = ident.unwrap();
                mangle(ident.token)
            } else {
                mangle("QuartzSelf".to_string())
            };

            return SolidityRuntimeFunction::load(offset, mangle_mem(lhs_enclosing));
        }

        SolidityRuntimeFunction::load_bool(offset, is_mem_access)
    }
}

#[derive(Debug)]
pub struct SolidityPropertyOffset {
    pub expression: Expression,
    pub enclosing_type: Type,
}

impl SolidityPropertyOffset {
    pub fn generate(&self, function_context: &mut FunctionContext) -> YulExpression {
        if let Expression::BinaryExpression(b) = self.expression.clone() {
            return SolidityPropertyAccess {
                lhs: *b.lhs_expression,
                rhs: *b.rhs_expression,
                is_left: true,
            }
            .generate(function_context);
        } else if let Expression::SubscriptExpression(s) = self.expression.clone() {
            return SoliditySubscriptExpression {
                expression: s.clone(),
                is_lvalue: true,
            }
            .generate(function_context);
        }

        if let Expression::Identifier(i) = self.expression.clone() {
            if let Type::UserDefinedType(t) = self.enclosing_type.clone() {
                let offset = function_context
                    .environment
                    .property_offset(i.token.clone(), &t.token);
                return YulExpression::Literal(YulLiteral::Num(offset));
            }

            panic!("Fatal Error")
        }

        panic!("Fatal Error")
    }
}
