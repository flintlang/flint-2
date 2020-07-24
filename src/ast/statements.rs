use crate::ast::*;
use crate::context::{BlockContext, Context, ScopeContext};
use crate::visitor::Visitor;

#[derive(Clone, Debug, PartialEq)]
pub enum Statement {
    ReturnStatement(ReturnStatement),
    Expression(Expression),
    BecomeStatement(BecomeStatement),
    EmitStatement(EmitStatement),
    ForStatement(Box<ForStatement>), // Boxed as rare and large
    IfStatement(IfStatement),
    DoCatchStatement(DoCatchStatement),
    Assertion(Assertion),
}

impl Statement {
    #[allow(dead_code)]
    pub fn is_expression(&self) -> bool {
        match self {
            Statement::Expression(_) => true,
            _ => false,
        }
    }
}

impl Visitable for Statement {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_statement(self, ctx)?;
        match self {
            Statement::ReturnStatement(r) => r.visit(v, ctx),
            Statement::Expression(e) => e.visit(v, ctx),
            Statement::BecomeStatement(b) => b.visit(v, ctx),
            Statement::EmitStatement(e) => e.visit(v, ctx),
            Statement::ForStatement(f) => f.visit(v, ctx),
            Statement::IfStatement(i) => i.visit(v, ctx),
            Statement::DoCatchStatement(d) => d.visit(v, ctx),
            Statement::Assertion(a) => a.visit(v, ctx),
        }?;
        v.finish_statement(self, ctx)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DoCatchStatement {
    pub error: Expression,
    pub do_body: Vec<Statement>,
    pub catch_body: Vec<Statement>,
}

impl Visitable for DoCatchStatement {
    fn visit(&mut self, _v: &mut dyn Visitor, _ctx: &mut Context) -> VResult {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IfStatement {
    pub condition: Expression,
    pub body: Vec<Statement>,
    pub else_body: Vec<Statement>,
    pub if_body_scope_context: Option<ScopeContext>,
    pub else_body_scope_context: Option<ScopeContext>,
}

impl IfStatement {
    pub fn ends_with_return(&self) -> bool {
        let body = self.body.clone();
        for b in body {
            if let Statement::ReturnStatement(_) = b {
                return true;
            }
        }
        false
    }
}

impl Visitable for IfStatement {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_if_statement(self, ctx)?;

        ctx.in_if_condition = true;

        self.condition.visit(v, ctx)?;

        ctx.in_if_condition = false;

        let pre_statements = ctx.pre_statements.clone();
        let post_statements = ctx.post_statements.clone();
        let scope = ctx.scope_context.clone();
        let block = ctx.block_context.clone();

        let blocks_scope = self
            .if_body_scope_context
            .clone()
            .or_else(|| ctx.scope_context.clone())
            .unwrap();
        let block_context = BlockContext {
            scope_context: blocks_scope,
        };

        ctx.block_context = Some(block_context);
        let mut statements: Vec<Vec<Statement>> = vec![];
        for statement in &mut self.body {
            ctx.pre_statements = vec![];
            ctx.post_statements = vec![];
            statement.visit(v, ctx)?;
            statements.push(ctx.pre_statements.clone());
            statements.push(ctx.post_statements.clone());
        }

        for (statement, counter) in self.body.iter().zip((1..).step_by(3)) {
            statements.insert(counter, vec![statement.clone()]);
        }

        let statements: Vec<Statement> = statements.into_iter().flatten().collect();

        self.body = statements;

        if self.if_body_scope_context.is_none() {
            self.if_body_scope_context = ctx.scope_context.clone();
        } else if let Some(ref block) = ctx.block_context {
            self.if_body_scope_context = Option::from(block.scope_context.clone());
        }

        let ctx_scope = ctx.scope_context().cloned();
        if let Some(ref mut scope) = ctx.scope_context {
            scope.counter += if let Some(ctx_scope) = ctx_scope {
                ctx_scope.local_variables.len() as u64
            } else {
                1
            };

            scope.counter += if let Some(ref ctx_scope) = ctx.block_context {
                let ctx_scope = &ctx_scope.scope_context;
                ctx_scope.local_variables.len() as u64
            } else {
                1
            };
        }

        let block_scope = self
            .else_body_scope_context
            .as_ref()
            .or_else(|| ctx.scope_context.as_ref())
            .unwrap();
        let block_context = BlockContext {
            scope_context: block_scope.clone(),
        };

        ctx.block_context = Some(block_context);

        let mut statements: Vec<Vec<Statement>> = vec![];
        for statement in &mut self.else_body {
            ctx.pre_statements = vec![];
            ctx.post_statements = vec![];
            statement.visit(v, ctx)?;
            statements.push(ctx.pre_statements.clone());
            statements.push(ctx.post_statements.clone());
        }

        for (statement, counter) in self.else_body.iter().zip((1..).step_by(3)) {
            statements.insert(counter, vec![statement.clone()]);
        }

        let statements: Vec<Statement> = statements.into_iter().flatten().collect();

        self.else_body = statements;

        self.else_body_scope_context = if let Some(ref block_ctx) = ctx.block_context {
            Some(block_ctx.scope_context.clone())
        } else {
            ctx.scope_context.clone()
        };

        ctx.scope_context = scope;
        ctx.block_context = block;
        ctx.pre_statements = pre_statements;
        ctx.post_statements = post_statements;

        v.finish_if_statement(self, ctx)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ForStatement {
    pub variable: VariableDeclaration,
    pub iterable: Expression,
    pub body: Vec<Statement>,
    pub for_body_scope_context: Option<ScopeContext>,
}

impl Visitable for ForStatement {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_for_statement(self, ctx)?;

        self.variable.visit(v, ctx)?;

        self.iterable.visit(v, ctx)?;

        let initial_scope_context = ctx.scope_context.clone();
        let initial_block_context = ctx.block_context.clone();
        let initial_pre_statements = ctx.pre_statements.clone();
        let initial_post_statements = ctx.post_statements.clone();

        let blocks_scope = self
            .for_body_scope_context
            .as_ref()
            .or_else(|| ctx.scope_context());
        let block_context = BlockContext {
            scope_context: blocks_scope.unwrap().clone(),
        };
        ctx.block_context = Some(block_context);

        let mut statements: Vec<Vec<Statement>> = vec![];

        for statement in &mut self.body {
            ctx.pre_statements = vec![];
            ctx.post_statements = vec![];
            statement.visit(v, ctx)?;
            statements.push(ctx.pre_statements.clone());
            statements.push(ctx.post_statements.clone());
        }

        for (statement, counter) in self.body.iter().zip((1..).step_by(3)) {
            statements.insert(counter, vec![statement.clone()]);
        }

        let statements: Vec<Statement> = statements.into_iter().flatten().collect();

        self.body = statements;

        ctx.scope_context = initial_scope_context;
        ctx.block_context = initial_block_context;
        ctx.pre_statements = initial_pre_statements;
        ctx.post_statements = initial_post_statements;

        v.finish_for_statement(self, ctx)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmitStatement {
    pub function_call: FunctionCall,
}

impl Visitable for EmitStatement {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_emit_statement(self, ctx)?;

        ctx.in_emit = true;
        self.function_call.visit(v, ctx)?;
        ctx.in_emit = false;

        v.finish_emit_statement(self, ctx)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BecomeStatement {
    pub state: TypeState,
    pub line_info: LineInfo,
}

impl Visitable for BecomeStatement {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        ctx.in_become = true;
        self.state.visit(v, ctx)?;
        if let Some(context) = ctx.contract_behaviour_declaration_context.clone() {
            let contract_name = &context.identifier.token;
            if ctx
                .environment
                .contains_type_state(contract_name, &self.state)
            {
                ctx.environment
                    .set_contract_state(contract_name, self.state.clone());
                ctx.in_become = false;
                return Ok(());
            }
        }
        Err(Box::from(format!(
            "Undeclared type state {} in become statement at line {}",
            self.state.identifier.token, self.line_info.line
        )))
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ReturnStatement {
    pub expression: Option<Expression>,
    pub cleanup: Vec<Statement>,
    pub line_info: LineInfo,
}

impl Visitable for ReturnStatement {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_return_statement(self, ctx)?;
        if let Some(ref mut expression) = self.expression {
            expression.visit(v, ctx)?;
        }

        v.finish_return_statement(self, ctx)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assertion {
    pub expression: Expression,
    pub line_info: LineInfo,
}

impl Visitable for Assertion {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        v.start_assertion(self, ctx)?;
        self.expression.visit(v, ctx)?;
        v.finish_assertion(self, ctx)?;
        Ok(())
    }
}
