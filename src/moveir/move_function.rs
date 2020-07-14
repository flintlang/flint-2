use crate::moveir::*;

#[derive(Debug)]
pub struct MoveFunction {
    pub function_declaration: FunctionDeclaration,
    pub environment: Environment,
    pub is_contract_function: bool,
    pub enclosing_type: Identifier,
}

impl MoveFunction {
    pub fn generate(&self, _return: bool) -> String {
        let scope = self.function_declaration.scope_context.clone();
        let scope = scope.unwrap_or(Default::default());

        let function_context = FunctionContext {
            environment: self.environment.clone(),
            scope_context: scope,
            enclosing_type: self.enclosing_type.token.clone(),
            block_stack: vec![MoveIRBlock { statements: vec![] }],
            in_struct_function: !self.is_contract_function,
            is_constructor: false,
        };

        let modifiers: Vec<String> = self
            .function_declaration
            .head
            .modifiers
            .clone()
            .into_iter()
            .filter(|s| s.eq("public"))
            .collect();
        let modifiers = modifiers.join(",");
        let name = self.function_declaration.head.identifier.token.clone();
        let name = self
            .function_declaration
            .mangled_identifier
            .as_ref()
            .unwrap_or(&name);
        let parameter_move_types: Vec<MoveType> = self
            .function_declaration
            .head
            .parameters
            .clone()
            .into_iter()
            .map(|p| MoveType::move_type(p.type_assignment, Option::from(self.environment.clone())))
            .collect();
        let parameters: Vec<MoveIRExpression> = self
            .function_declaration
            .head
            .parameters
            .clone()
            .into_iter()
            .map(|p| {
                MoveIdentifier {
                    identifier: p.identifier.clone(),
                    position: MovePosition::Left,
                }
                .generate(&function_context, false, false)
            })
            .collect();
        let parameters: Vec<String> = parameters
            .into_iter()
            .zip(parameter_move_types.into_iter())
            .map(|(p, t)| {
                format!(
                    "{parameter}: {ir_type}",
                    parameter = p,
                    ir_type = t.generate(&function_context)
                )
            })
            .collect();
        let parameters = parameters.join(", ");

        let result_type = if self.function_declaration.get_result_type().is_some() && _return {
            let result = self.function_declaration.get_result_type().clone();
            let result = result.unwrap();
            let result = MoveType::move_type(result, Option::from(self.environment.clone()));
            format!("{}", result.generate(&function_context))
        } else {
            "".to_string()
        };
        let tags = self.function_declaration.tags.clone();
        let tags = tags.join("");

        let scope = self.function_declaration.scope_context.clone();

        let mut scope = scope.unwrap_or(Default::default());

        let variables = self.function_declaration.body.clone();
        let variables: Vec<Expression> = variables
            .into_iter()
            .filter_map(|v| {
                if let Statement::Expression(e) = v {
                    Some(e)
                } else {
                    None
                }
            })
            .collect();

        let mut variables: Vec<VariableDeclaration> = variables
            .into_iter()
            .filter_map(|v| {
                if let Expression::VariableDeclaration(e) = v {
                    Some(e)
                } else {
                    None
                }
            })
            .collect();

        let mut all_variables = scope.local_variables.clone();
        all_variables.append(&mut variables);

        scope.local_variables = all_variables;
        let mut function_context = FunctionContext {
            environment: self.environment.clone(),
            enclosing_type: self.enclosing_type.token.clone(),
            block_stack: vec![MoveIRBlock { statements: vec![] }],
            scope_context: scope,
            in_struct_function: !self.is_contract_function,
            is_constructor: false,
        };
        let statements = self.function_declaration.body.clone();
        let mut statements: Vec<MoveStatement> = statements
            .into_iter()
            .map(|s| MoveStatement { statement: s })
            .collect();
        while !statements.is_empty() {
            let statement = statements.remove(0);
            let statement = statement.generate(&mut function_context);
            function_context.emit(statement);
        }

        let body = function_context.generate();
        if result_type.is_empty() {
            let result = format!(
                " {modifiers} {name} ({parameters}) {tags} {{ \n {body} \n }}",
                modifiers = modifiers,
                name = name,
                parameters = parameters,
                tags = tags,
                body = body
            );
            return result;
        }

        let result = format!(
            " {modifiers} {name} ({parameters}): {result} {tags} {{ \n {body} \n }}",
            modifiers = modifiers,
            name = name,
            parameters = parameters,
            result = result_type,
            tags = tags,
            body = body
        );
        result
    }
}

#[derive(Debug, Default, Clone)]
pub struct FunctionContext {
    pub environment: Environment,
    pub scope_context: ScopeContext,
    pub enclosing_type: String,
    pub block_stack: Vec<MoveIRBlock>,
    pub in_struct_function: bool,
    pub is_constructor: bool,
}

impl FunctionContext {
    pub fn generate(&mut self) -> String {
        let block = self.block_stack.last();
        if !self.block_stack.is_empty() {
            let statements = block.unwrap().statements.clone();
            let statements: Vec<String> = statements
                .into_iter()
                .map(|s| format!("{s}", s = s))
                .collect();
            return statements.join("\n");
        }

        String::from("")
    }

    pub fn emit(&mut self, statement: MoveIRStatement) {
        let count = self.block_stack.len();
        let block = self.block_stack.get_mut(count - 1);
        block.unwrap().statements.push(statement);
    }

    pub fn with_new_block(&mut self, count: usize) -> MoveIRBlock {
        while self.block_stack.len() != count {
            let block = MoveIRStatement::Block(self.pop_block());
            self.emit(block);
        }
        self.pop_block()
    }

    pub fn push_block(&mut self) -> usize {
        self.block_stack.push(MoveIRBlock { statements: vec![] });
        self.block_stack.len()
    }

    pub fn pop_block(&mut self) -> MoveIRBlock {
        self.block_stack.pop().unwrap()
    }
    pub fn emit_release_references(&mut self) {
        let references: Vec<Identifier> = self
            .scope_context
            .parameters
            .clone()
            .into_iter()
            .filter(|i| i.is_inout())
            .map(|p| p.identifier)
            .collect();
        for reference in references {
            let expression = MoveIdentifier {
                identifier: reference,
                position: Default::default(),
            }
            .generate(self, true, false);
            self.emit(MoveIRStatement::Inline(format!("_ = {}", expression)))
        }
    }

    pub fn self_type(&self) -> Type {
        let result = self.scope_context.type_for("self".to_string());
        if result.is_some() {
            result.unwrap()
        } else {
            self.environment.get_expression_type(
                Expression::SelfExpression,
                &self.enclosing_type,
                vec![],
                vec![],
                self.scope_context.clone(),
            )
        }
    }
}
