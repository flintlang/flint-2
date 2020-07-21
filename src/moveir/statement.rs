use super::call::MoveFunctionCall;
use super::expression::MoveExpression;
use super::function::FunctionContext;
use super::ir::{
    MoveIRAssignment, MoveIRExpression, MoveIRFunctionCall, MoveIRIf, MoveIROperation,
    MoveIRStatement, MoveIRTransfer, MoveIRVector
};
use crate::ast::mangle;
use crate::ast::{
    BecomeStatement, EmitStatement, Expression, ForStatement, Identifier, IfStatement,
    ReturnStatement, Statement,
};

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

fn remove_move_op(op: &MoveIROperation, statement: &Statement) -> Option<MoveIRExpression> {
    match op {
        MoveIROperation::Add(l, r) => {
            if let Some(new_r) = remove_move(&statement, r) {
                Some(MoveIRExpression::Operation(MoveIROperation::Add(
                    l.clone(),
                    Box::new(new_r),
                )))
            } else if let Some(new_l) = remove_move(&statement, l) {
                Some(MoveIRExpression::Operation(MoveIROperation::Add(
                    Box::new(new_l),
                    r.clone(),
                )))
            } else {
                None
            }
        }
        MoveIROperation::Minus(l, r) => {
            if let Some(new_r) = remove_move(statement, r) {
                Some(MoveIRExpression::Operation(MoveIROperation::Minus(
                    l.clone(),
                    Box::new(new_r),
                )))
            } else if let Some(new_l) = remove_move(statement, l) {
                Some(MoveIRExpression::Operation(MoveIROperation::Minus(
                    Box::new(new_l),
                    r.clone(),
                )))
            } else {
                None
            }
        }
        MoveIROperation::Times(l, r) => {
            if let Some(new_r) = remove_move(statement, r) {
                Some(MoveIRExpression::Operation(MoveIROperation::Times(
                    l.clone(),
                    Box::new(new_r),
                )))
            } else if let Some(new_l) = remove_move(statement, l) {
                Some(MoveIRExpression::Operation(MoveIROperation::Times(
                    Box::new(new_l),
                    r.clone(),
                )))
            } else {
                None
            }
        }
        MoveIROperation::Divide(l, r) => {
            if let Some(new_r) = remove_move(statement, r) {
                Some(MoveIRExpression::Operation(MoveIROperation::Divide(
                    l.clone(),
                    Box::new(new_r),
                )))
            } else if let Some(new_l) = remove_move(statement, l) {
                Some(MoveIRExpression::Operation(MoveIROperation::Divide(
                    Box::new(new_l),
                    r.clone(),
                )))
            } else {
                None
            }
        }
        MoveIROperation::Modulo(l, r) => {
            if let Some(new_r) = remove_move(statement, r) {
                Some(MoveIRExpression::Operation(MoveIROperation::Modulo(
                    l.clone(),
                    Box::new(new_r),
                )))
            } else if let Some(new_l) = remove_move(statement, l) {
                Some(MoveIRExpression::Operation(MoveIROperation::Modulo(
                    Box::new(new_l),
                    r.clone(),
                )))
            } else {
                None
            }
        }
        MoveIROperation::GreaterThan(l, r) => {
            if let Some(new_r) = remove_move(statement, r) {
                Some(MoveIRExpression::Operation(MoveIROperation::GreaterThan(
                    l.clone(),
                    Box::new(new_r),
                )))
            } else if let Some(new_l) = remove_move(statement, l) {
                Some(MoveIRExpression::Operation(MoveIROperation::GreaterThan(
                    Box::new(new_l),
                    r.clone(),
                )))
            } else {
                None
            }
        }
        MoveIROperation::GreaterThanEqual(l, r) => {
            if let Some(new_r) = remove_move(statement, r) {
                Some(MoveIRExpression::Operation(
                    MoveIROperation::GreaterThanEqual(l.clone(), Box::new(new_r)),
                ))
            } else if let Some(new_l) = remove_move(statement, l) {
                Some(MoveIRExpression::Operation(
                    MoveIROperation::GreaterThanEqual(Box::new(new_l), r.clone()),
                ))
            } else {
                None
            }
        }
        MoveIROperation::LessThan(l, r) => {
            if let Some(new_r) = remove_move(statement, r) {
                Some(MoveIRExpression::Operation(MoveIROperation::LessThan(
                    l.clone(),
                    Box::new(new_r),
                )))
            } else if let Some(new_l) = remove_move(statement, l) {
                Some(MoveIRExpression::Operation(MoveIROperation::LessThan(
                    Box::new(new_l),
                    r.clone(),
                )))
            } else {
                None
            }
        }
        MoveIROperation::LessThanEqual(l, r) => {
            if let Some(new_r) = remove_move(statement, r) {
                Some(MoveIRExpression::Operation(MoveIROperation::LessThanEqual(
                    l.clone(),
                    Box::new(new_r),
                )))
            } else if let Some(new_l) = remove_move(statement, l) {
                Some(MoveIRExpression::Operation(MoveIROperation::LessThanEqual(
                    Box::new(new_l),
                    r.clone(),
                )))
            } else {
                None
            }
        }
        MoveIROperation::Equal(l, r) => {
            if let Some(new_r) = remove_move(statement, r) {
                Some(MoveIRExpression::Operation(MoveIROperation::Equal(
                    l.clone(),
                    Box::new(new_r),
                )))
            } else if let Some(new_l) = remove_move(statement, l) {
                Some(MoveIRExpression::Operation(MoveIROperation::Equal(
                    Box::new(new_l),
                    r.clone(),
                )))
            } else {
                None
            }
        }
        MoveIROperation::NotEqual(l, r) => {
            if let Some(new_r) = remove_move(statement, r) {
                Some(MoveIRExpression::Operation(MoveIROperation::NotEqual(
                    l.clone(),
                    Box::new(new_r),
                )))
            } else if let Some(new_l) = remove_move(statement, l) {
                Some(MoveIRExpression::Operation(MoveIROperation::NotEqual(
                    Box::new(new_l),
                    r.clone(),
                )))
            } else {
                None
            }
        }
        MoveIROperation::And(l, r) => {
            if let Some(new_r) = remove_move(statement, r) {
                Some(MoveIRExpression::Operation(MoveIROperation::And(
                    l.clone(),
                    Box::new(new_r),
                )))
            } else if let Some(new_l) = remove_move(statement, l) {
                Some(MoveIRExpression::Operation(MoveIROperation::And(
                    Box::new(new_l),
                    r.clone(),
                )))
            } else {
                None
            }
        }
        MoveIROperation::Or(l, r) => {
            if let Some(new_r) = remove_move(statement, r) {
                Some(MoveIRExpression::Operation(MoveIROperation::Or(
                    l.clone(),
                    Box::new(new_r),
                )))
            } else if let Some(new_l) = remove_move(statement, l) {
                Some(MoveIRExpression::Operation(MoveIROperation::Or(
                    Box::new(new_l),
                    r.clone(),
                )))
            } else {
                None
            }
        }
        MoveIROperation::Not(r) => {
            let expr = remove_move(statement, r)?;
            Some(MoveIRExpression::Operation(MoveIROperation::Not(Box::new(
                expr,
            ))))
        }
        MoveIROperation::Power(l, r) => {
            if let Some(new_r) = remove_move(statement, r) {
                Some(MoveIRExpression::Operation(MoveIROperation::Power(
                    l.clone(),
                    Box::new(new_r),
                )))
            } else if let Some(new_l) = remove_move(statement, l) {
                Some(MoveIRExpression::Operation(MoveIROperation::Power(
                    Box::new(new_l),
                    r.clone(),
                )))
            } else {
                None
            }
        }
        MoveIROperation::Access(r, s) => {
            let expr = remove_move(statement, r)?;
            Some(MoveIRExpression::Operation(MoveIROperation::Access(
                Box::new(expr),
                s.to_string(),
            )))
        }
        MoveIROperation::Dereference(r) => {
            let expr = remove_move(statement, r)?;
            Some(MoveIRExpression::Operation(MoveIROperation::Dereference(
                Box::new(expr),
            )))
        }
        MoveIROperation::MutableReference(r) => {
            let expr = remove_move(statement, r)?;
            Some(MoveIRExpression::Operation(
                MoveIROperation::MutableReference(Box::new(expr)),
            ))
        }
        MoveIROperation::Reference(r) => {
            let expr = remove_move(statement, r)?;
            Some(MoveIRExpression::Operation(MoveIROperation::Reference(
                Box::new(expr),
            )))
        }
    }
}

