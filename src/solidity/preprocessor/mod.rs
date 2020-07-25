mod utils;

use crate::ast::Expression::SelfExpression;
use crate::ast::*;
use crate::context::*;
use crate::environment::*;
use crate::solidity::preprocessor::utils::*;
use crate::type_checker::ExpressionChecker;
use crate::visitor::Visitor;

pub(crate) struct SolidityPreProcessor {}

impl Visitor for SolidityPreProcessor {
    fn start_function_declaration(
        &mut self,
        _t: &mut FunctionDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        let enclosing_identifier = _ctx
            .enclosing_type_identifier().map(|id| &*id.token).unwrap_or_default();

        let param_types = _t.head.parameter_types().clone();
        let mangled_name = mangle_solidity_function_name(
            _t.head.identifier.token.clone(),
            param_types,
            enclosing_identifier,
        );
        _t.mangled_identifier = Some(mangled_name);

        if let Some(ref struct_ctx) = _ctx.struct_declaration_context {
            if enclosing_identifier != "Quartz_Global".to_string() {
                let param = construct_parameter(
                    "QuartzSelf".to_string(),
                    Type::InoutType(InoutType {
                        key_type: Box::new(Type::UserDefinedType(Identifier::generated(
                            &struct_ctx.identifier.token,
                        ))),
                    }),
                );

                _t.head.parameters.insert(0, param);
            }
        }

        let dynamic_params = _t.head.parameters.clone();
        let dynamic_params: Vec<Parameter> = dynamic_params
            .into_iter()
            .filter(|p| p.is_dynamic())
            .collect();

        for (index, (offset, p)) in dynamic_params.into_iter().enumerate().enumerate() {
            let ismem_param = construct_parameter(mangle_mem(&p.identifier.token), Type::Bool);
            _t.head.parameters.insert(index + offset + 1, ismem_param);
        }

        Ok(())
    }

