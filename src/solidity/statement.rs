use super::*;

pub struct SolidityStatement {
    pub statement: Statement,
}

impl SolidityStatement {
    pub fn generate(&self, function_context: &mut FunctionContext) -> YulStatement {
        match self.statement.clone() {
            Statement::ReturnStatement(r) => {
                SolidityReturnStatement { statement: r }.generate(function_context)
            }
            Statement::Expression(e) => YulStatement::Expression(
                SolidityExpression {
                    expression: e,
                    is_lvalue: false,
                }
                .generate(function_context),
            ),
            Statement::BecomeStatement(_) => panic!("Become Statement Not Currently Supported"),
            Statement::EmitStatement(_) => unimplemented!(),
            Statement::ForStatement(_) => unimplemented!(),
            Statement::IfStatement(i) => {
                SolidityIfStatement { statement: i }.generate(function_context)
            }
            Statement::DoCatchStatement(_) => panic!("Catch Statement Not Currently Supported"),
            Statement::Assertion(_) => panic!("Assertions not supported"),
        }
    }
}

struct SolidityReturnStatement {
    pub statement: ReturnStatement,
}

impl SolidityReturnStatement {
    pub fn generate(&self, function_context: &mut FunctionContext) -> YulStatement {
        if self.statement.expression.is_none() {
            return YulStatement::Inline("".to_string());
        }
        let expression = self.statement.expression.clone();
        let expression = expression.unwrap();
        let expression = SolidityExpression {
            expression,
            is_lvalue: false,
        }
        .generate(function_context);
        let string = format!("ret := {expression}", expression = expression);
        YulStatement::Inline(string)
    }
}

struct SolidityIfStatement {
    pub statement: IfStatement,
}

impl SolidityIfStatement {
    pub fn generate(&self, function_context: &mut FunctionContext) -> YulStatement {
        let condition = SolidityExpression {
            expression: self.statement.condition.clone(),
            is_lvalue: false,
        }
        .generate(function_context);

        println!("With new block");
        let count = function_context.push_block();
        for statement in self.statement.body.clone() {
            let statement = SolidityStatement { statement }.generate(function_context);
            function_context.emit(statement);
        }
        let body = function_context.with_new_block(count);

        YulStatement::Switch(YulSwitch {
            expression: condition,
            cases: vec![(YulLiteral::Num(1), body)],
            default: None,
        })
    }
}
