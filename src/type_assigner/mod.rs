
use crate::type_checker::ExpressionCheck;

use super::context::*;
use super::visitor::*;
use super::ast::*;

pub struct TypeAssigner {}

impl Visitor for TypeAssigner {
    fn start_variable_declaration(
        &mut self,
        _t: &mut VariableDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        if _ctx.in_function_or_special() {
            if _ctx.scope_context().is_some() {
                let context_ref = _ctx.scope_context.as_mut().unwrap();
                context_ref.local_variables.push(_t.clone());
            }

            if _ctx.is_function_declaration_context() {
                let context_ref = _ctx.function_declaration_context.as_mut().unwrap();
                context_ref.local_variables.push(_t.clone());
            }

            if _ctx.is_special_declaration_context() {
                let context_ref = _ctx.special_declaration_context.as_mut().unwrap();
                context_ref.local_variables.push(_t.clone());
            }
        }
        Ok(())
    }

    fn finish_binary_expression(
        &mut self,
        _t: &mut BinaryExpression,
        _ctx: &mut Context,
    ) -> VResult {
        if let BinOp::Dot = _t.op {
            let enclosing = _ctx.enclosing_type_identifier().clone();
            let enclosing = enclosing.unwrap();
            let scope = _ctx.scope_context.clone();
            let scope = scope.unwrap_or_default();
            let lhs_type = _ctx.environment.get_expression_type(
                *_t.lhs_expression.clone(),
                &enclosing.token,
                vec![],
                vec![],
                scope,
            );
            if let Expression::Identifier(i) = &*_t.lhs_expression {
                if _ctx.environment.is_enum_declared(&i.token) {
                    _t.rhs_expression.assign_enclosing_type(&i.token)
                } else {
                    _t.rhs_expression.assign_enclosing_type(&lhs_type.name());
                }
            } else if let Type::SelfType = lhs_type {
                if _ctx.trait_declaration_context.is_some() {
                    let trait_ctx = _ctx.trait_declaration_context.clone();
                    let trait_ctx = trait_ctx.unwrap();
                    let trait_name = trait_ctx.identifier.token;

                    _t.rhs_expression.assign_enclosing_type(&trait_name);
                }
            } else {
                _t.rhs_expression.assign_enclosing_type(&lhs_type.name());
            }
        }
        Ok(())
    }
}
