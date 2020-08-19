use crate::ast::{is_redeclaration, FunctionDeclaration, FunctionInformation, Identifier};
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

    pub fn conflicting<'a, T: IntoIterator<Item = &'a Identifier>>(
        &self,
        identifier: &Identifier,
        idents: T,
    ) -> bool {
        idents
            .into_iter()
            .any(|ident| is_redeclaration(identifier, &ident))
    }

    pub fn conflicting_property_declaration(&self, identifier: &Identifier, type_id: &str) -> bool {
        self.types
            .get(type_id)
            .map(|type_info| {
                type_info
                    .properties
                    .values()
                    .map(|p| p.property.get_identifier())
                    .any(|i| is_redeclaration(&i, identifier))
            })
            .unwrap_or(false)
    }

    pub fn conflicting_trait_signatures(&self, type_id: &str) -> bool {
        if let Some(type_info) = self.types.get(type_id) {
            let conflicting = |funcs: &[FunctionInformation]| {
                if let Some(first_signature) = funcs.get(0) {
                    let first_parameter = &first_signature.declaration.head;
                    let is_first_signature = |func: &FunctionInformation| {
                        func.get_parameter_types()
                            .eq(first_signature.get_parameter_types())
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
            let type_info = self.types.get(identifier).unwrap();
            let declarations = self
                .contract_declarations
                .iter()
                .chain(self.struct_declarations.iter())
                .chain(
                    type_info
                        .functions
                        .get(&function_declaration.head.identifier.token)
                        .into_iter()
                        .flat_map(|functions| {
                            functions
                                .iter()
                                .map(|function| &function.declaration.head.identifier)
                        }),
                );
            return self.conflicting(&function_declaration.head.identifier, declarations);
        }
        self.types
            .get(identifier)
            .and_then(|type_info| {
                type_info
                    .functions
                    .get(&function_declaration.head.identifier.token)
                    .map(|functions| {
                        functions.iter().any(|function| {
                            let declaration = &function.declaration.head.identifier;
                            let parameters = &function.declaration.head.parameters;
                            is_redeclaration(&function_declaration.head.identifier, declaration)
                                && &function_declaration.head.parameters == parameters
                        })
                    })
            })
            .unwrap_or(false)
    }
}
