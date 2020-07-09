use crate::moveir::*;

pub struct MoveAsset {
    pub declaration: AssetDeclaration,
    pub environment: Environment,
}

impl MoveAsset {
    pub fn generate(&self) -> String {
        let function_context = FunctionContext {
            environment: self.environment.clone(),
            scope_context: Default::default(),
            enclosing_type: self.declaration.identifier.token.clone(),
            block_stack: vec![MoveIRBlock { statements: vec![] }],
            in_struct_function: false,
            is_constructor: false,
        };

        let members: Vec<MoveIRExpression> = self
            .declaration
            .members
            .clone()
            .into_iter()
            .filter_map(|s| match s {
                AssetMember::VariableDeclaration(v) => {
                    Some(MoveFieldDeclaration { declaration: v }.generate(&function_context))
                }
                _ => None,
            })
            .collect();
        let members: Vec<String> = members.into_iter().map(|e| format!("{}", e)).collect();
        let members = members.join(",\n");
        let result = format!(
            "resource {name} {{ \n {members} \n }}",
            name = self.declaration.identifier.token,
            members = members
        );
        result
    }

    pub fn generate_all_functions(&self) -> String {
        format!(
            "{initialisers} \n\n {functions}",
            initialisers = self.generate_initialisers(),
            functions = self.generate_functions()
        )
    }

    pub fn generate_initialisers(&self) -> String {
        let initialisers: Vec<SpecialDeclaration> = self
            .declaration
            .members
            .clone()
            .into_iter()
            .filter_map(|m| {
                if let AssetMember::SpecialDeclaration(s) = m {
                    if s.is_init() {
                        return Some(s);
                    }
                }
                None
            })
            .collect();
        let initialisers: Vec<String> = initialisers
            .into_iter()
            .map(|i| {
                MoveStructInitialiser {
                    declaration: i.clone(),
                    identifier: self.declaration.identifier.clone(),
                    environment: self.environment.clone(),
                    properties: self.declaration.get_variable_declarations(),
                }
                .generate()
            })
            .collect();
        initialisers.join("\n\n")
    }

    pub fn generate_functions(&self) -> String {
        let functions: Vec<FunctionDeclaration> = self
            .declaration
            .members
            .clone()
            .into_iter()
            .filter_map(|m| {
                if let AssetMember::FunctionDeclaration(f) = m {
                    return Some(f);
                }
                None
            })
            .collect();
        let functions: Vec<String> = functions
            .into_iter()
            .map(|f| {
                MoveFunction {
                    function_declaration: f.clone(),
                    environment: self.environment.clone(),
                    is_contract_function: false,
                    enclosing_type: self.declaration.identifier.clone(),
                }
                .generate(true)
            })
            .collect();
        functions.join("\n\n")
    }
}