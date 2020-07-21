use super::call::MoveFunctionCall;
use super::expression::MoveExpression;
use super::function::FunctionContext;
use super::ir::{MoveIRAssignment, MoveIRExpression, MoveIRIf, MoveIRStatement};
use crate::ast::{
    BecomeStatement, EmitStatement, ForStatement, Identifier, IfStatement, ReturnStatement,
    Statement,
};
use crate::moveir::utils::*;

pub struct MoveStatement {
    pub statement: Statement,
}

impl MoveStatement {
    pub(crate) fn generate(&self, function_context: &mut FunctionContext) -> MoveIRStatement {
        match self.statement.clone() {
            Statement::ReturnStatement(r) => {
                MoveReturnStatement { statement: r }.generate(function_context)
            }
            Statement::Expression(e) => MoveIRStatement::Expression(
                MoveExpression {
                    expression: e,
                    position: Default::default(),
                }
                .generate(function_context),
            ),
            Statement::BecomeStatement(b) => {
                MoveBecomeStatement { statement: b }.generate(function_context)
            }
            Statement::EmitStatement(e) => {
                MoveEmitStatement { statement: e }.generate(function_context)
            }
            Statement::ForStatement(f) => {
                MoveForStatement { statement: f }.generate(function_context)
            }
            Statement::IfStatement(i) => {
                MoveIfStatement { statement: i }.generate(function_context)
            }
            Statement::DoCatchStatement(_) => panic!("Do Catch not currently supported"),
        }
    }
}

struct MoveIfStatement {
    pub statement: IfStatement,
}

impl MoveIfStatement {
    pub fn generate(&self, function_context: &mut FunctionContext) -> MoveIRStatement {
        let condition = MoveExpression {
            expression: self.statement.condition.clone(),
            position: Default::default(),
        }
        .generate(function_context);
        println!("With new block");
        let count = function_context.push_block();
        for statement in self.statement.body.clone() {
            let statement = MoveStatement { statement }.generate(function_context);
            function_context.emit(statement);
        }
        let body = function_context.with_new_block(count);
        MoveIRStatement::If(MoveIRIf {
            expression: condition,
            block: body,
            else_block: None,
        })
    }
}

struct MoveReturnStatement {
    pub statement: ReturnStatement,
}

impl MoveReturnStatement {
    pub fn generate(&self, function_context: &mut FunctionContext) -> MoveIRStatement {
        if self.statement.expression.is_none() {
            return MoveIRStatement::Inline(String::from("return"));
        }

        let return_identifier = Identifier {
            token: "ret".to_string(),
            enclosing_type: None,
            line_info: self.statement.line_info.clone(),
        };
        let expression = self.statement.expression.clone().unwrap();
        let expression = MoveExpression {
            expression,
            position: Default::default(),
        }
        .generate(&function_context);

        let (cleanup, expression) = remove_moves(self.statement.cleanup.clone(), expression);

        let assignment = MoveIRExpression::Assignment(MoveIRAssignment {
            identifier: return_identifier.token.clone(),
            expression: Box::from(expression),
        });
        function_context.emit(MoveIRStatement::Expression(assignment));

        for statement in cleanup.clone() {
            let move_statement = MoveStatement { statement }.generate(function_context);
            function_context.emit(move_statement);
        }

        let string = format!(
            "return move({identifier})",
            identifier = return_identifier.token
        );
        MoveIRStatement::Inline(string)
    }
}

struct MoveBecomeStatement {
    pub statement: BecomeStatement,
}

impl MoveBecomeStatement {
    pub fn generate(&self, _function_context: &mut FunctionContext) -> MoveIRStatement {
        panic!("Become Statements not currently supported")
    }
}

struct MoveForStatement {
    pub statement: ForStatement,
}

impl MoveForStatement {
    pub fn generate(&self, _function_context: &mut FunctionContext) -> MoveIRStatement {
        unimplemented!()
    }
}

struct MoveEmitStatement {
    pub statement: EmitStatement,
}

