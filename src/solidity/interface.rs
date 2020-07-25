use super::*;

pub struct SolidityInterface {
    pub contract: SolidityContract,
    pub environment: Environment,
}

impl SolidityInterface {
    pub fn generate(&self) -> String {
        let behaviour_declarations = &self.contract.behaviour_declarations;
        let mut functions: Vec<FunctionDeclaration> = vec![];
        for declarations in behaviour_declarations {
            for function in &declarations.members {
                match function {
                    ContractBehaviourMember::FunctionDeclaration(f) => {
                        functions.push(f.clone());
                    }
                    _ => {}
                }
            }
        }

        let functions: Vec<Option<String>> = functions
            .into_iter()
            .map(SolidityInterface::render_function)
            .collect();
        let functions: Vec<String> = functions.into_iter().filter_map(|s| s).collect();
        let functions = functions.join("\n");

        return format!(
            "interface _Interface{name} {{  \n {functions} \n }}",
            name = self.contract.declaration.identifier.token.clone(),
            functions = functions
        );
    }

    pub fn render_function(function_declaration: FunctionDeclaration) -> Option<String> {
        if function_declaration.is_public() {
            let params = function_declaration.head.parameters.clone();
            let params: Vec<String> = params
                .into_iter()
                .map(|p| {
                    let param_type =
                        SolidityIRType::map_to_solidity_type(p.type_assignment.clone()).generate();
                    let mangled_name = mangle(&p.identifier.token);
                    format!(
                        "{param_type} {mangled_name}",
                        param_type = param_type,
                        mangled_name = mangled_name
                    )
                })
                .collect();

            let params = params.join(", ");

            let mut attribute = "".to_string();
            if !function_declaration.is_mutating() {
                attribute = "view ".to_string();
            }

            let return_string = if let Some(result) = function_declaration.get_result_type() {
                let result = SolidityIRType::map_to_solidity_type(result).generate();
                format!(" returns ( {result} ret)", result = result)
            } else {
                format!("")
            };
            Option::from(format!(
                "function {name}({params}) {attribute}external{return_string};",
                name = function_declaration.head.identifier.token.clone(),
                params = params.clone(),
                attribute = attribute,
                return_string = return_string
            ))
        } else {
            None
        }
    }
}
