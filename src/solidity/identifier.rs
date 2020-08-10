use super::*;

pub struct SolidityIdentifier {
    pub identifier: Identifier,
    pub is_lvalue: bool,
}

impl SolidityIdentifier {
    pub fn generate(&self, function_context: &mut FunctionContext) -> YulExpression {
        if self.identifier.enclosing_type.is_some() {
            //REMOVEBEFOREFLIGHT
            return SolidityPropertyAccess {
                lhs: Expression::SelfExpression,
                rhs: Expression::Identifier(self.identifier.clone()),
                is_left: self.is_lvalue,
            }
            .generate(function_context);
        }

        YulExpression::Identifier(mangle(&self.identifier.token))
    }
}
