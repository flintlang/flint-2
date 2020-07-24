use crate::ast::{
    is_redeclaration, FunctionDeclaration, FunctionInformation, Identifier, PropertyInformation,
};
use crate::environment::*;

impl Environment {
    pub fn is_conflicting(&self, identifier: &Identifier) -> bool {
        let list = vec![
            &self.contract_declarations,
            &self.struct_declarations,
            &self.asset_declarations,
        ];
        let list: Vec<&Identifier> = list.iter().flat_map(|s| s.iter()).collect();

        for i in list {
            if is_redeclaration(i, identifier) {
                return true;
            }
        }
        false
    }

    pub fn conflicting(&self, identifier: &Identifier, list: Vec<&Vec<Identifier>>) -> bool {
        let list: Vec<&Identifier> = list.iter().flat_map(|s| s.iter()).collect();

        for i in list {
            if is_redeclaration(i, identifier) {
                return true;
            }
        }
        false
    }

    pub fn conflicting_property_declaration(&self, identifier: &Identifier, type_id: &str) -> bool {
        if let Some(type_info) = self.types.get(type_id) {
            let properties: Vec<&PropertyInformation> = type_info.properties.values().collect();

            let identifiers: Vec<Identifier> = properties
                .into_iter()
                .map(|p| p.property.get_identifier())
                .collect();
            for i in identifiers {
                if is_redeclaration(&i, identifier) {
                    return true;
                }
            }
        }
        false
    }

    pub fn conflicting_trait_signatures(&self, type_id: &str) -> bool {
        if let Some(type_info) = self.types.get(type_id) {
            let conflicting = |funcs: &[FunctionInformation]| {
                if let Some(first_signature) = funcs.get(0) {
                    let first_parameter = &first_signature.declaration.head;
                    let is_first_signature = |func: &FunctionInformation| {
                        func.get_parameter_types() == first_signature.get_parameter_types()
                            && func.declaration.head.is_equal(first_parameter.clone())
                    };
                    if funcs.iter().any(is_first_signature) {
                        return true;
                    }
                }
                false
            };
            return type_info
                .trait_functions()
                .into_iter()
                .any(|(_, v)| v.len() > 1 && conflicting(&v));
        }
        false
    }

    pub fn is_conflicting_function_declaration(
        &self,
        function_declaration: &FunctionDeclaration,
        identifier: &str,
    ) -> bool {
        if self.is_contract_declared(identifier) {
            let type_info = &self.types.get(identifier);
            let mut list = vec![&self.contract_declarations, &self.struct_declarations];
            let mut value = Vec::new();
            if type_info
                .unwrap()
                .functions
                .contains_key(&function_declaration.head.identifier.token)
            {
                for function in self
                    .types
                    .get(identifier)
                    .unwrap()
                    .functions
                    .get(&function_declaration.head.identifier.token)
                    .unwrap()
                {
                    value.push(function.declaration.head.identifier.clone());
                }
                list.push(&value);
            }
            return self.conflicting(&function_declaration.head.identifier, list);
        }
        let type_info = &self.types.get(identifier);
        if type_info.is_some()
            && type_info
                .unwrap()
                .functions
                .contains_key(&function_declaration.head.identifier.token)
        {
            for function in self
                .types
                .get(identifier)
                .unwrap()
                .functions
                .get(&function_declaration.head.identifier.token)
                .unwrap()
            {
                let declaration = &function.declaration.head.identifier;
                let parameters = &function.declaration.head.parameters;
                if is_redeclaration(&function_declaration.head.identifier, declaration)
                    && &function_declaration.head.parameters == parameters
                {
                    return true;
                }
            }
        }
        false
    }
}
