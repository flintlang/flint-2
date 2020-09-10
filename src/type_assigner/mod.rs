use super::ast::*;
use super::context::*;
use super::visitor::*;
use crate::type_checker::ExpressionChecker;

pub struct TypeAssigner {}

impl Visitor for TypeAssigner {
    fn start_variable_declaration(
        &mut self,
        declaration: &mut VariableDeclaration,
        ctx: &mut Context,
    ) -> VResult {
        if ctx.in_function_or_special() {
            if let Some(ref mut context) = ctx.scope_context {
                context.local_variables.push(declaration.clone());
            }

            if let Some(ref mut function_declaration_context) = ctx.function_declaration_context {
                function_declaration_context
                    .local_variables
                    .push(declaration.clone());
            } else if let Some(ref mut special_declaration_context) =
            ctx.special_declaration_context
            {
                special_declaration_context.local_variables.push(declaration.clone());
            }
        }
        Ok(())
    }

    fn finish_binary_expression(
        &mut self,
        bin_expr: &mut BinaryExpression,
        ctx: &mut Context,
    ) -> VResult {
        if let BinOp::Dot = bin_expr.op {
            let enclosing = ctx.enclosing_type_identifier();
            let enclosing = enclosing.unwrap();
            let scope = &ctx.scope_or_default();
            let lhs_type = ctx.environment.get_expression_type(
                &*bin_expr.lhs_expression,
                &enclosing.token,
                &[],
                &[],
                scope,
            );
            if let Expression::Identifier(i) = &*bin_expr.lhs_expression {
                if ctx.environment.is_enum_declared(&i.token) {
                    bin_expr.rhs_expression.assign_enclosing_type(&i.token)
                } else {
                    bin_expr
                        .rhs_expression
                        .assign_enclosing_type(&lhs_type.name());
                }
            } else if let Type::SelfType = lhs_type {
                if let Some(ref trait_ctx) = ctx.trait_declaration_context {
                    let trait_name = &trait_ctx.identifier.token;

                    bin_expr.rhs_expression.assign_enclosing_type(trait_name);
                }
            } else {
                bin_expr
                    .rhs_expression
                    .assign_enclosing_type(&lhs_type.name());
            }
        }
        Ok(())
    }
}
/*
#[cfg(test)]
mod test {

    use super::*;
    use std::collections::HashMap;
    use crate::environment::*;
    use crate::type_assigner::TypeAssigner;

    #[test]
    fn test_finish_binary_expression() {
        let mut type_assigner = TypeAssigner {};
        let mut bin_expr = BinaryExpression {
            lhs_expression: Box::new(Expression::Literal(Literal::IntLiteral(5))),
            rhs_expression: Box::new(Expression::Literal(Literal::IntLiteral(3))),
            op: BinOp::Plus,
            line_info: LineInfo {line:1, offset:0}
        };
        let mut context = Context {
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

        assert!(type_assigner.finish_binary_expression(&mut bin_expr, &mut context).is_ok());
    }
}
*/