fn remove_move(
    statement: &Statement,
    expression: &MoveIRExpression,
) -> Option<MoveIRExpression> {
    if let Statement::Expression(Expression::BinaryExpression(be)) = statement {
        if let Expression::Identifier(variable) = &*be.rhs_expression {
            match expression {
                MoveIRExpression::Transfer(transfer) => {
                    if let MoveIRTransfer::Copy(identifier) = transfer {
                        if let MoveIRExpression::Identifier(id) = &**identifier {
                            if *id == mangle(&variable.token) {
                                return Some(MoveIRExpression::Transfer(MoveIRTransfer::Move(
                                    Box::new(MoveIRExpression::Identifier(id.to_string())),
                                )));
                            }
                        }
                    }
                }
                MoveIRExpression::Operation(op) => return remove_move_op(op, &statement),
                MoveIRExpression::FunctionCall(fc) => {
                    // iterate backwards through arguments until an argument matches the statement or we reach the end of arguments
                    let mut arguments = fc.arguments.clone();
                    for argument in arguments.iter_mut().rev() {
                        if let Some(expr) = remove_move(&statement, &argument) {
                            *argument = expr;
                            return Some(MoveIRExpression::FunctionCall(MoveIRFunctionCall {
                                identifier: fc.identifier.clone(),
                                arguments: arguments,
                            }));
                        }
                    }
                }
                MoveIRExpression::Assignment(assignment) => {
                    let expr = remove_move(&statement, &assignment.expression)?;
                    return Some(MoveIRExpression::Assignment(MoveIRAssignment {
                        identifier: assignment.identifier.clone(),
                        expression: Box::new(expr),
                    }));
                }
                MoveIRExpression::Vector(vec) => {
                    // iterate backwards through arguments until an element matches the statement or we reach the end of arguments
                    let mut elements = vec.elements.clone();
                    for element in elements.iter_mut().rev() {
                        if let Some(expr) = remove_move(&statement, &element) {
                            *element = expr;
                            return Some(MoveIRExpression::Vector(MoveIRVector {
                                elements: elements,
                                vec_type: vec.vec_type.clone()
                            }));
                        }
                    }
                }
                //Identifier, Literal, Catchable, Inline, VariableDeclaration, StructConstructor, FieldDeclaration, Noop
                _ => return None,
            }
        }
    }
    None
}

fn remove_moves(
    statements: Vec<Statement>,
    expression: MoveIRExpression,
) -> (Vec<Statement>, MoveIRExpression) {
    let mut curr_expr = expression.clone();
    let mut post_statements = Vec::new();

    for statement in statements {
        if let Some(expr) = remove_move(&statement, &expression) {
            curr_expr = expr;
        } else {
            post_statements.push(statement);
        }
    }

    return (post_statements, curr_expr);
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
