use super::*;

pub struct SolidityLiteral {
    pub literal: Literal,
}

impl SolidityLiteral {
    pub fn generate(&self) -> YulLiteral {
        match self.literal.clone() {
            Literal::BooleanLiteral(b) => YulLiteral::Bool(b),
            Literal::AddressLiteral(a) => YulLiteral::Hex(a),
            Literal::StringLiteral(s) => YulLiteral::String(s),
            Literal::IntLiteral(i) => YulLiteral::Num(i),
            Literal::FloatLiteral(_) => panic!("Float Literal Currently Unsupported"),
        }
    }
}