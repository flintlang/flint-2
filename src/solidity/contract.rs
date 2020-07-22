use super::*;

#[derive(Clone)]
pub struct SolidityContract {
    pub declaration: ContractDeclaration,
    pub behaviour_declarations: Vec<ContractBehaviourDeclaration>,
    pub struct_declarations: Vec<StructDeclaration>,
    pub environment: Environment,
}

impl SolidityContract {
    pub fn generate(&self) -> String {
        //////////////////////// FUNCTIONS
        let mut functions: Vec<SolidityFunction> = vec![];
        for declarations in self.behaviour_declarations.clone() {
            for function in declarations.members.clone() {
                match function {
                    ContractBehaviourMember::FunctionDeclaration(f) => {
                        functions.push(SolidityFunction {
                            declaration: f.clone(),
                            identifier: self.declaration.identifier.clone(),
                            environment: self.environment.clone(),
                            caller_binding: declarations.caller_binding.clone(),
                            caller_protections: declarations.caller_protections.clone(),
                            is_contract_function: !declarations.caller_protections.is_empty(),
                        })
                    }
                    _ => {}
                }
            }
        }

        let functions_code: Vec<String> = functions
            .clone()
            .into_iter()
            .map(|f| f.generate(true))
            .collect();

        let functions_code = functions_code.join("\n");

        let wrapper_functions: Vec<String> = functions
            .clone()
            .into_iter()
            .filter(|f| !f.has_any_caller())
            .map(|f| {
                SolidityWrapperFunction { function: f }.generate(&self.declaration.identifier.token)
            })
            .collect();

        let _wrapper_functions = wrapper_functions.join("\n\n");

        let public_function: Vec<SolidityFunction> = functions
            .clone()
            .into_iter()
            .filter(|f| f.declaration.is_public())
            .collect();
        let selector = SolidityFunctionSelector {
            fallback: None,
            functions: public_function.clone(),
            enclosing: self.declaration.identifier.clone(),
            environment: self.environment.clone(),
        };
        let selector = selector.generate();

        let struct_declarations: Vec<String> = self
            .struct_declarations
            .clone()
            .into_iter()
            .map(|s| {
                SolidityStruct {
                    declaration: s.clone(),
                    environment: self.environment.clone(),
                }
                .generate()
            })
            .collect();

        let structs = struct_declarations.join("\n\n");

        let runtime = SolidityRuntimeFunction::get_all_functions();
        let runtime = runtime.join("\n\n");

        let mut contract_behaviour_declaration = None;
        let mut initialiser_declaration = None;
        for declarations in self.behaviour_declarations.clone() {
            for member in declarations.members.clone() {
                if let ContractBehaviourMember::SpecialDeclaration(s) = member {
                    if s.is_init() && s.is_public() {
                        contract_behaviour_declaration = Some(declarations.clone());
                        initialiser_declaration = Some(s.clone());
                    }
                }
            }
        }

        let initialiser_declaration = initialiser_declaration.unwrap();
        let contract_behaviour_declaration = contract_behaviour_declaration.unwrap();

        let parameter_sizes: Vec<u64> = initialiser_declaration
            .head
            .parameters
            .clone()
            .into_iter()
            .map(|p| self.environment.type_size(p.type_assignment))
            .collect();
        println!("{:?}", parameter_sizes);

        let mut offsets = parameter_sizes.clone();
        offsets.reverse();
        let mut elem_acc = 0;
        let mut list_acc = vec![];
        for offset in &offsets {
            elem_acc = elem_acc + offset * 32;
            list_acc.push(elem_acc);
        }
        offsets.reverse();
        let parameter_sizes: Vec<(u64, u64)> = offsets.into_iter().zip(parameter_sizes).collect();

        let mut scope = ScopeContext {
            parameters: vec![],
            local_variables: vec![],
            counter: 0,
        };

        if let Some(ref caller_binding) = contract_behaviour_declaration.caller_binding {
            let variable_declaration = VariableDeclaration {
                declaration_token: None,
                identifier: caller_binding.clone(),
                variable_type: Type::Address,
                expression: None,
            };
            scope.local_variables.push(variable_declaration);
        }

        scope.parameters = initialiser_declaration.head.parameters.clone();

        let mut function_context = FunctionContext {
            environment: self.environment.clone(),
            scope_context: scope,
            in_struct_function: false, //Inside Contract
            block_stack: vec![YulBlock { statements: vec![] }],
            enclosing_type: self.declaration.identifier.token.clone(),
            counter: 0,
        };

        let parameter_names: Vec<YulExpression> = initialiser_declaration
            .head
            .parameters
            .clone()
            .into_iter()
            .map(|p| {
                SolidityIdentifier {
                    identifier: p.identifier.clone(),
                    is_lvalue: false,
                }
                .generate(&mut function_context)
            })
            .collect();
        let parameter_names: Vec<String> = parameter_names
            .into_iter()
            .map(|p| format!("{}", p))
            .collect();

        let parameter_binding: Vec<String> = parameter_names
            .into_iter()
            .zip(parameter_sizes)
            .map(|(k, (v1, v2))| {
                format!(
                    "codecopy(0x0), sub(codesize, {offset}), {size}) \n let {param} := mload(0)",
                    offset = v1,
                    size = v2 * 32,
                    param = k
                )
            })
            .collect();

        let parameter_binding = parameter_binding.join("\n");

        let scope = initialiser_declaration.scope_context.clone();

        let mut function_context = FunctionContext {
            environment: self.environment.clone(),
            enclosing_type: self.declaration.identifier.token.clone(),
            block_stack: vec![YulBlock { statements: vec![] }],
            scope_context: scope,
            in_struct_function: false,
            counter: 0,
        };

        let caller_binding =
            if let Some(ref binding) = contract_behaviour_declaration.caller_binding {
                let binding = mangle(&binding.token);
                format!("let {binding} := caller()\n", binding = binding)
            } else {
                "".to_string()
            };

        let mut statements = initialiser_declaration.body.clone();
        while !statements.is_empty() {
            let statement = statements.remove(0);
            let yul_statement = SolidityStatement {
                statement: statement.clone(),
            }
            .generate(&mut function_context);
            function_context.emit(yul_statement);
            if let Statement::IfStatement(_) = statement {}
        }
        let body = function_context.generate();
        let body = format!("{binding} {body}", binding = caller_binding, body = body);

        let public_initialiser = format!(
            "init() \n\n function init() {{ \n {params} \n {body} \n }}",
            params = parameter_binding,
            body = body
        );

        let parameters: Vec<String> = initialiser_declaration
            .head
            .parameters
            .clone()
            .into_iter()
            .map(|_| format!(""))
            .collect();
        let parameters = parameters.join(", ");
        let contract_initialiser = format!(
            "constructor({params}) public {{ \n\n assembly {{ \n mstore(0x40, 0x60) \n\n {init} \n \
             /////////////////////////////// \n \
             //STRUCT FUNCTIONS \n  \
             /////////////////////////////// \n \
             {structs} \n\n \
             /////////////////////////////// \n \
             //RUNTIME FUNCTIONS \n  \
             /////////////////////////////// \n \
             {runtime} \n \
             }} \n }}",
            params = parameters,
            init = public_initialiser,
            structs = structs,
            runtime = runtime
        );

        return format!(
            "pragma solidity ^0.5.12; \n \
            contract {name} {{ \n\n \
                {init} \n\n \
                function () external payable {{ \n \
                    assembly {{ \n
                    mstore(0x40, 0x60) \n\n \
                    /////////////////////////////// \n \
                    //SELECTOR \n  \
                    /////////////////////////////// \n \
                    {selector} \n\n \
                    /////////////////////////////// \n \
                    //USER DEFINED FUNCTIONS \n  \
                    /////////////////////////////// \n \
                    {functions} \n\n \
                    /////////////////////////////// \n \
                    //WRAPPER FUNCTIONS \n  \
                    /////////////////////////////// \n \
                    /////////////////////////////// \n \
                    //STRUCT FUNCTIONS \n  \
                    /////////////////////////////// \n \
                    {structs} \n\n \
                    /////////////////////////////// \n \
                    //RUNTIME FUNCTIONS \n  \
                    /////////////////////////////// \n \
                    {runtime} \n \
                }} \n \
                }} \n \
             }}",
            name = self.declaration.identifier.token,
            init = contract_initialiser,
            functions = functions_code,
            structs = structs,
            runtime = runtime,
            selector = selector,
        );
    }
}
