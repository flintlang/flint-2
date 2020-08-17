use crate::ast::{BinOp, BinaryExpression, Expression, Identifier, Literal, TypeState};
use itertools::Itertools;

pub fn extract_allowed_states<'a>(
    permitted_states: &'a [TypeState],
    declared_states: &'a [TypeState],
) -> impl Iterator<Item = u8> + 'a {
    assert!(declared_states.len() < 256);
    permitted_states.iter().map(move |permitted_state| {
        declared_states
            .iter()
            .position(|declared_state| declared_state == permitted_state)
            .unwrap() as u8
    })
}

pub fn generate_type_state_condition(id: Identifier, allowed: &[u8]) -> BinaryExpression {
    assert!(!allowed.is_empty());
    allowed
        .iter()
        .map(|state| BinaryExpression {
            lhs_expression: Box::from(Expression::Identifier(id.clone())),
            rhs_expression: Box::from(Expression::Literal(Literal::U8Literal(*state))),
            op: BinOp::DoubleEqual,
            line_info: Default::default(),
        })
        .fold1(|left, right| BinaryExpression {
            lhs_expression: Box::from(Expression::BinaryExpression(left)),
            rhs_expression: Box::from(Expression::BinaryExpression(right)),
            op: BinOp::Or,
            line_info: Default::default(),
        })
        .unwrap()
}
