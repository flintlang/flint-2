use crate::ast::Type;
use nom::lib::std::fmt::Formatter;
use std::fmt::{Display, Result};

#[allow(dead_code)]
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
}

use self::BinOp::*;

impl Display for BinOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}",
            match self {
                Plus => "+",
                OverflowingPlus => "&+",
                Minus => "-",
                OverflowingMinus => "&-",
                Times => "*",
                OverflowingTimes => "&*",
                Power => "^",
                Divide => "/",
                Percent => "%",
                Dot => ".",
                Equal => "=",
                PlusEqual => "+=",
                MinusEqual => "-=",
                TimesEqual => "*=",
                DivideEqual => "/=",
                DoubleEqual => "==",
                NotEqual => "!=",
                LessThan => "<",
                LessThanOrEqual => "<=",
                GreaterThan => ">",
                GreaterThanOrEqual => ">=",
                Or => "||",
                And => "&&",
            }
        )
    }
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
            _ => false,
        }
    }

    pub fn is_assignment(&self) -> bool {
        match self {
            BinOp::Equal => true,
            _ => self.is_assignment_shorthand(),
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
            BinOp::PlusEqual => Plus,
            BinOp::MinusEqual => Minus,
            BinOp::TimesEqual => Times,
            BinOp::DivideEqual => Divide,
            _ => unimplemented!(),
        }
    }

    pub fn accepts(&self, left: &Type, right: &Type) -> bool {
        match self {
            BinOp::Dot => !matches!(*left, Type::Int | Type::Bool | Type::Address),
            BinOp::PlusEqual
            | BinOp::Plus
            | BinOp::OverflowingPlus
            | BinOp::Minus
            | BinOp::MinusEqual
            | BinOp::OverflowingMinus
            | BinOp::Times
            | BinOp::TimesEqual
            | BinOp::OverflowingTimes
            | BinOp::Divide
            | BinOp::DivideEqual
            | BinOp::Power
            | BinOp::Percent
            | BinOp::GreaterThan
            | BinOp::LessThan
            | BinOp::GreaterThanOrEqual
            | BinOp::LessThanOrEqual => *left == Type::Int && *right == Type::Int,
            BinOp::And | BinOp::Or => *left == Type::Bool && *right == Type::Bool,
            _ => *left == *right,
        }
    }
}
