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

    pub fn conflicting_property_declaration(
        &self,
        identifier: &Identifier,
        t: &TypeIdentifier,
    ) -> bool {
        let type_info = self.types.get(t);
        if type_info.is_some() {
            let properties: Vec<&PropertyInformation> =
                type_info.unwrap().properties.values().collect();

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

    pub fn conflicting_trait_signatures(&self, t: &TypeIdentifier) -> bool {
        let type_info = self.types.get(t);
        let conflicting = |f: &Vec<FunctionInformation>| {
            let first = f.get(0);
            if first.is_none() {
                return false;
            }
            let first_signature = first.unwrap().clone();
            let first_parameter = first_signature.declaration.head.clone();
            for function in f {
                if function.get_parameter_types() == first_signature.get_parameter_types()
                    && function.declaration.head.is_equal(first_parameter.clone())
                {
                    return true;
                }
            }
            false
        };
        if type_info.is_some() {
            let traits = type_info.unwrap().trait_functions().clone();
            let traits: HashMap<String, Vec<FunctionInformation>> = traits
                .into_iter()
                .filter(|(_, v)| v.len() > 1)
                .filter(|(_, v)| conflicting(v))
                .collect();
            if !traits.is_empty() {
                return true;
            }
        }

        false
    }

    pub fn is_conflicting_function_declaration(
        &self,
        function_declaration: &FunctionDeclaration,
        identifier: &TypeIdentifier,
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
