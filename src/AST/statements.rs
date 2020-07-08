use crate::AST::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Statement {
    ReturnStatement(ReturnStatement),
    Expression(Expression),
    BecomeStatement(BecomeStatement),
    EmitStatement(EmitStatement),
    ForStatement(ForStatement),
    IfStatement(IfStatement),
    DoCatchStatement(DoCatchStatement),
}

impl Statement {
    pub fn is_expression(&self) -> bool {
        match self {
            Statement::Expression(_) => true,
            _ => false,
        }
    }
}

impl Visitable for Statement {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_statement(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = match self {
            Statement::ReturnStatement(r) => r.visit(v, ctx),
            Statement::Expression(e) => e.visit(v, ctx),
            Statement::BecomeStatement(b) => b.visit(v, ctx),
            Statement::EmitStatement(e) => e.visit(v, ctx),
            Statement::ForStatement(f) => f.visit(v, ctx),
            Statement::IfStatement(i) => i.visit(v, ctx),
            Statement::DoCatchStatement(d) => d.visit(v, ctx),
        };
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let result = v.finish_statement(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
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
        v.start_if_statement(self, ctx);

        ctx.in_if_condition = true;

        self.condition.visit(v, ctx);

        ctx.in_if_condition = false;

        let pre_statements = ctx.pre_statements.clone();
        let post_statements = ctx.post_statements.clone();
        let scope = ctx.scope_context.clone();
        let block = ctx.block_context.clone();

        let blocks_scope = if self.if_body_scope_context.is_some() {
            let temp = self.if_body_scope_context.clone();
            temp.unwrap()
        } else {
            let temp = ctx.scope_context.clone();
            temp.unwrap()
        };
        let block_context = BlockContext {
            scope_context: blocks_scope,
        };

        ctx.block_context = Some(block_context);
        let mut statements: Vec<Vec<Statement>> = vec![];
        for statement in &mut self.body {
            ctx.pre_statements = vec![];
            ctx.post_statements = vec![];
            statement.visit(v, ctx);
            statements.push(ctx.pre_statements.clone());
            statements.push(ctx.post_statements.clone());
        }

        let body = self.body.clone();
        let mut counter = 1;
        for statement in body {
            statements.insert(counter, vec![statement]);
            counter = counter + 3;
        }

        let statements: Vec<Statement> = statements.into_iter().flatten().collect();

        self.body = statements;

        if self.if_body_scope_context.is_none() {
            self.if_body_scope_context = ctx.scope_context.clone();
        } else if ctx.block_context.is_some() {
            let block = ctx.block_context.clone();
            let block = block.unwrap();
            self.if_body_scope_context = Option::from(block.scope_context.clone());
        }

        if scope.is_some() {
            let temp_scope = scope.clone();
            let mut temp_scope = temp_scope.unwrap();

            temp_scope.counter = if ctx.scope_context().is_some() {
                let ctx_scope = ctx.scope_context.clone();
                let ctx_scope = ctx_scope.unwrap();

                temp_scope.counter + ctx_scope.local_variables.len() as u64
            } else {
                temp_scope.counter + 1
            };

            temp_scope.counter = if ctx.block_context.is_some() {
                let ctx_block = ctx.block_context.clone();
                let ctx_scope = ctx_block.unwrap();
                let ctx_scope = ctx_scope.scope_context;
                temp_scope.counter + ctx_scope.local_variables.len() as u64
            } else {
                temp_scope.counter + 1
            };

            ctx.scope_context = Option::from(temp_scope);
        }

        let blocks_scope = if self.else_body_scope_context.is_some() {
            let temp = self.else_body_scope_context.clone();
            temp.unwrap()
        } else {
            let temp = ctx.scope_context.clone();
            temp.unwrap()
        };
        let block_context = BlockContext {
            scope_context: blocks_scope,
        };

        ctx.block_context = Some(block_context);

        let mut statements: Vec<Vec<Statement>> = vec![];
        for statement in &mut self.else_body {
            ctx.pre_statements = vec![];
            ctx.post_statements = vec![];
            statement.visit(v, ctx);
            statements.push(ctx.pre_statements.clone());
            statements.push(ctx.post_statements.clone());
        }

        let body = self.else_body.clone();
        let mut counter = 1;
        for statement in body {
            statements.insert(counter, vec![statement]);
            counter = counter + 3;
        }

        let statements: Vec<Statement> = statements.into_iter().flatten().collect();

        self.else_body = statements;

        if self.else_body_scope_context.is_none() {
            self.else_body_scope_context = ctx.scope_context.clone();
        } else if ctx.block_context.is_some() {
            let block = ctx.block_context.clone();
            let block = block.unwrap();
            self.else_body_scope_context = Option::from(block.scope_context.clone());
        }

        ctx.scope_context = scope;
        ctx.block_context = block;
        ctx.pre_statements = pre_statements;
        ctx.post_statements = post_statements;

        let result = v.finish_if_statement(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
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
        v.start_for_statement(self, ctx);

        self.variable.visit(v, ctx);

        self.iterable.visit(v, ctx);

        let scopeContext = ctx.scope_context.clone();
        let blockContext = ctx.block_context.clone();
        let PreStatements = ctx.pre_statements.clone();
        let PostStatements = ctx.post_statements.clone();

        let blocks_scope = if self.for_body_scope_context.is_some() {
            let temp = self.for_body_scope_context.clone();
            temp.unwrap()
        } else {
            let temp = ctx.scope_context.clone();
            temp.unwrap()
        };
        let block_context = BlockContext {
            scope_context: blocks_scope,
        };
        ctx.block_context = Some(block_context);

        let mut statements: Vec<Vec<Statement>> = vec![];
        for statement in &mut self.body {
            ctx.pre_statements = vec![];
            ctx.post_statements = vec![];
            statement.visit(v, ctx);
            statements.push(ctx.pre_statements.clone());
            statements.push(ctx.post_statements.clone());
        }

        let body = self.body.clone();
        let mut counter = 1;
        for statement in body {
            statements.insert(counter, vec![statement]);
            counter = counter + 3;
        }

        let statements: Vec<Statement> = statements.into_iter().flatten().collect();

        self.body = statements;

        ctx.scope_context = scopeContext;
        ctx.block_context = blockContext;
        ctx.pre_statements = PreStatements;
        ctx.post_statements = PostStatements;

        let result = v.finish_for_statement(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmitStatement {
    pub function_call: FunctionCall,
}

impl Visitable for EmitStatement {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        let result = v.start_emit_statement(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }

        ctx.in_emit = true;
        let result = self.function_call.visit(v, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        ctx.in_emit = false;

        let result = v.finish_emit_statement(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BecomeStatement {
    pub expression: Expression,
    pub line_info: LineInfo,
}

impl Visitable for BecomeStatement {
    fn visit(&mut self, v: &mut dyn Visitor, ctx: &mut Context) -> VResult {
        ctx.in_become = true;
        self.expression.visit(v, ctx);
        ctx.in_become = false;
        Ok(())
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
        let result = v.start_return_statement(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        if self.expression.is_some() {
            let expression = self.expression.clone();
            let mut expression = expression.unwrap();
            let result = expression.visit(v, ctx);
            match result {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
            self.expression = Option::from(expression);
        }

        let result = v.finish_return_statement(self, ctx);
        match result {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }
}