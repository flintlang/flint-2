use crate::Parser::utils::*;

pub fn parse_statements(i: Span) -> nom::IResult<Span, Vec<Statement>> {
    let (i, statements) = many0(nom::sequence::terminated(
        preceded(whitespace, parse_statement),
        whitespace,
    ))(i)?;
    Ok((i, statements))
}

fn parse_statement(i: Span) -> nom::IResult<Span, Statement> {
    alt((
        parse_return_statement,
        parse_become_statement,
        parse_emit_statement,
        parse_for_statement,
        parse_if_statement,
        parse_docatch_statement,
        map(parse_expression, |e| Statement::Expression(e)),
    ))(i)
}

fn parse_docatch_statement(i: Span) -> nom::IResult<Span, Statement> {
    let (i, _) = tag("do")(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, do_body) = parse_code_block(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, _) = tag("catch")(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, _) = tag("is")(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, error) = parse_expression(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, catch_body) = parse_code_block(i)?;
    let do_catch_statement = DoCatchStatement {
        error,
        do_body,
        catch_body,
    };
    Ok((i, Statement::DoCatchStatement(do_catch_statement)))
}

fn parse_if_statement(i: Span) -> nom::IResult<Span, Statement> {
    let (i, _) = tag("if")(i)?;
    let (i, _) = whitespace(i)?;
    let (i, condition) = parse_expression(i)?;
    let (i, _) = whitespace(i)?;
    let (i, statements) = parse_code_block(i)?;
    let (i, _) = whitespace(i)?;
    let (i, else_token) = nom::combinator::opt(tag("else"))(i)?;
    if else_token.is_some() {
        let (i, _) = whitespace(i)?;
        let (i, else_statements) = parse_code_block(i)?;
        let if_statement = IfStatement {
            condition,
            body: statements,
            else_body: else_statements,
            IfBodyScopeContext: None,
            ElseBodyScopeContext: None,
        };
        return Ok((i, Statement::IfStatement(if_statement)));
    }
    let if_statement = IfStatement {
        condition,
        body: statements,
        else_body: Vec::new(),
        IfBodyScopeContext: None,
        ElseBodyScopeContext: None,
    };
    Ok((i, Statement::IfStatement(if_statement)))
}

fn parse_for_statement(i: Span) -> nom::IResult<Span, Statement> {
    let (i, _) = tag("for")(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, variable) = parse_variable_declaration(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, _) = tag("in")(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, iterable) = parse_expression(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, statements) = parse_code_block(i)?;
    let for_statement = ForStatement {
        variable,
        iterable,
        body: statements,
        ForBodyScopeContext: None,
    };
    Ok((i, Statement::ForStatement(for_statement)))
}

pub fn parse_emit_statement(i: Span) -> nom::IResult<Span, Statement> {
    let (i, _) = tag("emit")(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, function_call) = parse_function_call(i)?;
    let emit_statement = EmitStatement { function_call };
    Ok((i, Statement::EmitStatement(emit_statement)))
}

fn parse_become_statement(i: Span) -> nom::IResult<Span, Statement> {
    let line_info = LineInfo {
        line: i.location_line(),
        offset: i.location_offset(),
    };
    let (i, _) = tag("become")(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, expression) = parse_expression(i)?;
    let become_statement = BecomeStatement {
        expression,
        line_info,
    };
    Ok((i, Statement::BecomeStatement(become_statement)))
}

fn parse_return_statement(i: Span) -> nom::IResult<Span, Statement> {
    let line_info = LineInfo {
        line: i.location_line(),
        offset: i.location_offset(),
    };
    let (i, _) = tag("return")(i)?;
    let (i, _) = nom::character::complete::space0(i)?;
    let (i, expression) = nom::combinator::opt(parse_expression)(i)?;
    let return_statement = ReturnStatement {
        expression,
        line_info,
        ..Default::default()
    };
    Ok((i, Statement::ReturnStatement(return_statement)))
}

#[cfg(test)]
mod tests {
    use nom_locate::{LocatedSpan, position};
    use sha3::Digest;

    use crate::AST::{*, BinOp::*, Literal::*};
    use crate::Parser::*;
    use crate::Parser::statements::*;

    use nom::error::ErrorKind;

    #[test]
    fn test_docatch_statement() {
        let input = LocatedSpan::new("do {return id} catch is error_type {return error}");
        let (rest, result) = parse_docatch_statement(input).expect("Error with docatch statement");
        assert_eq!(
            result,
            Statement::DoCatchStatement(DoCatchStatement {
                error: Expression::Identifier(Identifier {
                    token: String::from("error_type"),
                    enclosing_type: None,
                    line_info: LineInfo {
                        line: 1,
                        offset: 24,
                    },
                }, ),

                do_body: vec![Statement::ReturnStatement(ReturnStatement {
                    expression: Some(Expression::Identifier(Identifier {
                        token: String::from("id"),
                        enclosing_type: None,
                        line_info: LineInfo {
                            line: 1,
                            offset: 11,
                        },
                    })),

                    cleanup: vec![],
                    line_info: LineInfo { line: 1, offset: 4 },
                })],

                catch_body: vec![Statement::ReturnStatement(ReturnStatement {
                    expression: Some(Expression::Identifier(Identifier {
                        token: String::from("error"),
                        enclosing_type: None,
                        line_info: LineInfo {
                            line: 1,
                            offset: 43,
                        },
                    })),

                    cleanup: vec![],
                    line_info: LineInfo {
                        line: 1,
                        offset: 36,
                    },
                })],
            })
        );
    }

