use super::call::MoveFunctionCall;
use super::expression::MoveExpression;
use super::function::FunctionContext;
use super::ir::{MoveIRAssignment, MoveIRExpression, MoveIRIf, MoveIRStatement};
use crate::ast::{
    EmitStatement, ForStatement, Identifier, IfStatement, ReturnStatement, Statement,
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
            Statement::BecomeStatement(_) => {
                panic!("Should have been implemented in the preprocessor")
            }
            Statement::EmitStatement(e) => {
                MoveEmitStatement { statement: e }.generate(function_context)
            }
            Statement::ForStatement(f) => {
                MoveForStatement { statement: *f }.generate(function_context)
            }
            Statement::IfStatement(i) => {
                MoveIfStatement { statement: i }.generate(function_context)
            }
            Statement::DoCatchStatement(_) => panic!("Do Catch not currently supported"),
            Statement::Assertion(a) => MoveIRStatement::Assert(
                MoveExpression {
                    expression: a.expression,
                    position: Default::default(),
                }
                .generate(function_context),
                a.line_info.line,
            ),
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

        let count = function_context.push_block();

        for statement in self.statement.body.clone() {
            let statement = MoveStatement { statement }.generate(function_context);
            function_context.emit(statement);
        }

        let body = function_context.with_new_block(count);

        if self.statement.else_body.is_empty() {
            MoveIRStatement::If(MoveIRIf {
                expression: condition,
                block: body,
                else_block: None,
            })
        } else {
            let count = function_context.push_block();

            for statement in self.statement.else_body.clone() {
                let statement = MoveStatement { statement }.generate(function_context);
                function_context.emit(statement);
            }

            let else_block = function_context.with_new_block(count);

            MoveIRStatement::If(MoveIRIf {
                expression: condition,
                block: body,
                else_block: Some(else_block),
            })
        }
    }
}

struct MoveReturnStatement {
    pub statement: ReturnStatement,
}

impl MoveReturnStatement {
    pub fn generate(&self, mut function_context: &mut FunctionContext) -> MoveIRStatement {
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

        let (cleanup, expression) =
            remove_moves(self.statement.cleanup.iter().cloned(), expression);
        let assignment = MoveIRExpression::Assignment(MoveIRAssignment {
            identifier: return_identifier.token.clone(),
            expression: Box::from(expression),
        });
        function_context.emit(MoveIRStatement::Expression(assignment));

        for statement in cleanup.clone() {
            let move_statement = MoveStatement { statement }.generate(&mut function_context);
            function_context.emit(move_statement);
        }

        let string = format!(
            "return move({identifier})",
            identifier = return_identifier.token
        );
        MoveIRStatement::Inline(string)
    }
}

struct MoveForStatement {
    pub statement: ForStatement,
}

impl MoveForStatement {
    pub fn generate(&self, _function_context: &FunctionContext) -> MoveIRStatement {
        unimplemented!()
    }
}

struct MoveEmitStatement {
    pub statement: EmitStatement,
}

impl MoveEmitStatement {
    pub fn generate(&self, function_context: &FunctionContext) -> MoveIRStatement {
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
