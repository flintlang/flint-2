use crate::AST::*;

#[derive(Clone, Debug, PartialEq)]
pub enum BinOp {
    Plus,
    OverflowingPlus,
    Minus,
    OverflowingMinus,
    Times,
    OverflowingTimes,
    Power,
    Divide,
    Percent,
    Dot,
    Equal,
    PlusEqual,
    MinusEqual,
    TimesEqual,
    DivideEqual,
    DoubleEqual,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Or,
    And,
    Implies,
}

impl BinOp {
    pub fn is_left(&self) -> bool {
        match self {
            BinOp::Plus => true,
            BinOp::Times => true,
            BinOp::Dot => true,
            _ => false,
        }
    }
    pub fn is_boolean(&self) -> bool {
        match self {
            BinOp::DoubleEqual => true,
            BinOp::NotEqual => true,
            BinOp::LessThan => true,
            BinOp::LessThanOrEqual => true,
            BinOp::GreaterThan => true,
            BinOp::GreaterThanOrEqual => true,
            BinOp::Or => true,
            BinOp::And => true,
            BinOp::Implies => true,
            _ => false,
        }
    }

    pub fn is_assignment(&self) -> bool {
        match self {
            BinOp::Equal => true,
            BinOp::PlusEqual => true,
            BinOp::MinusEqual => true,
            BinOp::TimesEqual => true,
            BinOp::DivideEqual => true,
            _ => false,
        }
    }

    pub fn is_assignment_shorthand(&self) -> bool {
        match self {
            BinOp::PlusEqual => true,
            BinOp::MinusEqual => true,
            BinOp::TimesEqual => true,
            BinOp::DivideEqual => true,
            _ => false,
        }
    }

    pub fn get_assignment_shorthand(&self) -> BinOp {
        match self {
            BinOp::PlusEqual => BinOp::Plus,
            BinOp::MinusEqual => BinOp::Minus,
            BinOp::TimesEqual => BinOp::Times,
            BinOp::DivideEqual => BinOp::Divide,
            _ => unimplemented!(),
        }
    }
}