use super::ir::MoveIRLiteral;
use crate::ast::Literal;

impl MoveLiteralToken {
    pub fn generate(&self) -> MoveIRLiteral {
        match self.token.clone() {
            Literal::BooleanLiteral(b) => MoveIRLiteral::Bool(b),
            Literal::AddressLiteral(a) => MoveIRLiteral::Hex(a),
            Literal::StringLiteral(s) => MoveIRLiteral::String(s),
            Literal::IntLiteral(i) => MoveIRLiteral::Num(i),
            Literal::FloatLiteral(_) => panic!("Floats not currently supported"),
        }
    }
}

pub(crate) struct MoveLiteralToken {
    pub token: Literal,
}