    #[test]
    fn test_if_statement() {
        let input = LocatedSpan::new("if x<5 {return x}");
        let (rest, result) = parse_if_statement(input).expect("Error parsing if statement");
        assert_eq!(
            result,
            Statement::IfStatement(IfStatement {
                condition: Expression::BinaryExpression(BinaryExpression {
                    lhs_expression: Box::new(Expression::Identifier(Identifier {
                        token: String::from("x"),
                        enclosing_type: None,
                        line_info: LineInfo { line: 1, offset: 3 },
                    })),
                    rhs_expression: Box::new(Expression::Literal(IntLiteral(5))),
                    op: LessThan,
                    line_info: LineInfo { line: 1, offset: 3 },
                }),

                body: vec![Statement::ReturnStatement(ReturnStatement {
                    expression: Some(Expression::Identifier(Identifier {
                        token: String::from("x"),
                        enclosing_type: None,
                        line_info: LineInfo {
                            line: 1,
                            offset: 15,
                        },
                    })),

                    cleanup: vec![],
                    line_info: LineInfo { line: 1, offset: 8 },
                })],

                else_body: vec![],
                IfBodyScopeContext: None,
                ElseBodyScopeContext: None,
            })
        );
    }

    #[test]
    fn test_parse_emit_statement() {
        let input = LocatedSpan::new("emit foo()");
        let (rest, result) = parse_emit_statement(input).expect("Error parsing emit statement");
        assert_eq!(
            result,
            Statement::EmitStatement(EmitStatement {
                function_call: FunctionCall {
                    identifier: Identifier {
                        token: String::from("foo"),
                        enclosing_type: None,
                        line_info: LineInfo { line: 1, offset: 5 },
                    },

                    arguments: vec![],
                    mangled_identifier: None,
                }
            })
        );
    }

    #[test]
    fn test_become_statement() {
        let input = LocatedSpan::new("become example");
        let (rest, result) = parse_become_statement(input).expect("Error parsing become statement");
        assert_eq!(
            result,
            Statement::BecomeStatement(BecomeStatement {
                expression: Expression::Identifier(Identifier {
                    token: String::from("example"),
                    enclosing_type: None,
                    line_info: LineInfo { line: 1, offset: 0 },
                }),

                line_info: LineInfo { line: 1, offset: 0 },
            })
        );
    }

    #[test]
    fn test_for_statement() {
        let input = LocatedSpan::new("for let i: Int in (1...5) {5}");
        let (rest, result) = parse_for_statement(input).expect("Error parsing for statement");
        assert_eq!(
            result,
            Statement::ForStatement(ForStatement {
                variable: VariableDeclaration {
                    declaration_token: Some(String::from("let")),
                    identifier: Identifier {
                        token: String::from("i"),
                        enclosing_type: None,
                        line_info: LineInfo { line: 1, offset: 8 },
                    },

                    variable_type: Type::Int,
                    expression: None,
                },

                iterable: Expression::RangeExpression(RangeExpression {
                    start_expression: Box::new(Expression::Literal(IntLiteral(1))),
                    end_expression: Box::new(Expression::Literal(IntLiteral(5))),
                    op: String::from("..."),
                }),

                body: vec![Statement::Expression(Expression::Literal(IntLiteral(5)))],
                ForBodyScopeContext: None,
            })
        );
    }

    #[test]
    fn test_parse_return_statement() {
        let input = LocatedSpan::new("return");
        let (rest, result) = parse_return_statement(input).expect("Error parsing return statement");
        assert_eq!(
            result,
            Statement::ReturnStatement(ReturnStatement {
                expression: None,
                cleanup: vec![],
                line_info: LineInfo { line: 1, offset: 0 },
            })
        );

        let input = LocatedSpan::new("return id");
        let (rest, result) =
            parse_return_statement(input).expect("Error parsing statement returning identifier");
        assert_eq!(
            result,
            Statement::ReturnStatement(ReturnStatement {
                expression: Some(Expression::Identifier(Identifier {
                    token: String::from("id"),
                    enclosing_type: None,
                    line_info: LineInfo { line: 1, offset: 0 },
                })),

                cleanup: vec![],
                line_info: LineInfo { line: 1, offset: 0 },
            })
        );
    }

}