    fn start_expression(&mut self, expression: &mut Expression, _ctx: &mut Context) -> VResult {
        if let Expression::BinaryExpression(binary) = expression.clone() {
            if let BinOp::Dot = binary.op {
                if let Expression::Identifier(lhs) = *binary.lhs_expression {
                    if let Expression::Identifier(_) = *binary.rhs_expression {
                        if _ctx.environment.is_enum_declared(&lhs.token) {
                            unimplemented!()
                        }
                    }
                }
            } else if let BinOp::Equal = binary.op {
                if let Expression::FunctionCall(mut call) = *binary.rhs_expression.clone() {
                    if _ctx.environment.is_initialise_call(&call) {
                        let inout = Expression::InoutExpression(InoutExpression {
                            ampersand_token: "&".to_string(),
                            expression: binary.lhs_expression.clone(),
                        });
                        call.arguments.insert(
                            0,
                            FunctionArgument {
                                identifier: None,
                                expression: inout,
                            },
                        );

                        *expression = Expression::FunctionCall(call.clone());

                        if let Expression::VariableDeclaration(v) = *binary.lhs_expression.clone() {
                            if v.variable_type.is_dynamic_type() {
                                let function_arg = Expression::Identifier(v.identifier.clone());
                                let function_arg = Expression::InoutExpression(InoutExpression {
                                    ampersand_token: "".to_string(),
                                    expression: Box::new(function_arg),
                                });

                                let mut call = call.clone();
                                call.arguments.remove(0);
                                call.arguments.insert(
                                    0,
                                    FunctionArgument {
                                        identifier: None,
                                        expression: function_arg,
                                    },
                                );

                                *expression = Expression::Sequence(vec![
                                    Expression::VariableDeclaration(v.clone()),
                                    Expression::FunctionCall(call.clone()),
                                ]);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn start_binary_expression(
        &mut self,
        _t: &mut BinaryExpression,
        _ctx: &mut Context,
    ) -> VResult {
        if _t.op.is_assignment_shorthand() {
            let op = _t.op.clone();
            let op = op.get_assignment_shorthand();
            _t.op = BinOp::Equal;

            let rhs = BinaryExpression {
                lhs_expression: _t.lhs_expression.clone(),
                rhs_expression: _t.rhs_expression.clone(),
                op,
                line_info: _t.line_info.clone(),
            };
            _t.rhs_expression = Box::from(Expression::BinaryExpression(rhs));
        } else if let BinOp::Dot = _t.op {
            let mut trail = _ctx.function_call_receiver_trail.clone();
            trail.push(*_t.lhs_expression.clone());
            _ctx.function_call_receiver_trail = trail;
        }

        let op = _t.op.clone();

        if let BinOp::LessThanOrEqual = op {
            let lhs = Expression::BinaryExpression(BinaryExpression {
                lhs_expression: _t.lhs_expression.clone(),
                rhs_expression: _t.rhs_expression.clone(),
                op: BinOp::LessThan,
                line_info: _t.line_info.clone(),
            });
            let rhs = Expression::BinaryExpression(BinaryExpression {
                lhs_expression: _t.lhs_expression.clone(),
                rhs_expression: _t.rhs_expression.clone(),
                op: BinOp::DoubleEqual,
                line_info: _t.line_info.clone(),
            });
            _t.lhs_expression = Box::from(lhs);

            _t.rhs_expression = Box::from(rhs);
            _t.op = BinOp::Or;
        } else if let BinOp::GreaterThanOrEqual = op {
            let lhs = Expression::BinaryExpression(BinaryExpression {
                lhs_expression: _t.lhs_expression.clone(),
                rhs_expression: _t.rhs_expression.clone(),
                op: BinOp::GreaterThan,
                line_info: _t.line_info.clone(),
            });
            let rhs = Expression::BinaryExpression(BinaryExpression {
                lhs_expression: _t.lhs_expression.clone(),
                rhs_expression: _t.rhs_expression.clone(),
                op: BinOp::DoubleEqual,
                line_info: _t.line_info.clone(),
            });
            _t.lhs_expression = Box::from(lhs);

            _t.rhs_expression = Box::from(rhs);
            _t.op = BinOp::Or;
        }

        Ok(())
    }

    fn start_variable_declaration(
        &mut self,
        _t: &mut VariableDeclaration,
        _ctx: &mut Context,
    ) -> VResult {
        if _ctx.in_function_or_special() {
            if let Some(ref mut context) = _ctx.scope_context {
                context.local_variables.push(_t.clone());
            }

            if let Some(ref mut function_declaration_context) = _ctx.function_declaration_context {
                function_declaration_context.local_variables.push(_t.clone());
            } else if let Some(ref mut special_declaration_context) = _ctx.special_declaration_context {
                special_declaration_context.local_variables.push(_t.clone());
            }
        }
        Ok(())
    }

    fn start_function_call(&mut self, call: &mut FunctionCall, _ctx: &mut Context) -> VResult {
        if is_ether_runtime_function_call(call) {
            return Ok(());
        }

        if _ctx.function_call_receiver_trail.is_empty() {
            _ctx.function_call_receiver_trail = vec![Expression::SelfExpression];
        }

        let mut f_call = call.clone();
        if _ctx.environment.is_initialise_call(&f_call) {
            let mut temp = f_call.clone();
            if _ctx.function_declaration_context.is_some()
                || _ctx.special_declaration_context.is_some() && !temp.arguments.is_empty()
            {
                temp.arguments.remove(0);
            }

            let mangled = mangle_function_call_name(&temp, _ctx);
            if mangled.is_some() {
                let mangled = mangled.unwrap();
                call.mangled_identifier = Option::from(Identifier {
                    token: mangled.clone(),
                    enclosing_type: None,
                    line_info: Default::default(),
                });
                f_call.mangled_identifier = Option::from(Identifier {
                    token: mangled,
                    enclosing_type: None,
                    line_info: Default::default(),
                });
            }
        } else {
            let enclosing_type = if is_global_function_call(&f_call, _ctx) {
                "Quartz_Global".to_string()
            } else {
                let trail_last = _ctx.function_call_receiver_trail.last().unwrap();

                let enclosing_ident = _ctx.enclosing_type_identifier().map(|id| &*id.token).unwrap_or_default();

                let scope = _ctx.scope_context.clone();
                let scope = scope.unwrap_or(ScopeContext {
                    parameters: vec![],
                    local_variables: vec![],
                    counter: 0,
                });

                let d_type = _ctx.environment.get_expression_type(
                    trail_last,
                    enclosing_ident,
                    &[],
                    &[],
                    &scope,
                );

                d_type.name()
            };

            if let Some(mangled) = mangle_function_call_name(&f_call, _ctx) {
                call.mangled_identifier = Option::from(Identifier {
                    token: mangled.clone(),
                    enclosing_type: None,
                    line_info: Default::default(),
                });
                f_call.mangled_identifier = Option::from(Identifier::generated(&mangled));
            }

            if _ctx.environment.is_struct_declared(&enclosing_type)
                && !is_global_function_call(&f_call, _ctx)
            {
                let receiver = construct_expression(_ctx.function_call_receiver_trail.clone());
                let inout_expression = InoutExpression {
                    ampersand_token: "".to_string(),
                    expression: Box::new(receiver),
                };
                f_call.arguments.insert(
                    0,
                    FunctionArgument {
                        identifier: None,
                        expression: Expression::InoutExpression(inout_expression),
                    },
                );
                *call = f_call.clone();
            }
        }

        println!("{:?}", _ctx.environment.is_initialise_call(&f_call));
        println!("{:?}", f_call.mangled_identifier);
        let scope = _ctx.scope_context.clone();
        let scope = scope.unwrap_or(ScopeContext {
            parameters: vec![],
            local_variables: vec![],
            counter: 0,
        });

        let enclosing: &str = if let Some(ref enclosing) =  f_call.identifier.enclosing_type {
            *&enclosing
        } else {
            _ctx.enclosing_type_identifier().map(|id| &*id.token).unwrap()
        };

        let match_result = _ctx
            .environment
            .match_function_call(&f_call, &enclosing, &[], &scope);

        let mut is_external = false;
        if let FunctionCallMatchResult::MatchedFunction(m) = match_result.clone() {
            is_external = m.declaration.is_external;
        }

        let mut f_call = f_call.clone();

        if !is_external {
            let mut offset = 0;
            let mut index = 0;
            let args = f_call.arguments.clone();
            for arg in args {
                let mut is_mem = SelfExpression;
                let param_name = scope.enclosing_parameter(&arg.expression, &enclosing);

                if param_name.is_some() {
                    let _param_name = param_name.unwrap();
                    unimplemented!()
                }

                let arg_type = _ctx.environment.get_expression_type(
                    &arg.expression,
                    &enclosing,
                    &[],
                    &[],
                    &scope,
                );

                if let Type::Error = arg_type.clone() {
                    panic!("Can not handle Type Error")
                }

                if !arg_type.is_dynamic_type() {
                    continue;
                }

                if arg.expression.enclosing_identifier().is_some() {
                    let arg_enclosing = arg.expression.enclosing_identifier().clone();
                    let arg_enclosing = arg_enclosing.unwrap();

                    if scope.contains_variable_declaration(&arg_enclosing.token) {
                        is_mem = Expression::Literal(Literal::BooleanLiteral(true));
                    } else if scope.contains_parameter_declaration(&arg_enclosing.token) {
                        is_mem = Expression::Identifier(Identifier::generated(&mangle_mem(
                            &arg_enclosing.token,
                        )));
                    }
                } else if let Expression::InoutExpression(i) = arg.expression.clone() {
                    if let Expression::SelfExpression = *i.expression.clone() {
                        is_mem = Expression::Identifier(Identifier {
                            token: mangle_mem("QuartzSelf"),
                            enclosing_type: None,
                            line_info: Default::default(),
                        });
                    }
                } else {
                    is_mem = Expression::Literal(Literal::BooleanLiteral(false));
                }

                f_call.arguments.insert(
                    index + offset + 1,
                    FunctionArgument {
                        identifier: None,
                        expression: is_mem,
                    },
                );
                offset += 1;
                index += 1;
            }
            *call = f_call;
        }

        _ctx.function_call_receiver_trail = vec![];

        Ok(())
    }

    fn start_struct_member(&mut self, _t: &mut StructMember, _ctx: &mut Context) -> VResult {
        let member = _t.clone();

        if let StructMember::SpecialDeclaration(s) = member {
            if s.is_init() {
                let mut new_s = s.clone();
                let default_assignments = default_assignments(_ctx);
                for d in default_assignments {
                    new_s.body.insert(0, d);
                }
                let new_init = new_s.as_function_declaration();
                *_t = StructMember::FunctionDeclaration(new_init);
            }
        }
        Ok(())
    }
}
