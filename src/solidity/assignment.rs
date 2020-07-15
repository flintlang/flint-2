use super::*;

pub struct SolidityAssignment {
    pub lhs: Expression,
    pub rhs: Expression,
}

impl SolidityAssignment {
    pub fn generate(&self, function_context: &mut FunctionContext) -> YulExpression {
        let rhs_code = SolidityExpression {
            expression: self.rhs.clone(),
            is_lvalue: false,
        }
        .generate(function_context);

        match self.lhs.clone() {
            Expression::VariableDeclaration(v) => {
                let mangle = mangle(&v.identifier.token);
                YulExpression::VariableDeclaration(YulVariableDeclaration {
                    declaration: mangle,
                    declaration_type: YulType::Any,
                    expression: Some(Box::from(rhs_code)),
                })
            }
            Expression::Identifier(i) if i.enclosing_type.is_none() => {
                YulExpression::Assignment(YulAssignment {
                    identifiers: vec![mangle(&i.token)],
                    expression: Box::from(rhs_code),
                })
            }
            _ => {
                println!("HERE we drop");
                let lhs_code = SolidityExpression {
                    expression: self.lhs.clone(),
                    is_lvalue: true,
                }
                .generate(function_context);

                if function_context.in_struct_function {
                    let enclosing_name = function_context
                        .scope_context
                        .enclosing_parameter(self.lhs.clone(), &function_context.enclosing_type);
                    let enclosing_name = if let Some(ref enclosing_name) = enclosing_name {
                        enclosing_name
                    } else { "QuartzSelf" };

                    return SolidityRuntimeFunction::store(
                        lhs_code,
                        rhs_code,
                        mangle(&mangle_mem(enclosing_name)),
                    );
                } else if let Some(enclosing) = self.lhs.enclosing_identifier() {
                    if function_context
                        .scope_context
                        .contains_variable_declaration(enclosing.token.clone())
                    {
                        return SolidityRuntimeFunction::store_bool(lhs_code, rhs_code, true);
                    }
                }
                SolidityRuntimeFunction::store_bool(lhs_code, rhs_code, false)
            }
        }
    }
}
