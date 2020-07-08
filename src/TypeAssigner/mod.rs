use super::context::*;
use super::visitor::*;
use super::AST::*;

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

/*
#[cfg(test)]
mod test {

    use crate::context::*;
    use crate::visitor::*;
    use crate::AST::*;
    use crate::TypeAssigner::TypeAssigner;
    use std::collections::HashMap;

    fn test_finish_binary_expression() {
        let type_assigner = TypeAssigner {};
        let bin_expr = BinaryExpression {
            lhs_expression: Box::new(Expression::Literal(Literal::IntLiteral(5))),
            rhs_expression: Box::new(Expression::Literal(Literal::IntLiteral(3))),
            op: BinOp::Plus,
            line_info: LineInfo {line:1, offset:0}
        };
        let context = Context {
            environment: Environment {
                contract_declarations: vec![],
                struct_declarations: vec![],
                enum_declarations: vec![],
                event_declarations: vec![],
                trait_declarations: vec![],
                asset_declarations: vec![],
                types: HashMap::new()
            },

            contract_declaration_context: None,
            contract_behaviour_declaration_context: None,
            struct_declaration_context: None,
            function_declaration_context: None,
            special_declaration_context: None,
            trait_declaration_context: None,
            scope_context: None,
            asset_context: None,
            block_context: None,
            function_call_receiver_trail: vec![],
            is_property_default_assignment: false,
            is_function_call_context: false,
            is_function_call_argument: false,
            is_function_call_argument_label: false,
            external_call_context: None,
            is_external_function_call: false,
            in_assignment: false,
            in_if_condition: false,
            in_become: false,
            is_lvalue: false,
            in_subscript: false,
            is_enclosing: false,
            in_emit: false,
            pre_statements: vec![],
            post_statements: vec![]
        };

        assert_eq!(Ok(()), finish_binary_expression(type_assigner, bin_expr, context));
    }
}
*/
