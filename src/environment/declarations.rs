use crate::ast::*;
use crate::environment::*;

impl Environment {
    pub fn add_event_declaration(&mut self, e: &EventDeclaration) {
        let identifier = e.identifier.clone();
        self.event_declarations.push(identifier);
    }

    pub fn add_contract_declaration(&mut self, c: &ContractDeclaration) {
        let identifier = c.identifier.clone();
        self.contract_declarations.push(identifier);
        self.types.insert(
            c.identifier.token.clone(),
            TypeInfo {
                ..Default::default()
            },
        );

        for conformance in &c.conformances {
            self.add_conformance(
                &c.identifier.token.clone(),
                &conformance.identifier.token.clone(),
            )
        }

        for type_state in &c.type_states {
            self.add_type_state(&c.identifier.token, type_state.clone());
        }

        let members = &c.contract_members;
        for member in members {
            match member {
                ContractMember::EventDeclaration(e) => self.add_event_declaration(&e),
                ContractMember::VariableDeclaration(v, modifier) => self.add_property(
                    Property::VariableDeclaration(v.clone(), modifier.clone()),
                    &v.identifier.token,
                    &c.identifier.token,
                ),
            }
        }
    }

    pub fn add_struct_declaration(&mut self, declaration: &StructDeclaration) {
        let identifier = declaration.identifier.clone();
        self.struct_declarations.push(identifier);

        let type_info = if declaration
            .members
            .iter()
            .any(|s| matches!(s, StructMember::SpecialDeclaration(d) if d.is_init()))
        {
            TypeInfo {
                ..Default::default()
            }
        } else {
            TypeInfo {
                initialisers: vec![SpecialInformation::default_initialiser(declaration)],
                ..Default::default()
            }
        };

        self.types
            .insert(declaration.identifier.token.clone(), type_info);

        let members = &declaration.members;
        for member in members {
            match member {
                StructMember::VariableDeclaration(v, modifier) => self.add_property(
                    Property::VariableDeclaration(v.clone(), modifier.clone()),
                    &v.identifier.token,
                    &declaration.identifier.token,
                ),
                StructMember::FunctionDeclaration(f) => {
                    self.add_function(f.clone(), &declaration.identifier.token, vec![], vec![])
                }
                StructMember::SpecialDeclaration(sd) => {
                    self.add_special(sd.clone(), &declaration.identifier.token, vec![], vec![])
                }
            }
        }
    }

    pub fn add_asset_declaration(&mut self, a: &AssetDeclaration) {
        let identifier = a.identifier.clone();
        self.asset_declarations.push(identifier);

        self.types.insert(
            a.identifier.token.clone(),
            TypeInfo {
                ..Default::default()
            },
        );

        let members = &a.members;
        for member in members {
            match member {
                AssetMember::VariableDeclaration(v) => self.add_property(
                    Property::VariableDeclaration(v.clone(), None),
                    &v.identifier.token,
                    &a.identifier.token,
                ),
                AssetMember::FunctionDeclaration(f) => {
                    self.add_function(f.clone(), &a.identifier.token, vec![], vec![])
                }
                AssetMember::SpecialDeclaration(sd) => {
                    self.add_special(sd.clone(), &a.identifier.token, vec![], vec![])
                }
            }
        }
    }

    pub fn add_trait_declaration(&mut self, t: &TraitDeclaration) {
        let identifier = t.identifier.clone();
        self.trait_declarations.push(identifier);

        let special = Environment::external_trait_init();
        self.add_init_sig(special, &t.identifier.token.clone(), vec![], vec![], true);

        if !t.modifiers.is_empty() {
            if self.types.get(&t.identifier.token).is_none() {
                self.types
                    .insert(t.identifier.token.clone(), TypeInfo::new());
            }

            if self.types.get(&t.identifier.token).is_some() {
                let type_info = self.types.get_mut(&t.identifier.token);
                let type_info = type_info.unwrap();
                type_info.modifiers = t.modifiers.clone();
            }
        }

        for member in t.members.clone() {
            match member {
                TraitMember::FunctionDeclaration(f) => {
                    self.add_function(f, &t.identifier.token, vec![], vec![])
                }
                TraitMember::SpecialDeclaration(s) => {
                    self.add_special(s, &t.identifier.token, vec![], vec![])
                }
                TraitMember::FunctionSignatureDeclaration(f) => {
                    self.add_function_signature(f, &t.identifier.token, vec![], vec![], true)
                }
                TraitMember::SpecialSignatureDeclaration(_) => unimplemented!(),
                TraitMember::ContractBehaviourDeclaration(_) => {}
                TraitMember::EventDeclaration(_) => {}
            }
        }
    }

    pub fn add_contract_behaviour_declaration(&mut self, c: &ContractBehaviourDeclaration) {
        let members = &c.members;
        for member in members {
            match member {
                ContractBehaviourMember::FunctionDeclaration(f) => self.add_function(
                    f.clone(),
                    &c.identifier.token,
                    c.caller_protections.clone(),
                    c.type_states.clone(),
                ),
                ContractBehaviourMember::SpecialDeclaration(s) => self.add_special(
                    s.clone(),
                    &c.identifier.token,
                    c.caller_protections.clone(),
                    c.type_states.clone(),
                ),
                ContractBehaviourMember::SpecialSignatureDeclaration(_) => continue,
                ContractBehaviourMember::FunctionSignatureDeclaration(_) => continue,
            }
        }
    }

    pub fn add_enum_declaration(&mut self, e: &EnumDeclaration) {
        let identifier = e.identifier.clone();
        self.trait_declarations.push(identifier);

        self.types.insert(
            e.identifier.token.clone(),
            TypeInfo {
                ..Default::default()
            },
        );
    }

    pub fn add_init_sig(
        &mut self,
        sig: SpecialSignatureDeclaration,
        enclosing: &str,
        caller_protections: Vec<CallerProtection>,
        type_states: Vec<TypeState>,
        generated: bool,
    ) {
        let special = SpecialDeclaration {
            head: sig,
            body: vec![],
            scope_context: Default::default(),
            generated,
        };
        let type_info = &self.types.get_mut(enclosing);
        if type_info.is_some() {
            self.types
                .get_mut(enclosing)
                .unwrap()
                .initialisers
                .push(SpecialInformation {
                    declaration: special,
                    caller_protections,
                    type_states,
                });
        } else {
            self.types.insert(enclosing.to_string(), TypeInfo::new());
            self.types
                .get_mut(enclosing)
                .unwrap()
                .initialisers
                .push(SpecialInformation {
                    declaration: special,
                    type_states,
                    caller_protections,
                });
        }
    }
}
