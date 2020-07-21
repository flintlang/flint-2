use crate::ast::*;
use crate::context::Context;
use crate::visitor::Visitor;

#[derive(Clone, Debug, PartialEq)]
pub struct DictionaryLiteral {
    pub elements: Vec<(Expression, Expression)>,
}

impl Visitable for DictionaryLiteral {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_dictionary_literal(self, ctx)?;

        for (e, l) in &mut self.elements {
            e.visit(v, ctx)?;
            l.visit(v, ctx)?;
        }
        v.finish_dictionary_literal(self, ctx)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayLiteral {
    pub elements: Vec<Expression>,
}

impl Visitable for ArrayLiteral {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_array_literal(self, ctx)?;

        self.elements.visit(v, ctx)?;

        v.finish_array_literal(self, ctx)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Literal {
    BooleanLiteral(bool),
    AddressLiteral(String),
    // TODO make this a defined vec of size 32 to avoid bad addresses?
    StringLiteral(String),
    U8Literal(u8),
    IntLiteral(u64),
    FloatLiteral(f64),
}

impl Visitable for Literal {
    fn visit(&mut self, _v: &mut dyn Visitor, _ctx: &mut Context) -> VResult {
        Ok(())
    }
}
