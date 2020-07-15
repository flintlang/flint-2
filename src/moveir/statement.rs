use super::function::FunctionContext;
use super::ir::{MoveIRStatement, MoveIRIf, MoveIRExpression, MoveIRAssignment};
use crate::ast::{EmitStatement, ForStatement, BecomeStatement, ReturnStatement, IfStatement, Statement, Identifier};
use super::expression::MoveExpression;
use super::call::MoveFunctionCall;

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
            function_context.emit_release_references();
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
        let assignment = MoveIRExpression::Assignment(MoveIRAssignment {
            identifier: return_identifier.token.clone(),
            expression: Box::from(expression),
        });
        function_context.emit(MoveIRStatement::Expression(assignment));

        for statement in self.statement.cleanup.clone() {
            let move_statement = MoveStatement { statement }.generate(function_context);
            function_context.emit(move_statement);
        }

        function_context.emit_release_references();
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