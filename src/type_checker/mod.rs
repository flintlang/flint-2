mod type_error;

use super::ast::*;
use super::context::*;
use super::visitor::*;
use type_error::StateTypeError;

pub struct TypeChecker {}

pub trait ExpressionChecker {
    fn get_expression_type(
        &self,
        expr: &Expression,
        type_id: &str,
        type_states: &[TypeState],
        caller_protections: &[CallerProtection],
        scope: &ScopeContext,
    ) -> Type;
}

impl Visitor for TypeChecker {
    fn start_contract_behaviour_declaration(
        &mut self,
        _t: &mut ContractBehaviourDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        let states = _t.type_states.clone();
        for state in states {
            if !_ctx
                .environment
                .is_state_declared(&state, &_t.identifier.token)
                && !state.is_any()
            {
                return Err(Box::from(StateTypeError::new(
                    state.identifier.token,
                    state.identifier.line_info.line,
                )));
            }
        }

        Ok(())
    }

    fn start_variable_declaration(
        &mut self,
        _t: &mut VariableDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        if _ctx.in_function_or_special() {
            if let Some(context) = _ctx.scope_context.as_mut() {
                context.local_variables.push(_t.clone());
            }

            if let Some(ref mut function_declaration_context) = _ctx.function_declaration_context {
                function_declaration_context
                    .local_variables
                    .push(_t.clone());
            } else if let Some(ref mut special_declaration_context) =
                _ctx.special_declaration_context
            {
                special_declaration_context.local_variables.push(_t.clone());
            }
        }
        Ok(())
    }

    fn start_binary_expression(
        &mut self,
        _t: &mut BinaryExpression,
        _ctx: &mut Context,
    ) -> VResult {
        let enclosing = _ctx
            .enclosing_type_identifier()
            .map(|id| &*id.token)
            .unwrap_or_default();
        let lhs_type = _ctx.environment.get_expression_type(
            &*_t.lhs_expression,
            enclosing,
            &[],
            &[],
            _ctx.scope_or_default(),
        );
        match _t.op {
            BinOp::Dot => _t.rhs_expression.assign_enclosing_type(&lhs_type.name()),
            BinOp::Equal => {}
            _ => {}
        }
        Ok(())
    }
}
