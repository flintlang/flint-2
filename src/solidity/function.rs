use super::*;

#[derive(Clone)]
pub struct SolidityFunction {
    pub declaration: FunctionDeclaration,
    pub identifier: Identifier,
    pub environment: Environment,
    pub caller_binding: Option<Identifier>,
    pub caller_protections: Vec<CallerProtection>,
    pub is_contract_function: bool,
}

impl SolidityFunction {
    pub fn has_any_caller(&self) -> bool {
        let callers = self.caller_protections.clone();
        for caller in callers {
            if caller.is_any() {
                return true;
            }
        }
        false
    }

    pub fn generate(&self, returns: bool) -> String {
        let returns = self.declaration.head.result_type.is_some() && returns;

        let scope = self.declaration.scope_context.clone();
        let scope = scope.unwrap_or(Default::default());
        let mut function_context = FunctionContext {
            environment: self.environment.clone(),
            scope_context: scope,
            in_struct_function: !self.is_contract_function,
            block_stack: vec![YulBlock { statements: vec![] }],
            enclosing_type: self.identifier.token.clone(),
            counter: 0,
        };
        let parameters = self.declaration.head.parameters.clone();
        let parameters: Vec<String> = parameters
            .into_iter()
            .map(|p| {
                SolidityIdentifier {
                    identifier: p.identifier.clone(),
                    is_lvalue: false,
                }
                .generate(&mut function_context)
            })
            .map(|p| format!("{}", p))
            .collect();
        let parameters = parameters.join(", ");
        let return_var = if returns {
            "-> ret".to_string()
        } else {
            "".to_string()
        };
        let name = self.declaration.mangled_identifier.clone();
        let name = name.unwrap_or_default();
        let signature = format!(
            "{name}({parameters}) {return_var}",
            name = name,
            parameters = parameters,
            return_var = return_var
        );

        let scope = self.declaration.scope_context.clone();
        let scope = scope.unwrap_or(Default::default());

        let mut function_context = FunctionContext {
            environment: self.environment.clone(),
            enclosing_type: self.identifier.token.clone(),
            block_stack: vec![YulBlock { statements: vec![] }],
            scope_context: scope,
            in_struct_function: !self.is_contract_function,
            counter: 0,
        };

        let caller_binding = if let Some(ref binding) = self.caller_binding {
            let binding = mangle(&binding.token);
            format!("let {binding} := caller()\n", binding = binding)
        } else {
            "".to_string()
        };

        let mut statements = self.declaration.body.clone();
        let mut emit_last_brace = false;
        while !statements.is_empty() {
            let statement = statements.remove(0);
            let yul_statement = SolidityStatement {
                statement: statement.clone(),
            }
            .generate(&mut function_context);
            function_context.emit(yul_statement);
            if let Statement::IfStatement(i) = statement {
                if i.ends_with_return() {
                    let else_body = i.else_body.clone();
                    if else_body.is_empty() {
                        let st = YulStatement::Inline("default {".to_string());
                        emit_last_brace = true;
                        function_context.emit(st);
                    }
                }
            }
        }
        if emit_last_brace {
            let st = YulStatement::Inline("}".to_string());
            function_context.emit(st);
        }
        let body = function_context.generate();
        let body = format!("{binding} {body}", binding = caller_binding, body = body);
        format!(
            "function {signature} {{ \n {body} \n }}",
            signature = signature,
            body = body
        )
    }

    pub fn mangled_signature(&self) -> String {
        let name = self.declaration.head.identifier.token.clone();
        let parameters = self.declaration.head.parameters.clone();
        let parameters: Vec<String> = parameters
            .into_iter()
            .map(|p| SolidityIRType::map_to_solidity_type(p.type_assignment).generate())
            .collect();
        let parameters = parameters.join(",");

        format!("{name}({params})", name = name, params = parameters)
    }
}

pub struct FunctionContext {
    pub environment: Environment,
    pub scope_context: ScopeContext,
    pub in_struct_function: bool,
    pub block_stack: Vec<YulBlock>,
    pub enclosing_type: String,
    pub counter: u64,
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

    pub fn emit(&mut self, statement: YulStatement) {
        let count = self.block_stack.len();
        let block = self.block_stack.get_mut(count - 1);
        block.unwrap().statements.push(statement);
    }

    pub fn push_block(&mut self) -> usize {
        self.block_stack.push(YulBlock { statements: vec![] });
        self.block_stack.len()
    }

    pub fn pop_block(&mut self) -> YulBlock {
        self.block_stack.pop().unwrap()
    }

    pub fn with_new_block(&mut self, count: usize) -> YulBlock {
        while self.block_stack.len() != count {
            let block = YulStatement::Block(self.pop_block());
            self.emit(block);
        }
        self.pop_block()
    }

    pub fn fresh_variable(&mut self) -> String {
        let name = format!("$temp{}", self.counter);
        self.counter += 1;
        name
    }
}