impl MoveEmitStatement {
    pub fn generate(&self, function_context: &mut FunctionContext) -> MoveIRStatement {
        MoveIRStatement::Inline(format!(
            "{}",
            MoveFunctionCall {
                function_call: self.statement.function_call.clone(),
                module_name: "Self".to_string(),
            }
            .generate(function_context)
        ))
    }
}

#[cfg(test)]
mod test {

    use crate::ast::expressions::BinaryExpression;
    use crate::ast::expressions::Expression::*;
    use crate::ast::expressions::Identifier;
    use crate::ast::operators::BinOp::Equal;
    use crate::ast::statements::Statement::Expression;
    use crate::ast::types::InoutType;
    use crate::ast::types::Type;
    use crate::ast::LineInfo;
    use crate::moveir::ir::MoveIRExpression;
    use crate::moveir::ir::MoveIROperation;
    use crate::moveir::ir::MoveIRTransfer;

    use crate::moveir::statement::remove_move;

    #[test]
    fn test_remove_move() {
        let expr = MoveIRExpression::Operation(MoveIROperation::Add(
            Box::new(MoveIRExpression::Operation(MoveIROperation::Access(
                Box::new(MoveIRExpression::Operation(MoveIROperation::Dereference(
                    Box::new(MoveIRExpression::Operation(
                        MoveIROperation::MutableReference(Box::new(MoveIRExpression::Transfer(
                            MoveIRTransfer::Copy(Box::new(MoveIRExpression::Identifier(
                                "_temp__3".to_string(),
                            ))),
                        ))),
                    )),
                ))),
                "width".to_string(),
            ))),
            Box::new(MoveIRExpression::Operation(MoveIROperation::Access(
                Box::new(MoveIRExpression::Operation(MoveIROperation::Dereference(
                    Box::new(MoveIRExpression::Operation(
                        MoveIROperation::MutableReference(Box::new(MoveIRExpression::Transfer(
                            MoveIRTransfer::Copy(Box::new(MoveIRExpression::Identifier(
                                "_temp__3".to_string(),
                            ))),
                        ))),
                    )),
                ))),
                "height".to_string(),
            ))),
        ));
        let statement = Expression(BinaryExpression(BinaryExpression {
            lhs_expression: Box::new(RawAssembly(
                "_".to_string(),
                Some(Type::InoutType(InoutType {
                    key_type: Box::new(Type::UserDefinedType(Identifier {
                        token: "Rectangle".to_string(),
                        enclosing_type: None,
                        line_info: LineInfo {
                            line: 2,
                            offset: 37,
                        },
                    })),
                })),
            )),
            rhs_expression: Box::new(Identifier(Identifier {
                token: "temp__3".to_string(),
                enclosing_type: None,
                line_info: LineInfo {
                    line: 18,
                    offset: 377,
                },
            })),
            op: Equal,
            line_info: LineInfo {
                line: 18,
                offset: 377,
            },
        }));

        let result = remove_move(&statement, &expr).expect("Error with remove_move");

        assert_eq!(
            result,
            MoveIRExpression::Operation(MoveIROperation::Add(
                Box::new(MoveIRExpression::Operation(MoveIROperation::Access(
                    Box::new(MoveIRExpression::Operation(MoveIROperation::Dereference(
                        Box::new(MoveIRExpression::Operation(
                            MoveIROperation::MutableReference(Box::new(
                                MoveIRExpression::Transfer(MoveIRTransfer::Copy(Box::new(
                                    MoveIRExpression::Identifier("_temp__3".to_string())
                                )),)
                            ))
                        ))
                    ))),
                    "width".to_string(),
                ))),
                Box::new(MoveIRExpression::Operation(MoveIROperation::Access(
                    Box::new(MoveIRExpression::Operation(MoveIROperation::Dereference(
                        Box::new(MoveIRExpression::Operation(
                            MoveIROperation::MutableReference(Box::new(
                                MoveIRExpression::Transfer(MoveIRTransfer::Move(Box::new(
                                    MoveIRExpression::Identifier("_temp__3".to_string())
                                )),)
                            ))
                        ))
                    ))),
                    "height".to_string(),
                )),)
            ))
        );
    }
}
