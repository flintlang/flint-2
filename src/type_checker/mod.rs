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
        declaration: &mut ContractBehaviourDeclaration,
        ctx: &mut Context,
    ) -> VResult {
        let states = declaration.type_states.clone();
        for state in states {
            if !ctx
                .environment
                .is_state_declared(&state, &declaration.identifier.token)
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
        declaration: &mut VariableDeclaration,
        ctx: &mut Context,
    ) -> VResult {
        if ctx.in_function_or_special() {
            if let Some(context) = ctx.scope_context.as_mut() {
                context.local_variables.push(declaration.clone());
            }

            if let Some(ref mut function_declaration_context) = ctx.function_declaration_context {
                function_declaration_context
                    .local_variables
                    .push(declaration.clone());
            } else if let Some(ref mut special_declaration_context) =
                ctx.special_declaration_context
            {
                special_declaration_context
                    .local_variables
                    .push(declaration.clone());
            }
        }
        Ok(())
    }

    fn start_binary_expression(
        &mut self,
        declaration: &mut BinaryExpression,
        ctx: &mut Context,
    ) -> VResult {
        let enclosing = ctx
            .enclosing_type_identifier()
            .map(|id| &*id.token)
            .unwrap_or_default();
        let lhs_type = ctx.environment.get_expression_type(
            &*declaration.lhs_expression,
            enclosing,
            &[],
            &[],
            ctx.scope_or_default(),
        );
        match declaration.op {
            BinOp::Dot => declaration
                .rhs_expression
                .assign_enclosing_type(&lhs_type.name()),
            BinOp::Equal => {}
            _ => {}
        }
        Ok(())
    }
}
