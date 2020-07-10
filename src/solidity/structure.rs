use super::*;

pub struct SolidityStruct {
    pub declaration: StructDeclaration,
    pub environment: Environment,
}

impl SolidityStruct {
    pub fn generate(&self) -> String {
        let functions: Vec<FunctionDeclaration> = self
            .declaration
            .members
            .clone()
            .into_iter()
            .filter_map(|s| {
                if let StructMember::FunctionDeclaration(f) = s {
                    Some(f)
                } else {
                    None
                }
            })
            .collect();
        let functions: Vec<String> = functions
            .into_iter()
            .map(|f| {
                SolidityFunction {
                    declaration: f.clone(),
                    identifier: self.declaration.identifier.clone(),
                    environment: self.environment.clone(),
                    caller_binding: None,
                    caller_protections: vec![],
                    is_contract_function: false,
                }
                .generate(true)
            })
            .collect();

        functions.join("\n\n")
    }
}